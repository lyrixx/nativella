//! `window.printToPdf`: the page of a window as a PDF file, without a dialog,
//! as the system would print it, `@media print` styles included.
//!
//! WebKitGTK prints to a file through the "Print to File" printer of GTK.
//! WKWebView prints with a print operation of AppKit that saves to the file
//! instead of printing, without its panels. Windows (WebView2's PrintToPdf)
//! is not done yet.

use std::path::Path;
#[cfg(target_os = "linux")]
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

#[cfg(target_os = "linux")]
use gtk::PrintSettings;
use tauri::WebviewWindow;
#[cfg(target_os = "linux")]
use webkit2gtk::{PrintOperation, PrintOperationExt};

use super::{NativeError, failed, invalid_payload, timeout};

/// A page with many images may take a while; a stuck one must not hold PHP
/// forever.
const TIMEOUT: Duration = Duration::from_secs(60);

/// Fails unless `path` is absolute: PHP's working directory is not the
/// application's.
fn absolute(path: &Path) -> Result<(), NativeError> {
    if path.is_absolute() {
        return Ok(());
    }

    Err(invalid_payload(format!(
        "The PDF goes to an absolute path, not \"{}\".",
        path.display()
    )))
}

#[cfg(target_os = "linux")]
pub(crate) fn print(window: &WebviewWindow, path: &Path) -> Result<(), NativeError> {
    absolute(path)?;
    let uri = gtk::glib::filename_to_uri(path, None)
        .map_err(|error| {
            invalid_payload(format!(
                "{} is no valid file name: {error}.",
                path.display()
            ))
        })?
        .to_string();

    let (sender, receiver) = mpsc::sync_channel::<Result<(), NativeError>>(2);
    let target: PathBuf = path.to_owned();
    window
        .with_webview(move |webview| {
            let settings = PrintSettings::new();
            // GTK names the printer of its file backend in the language of
            // the desktop: "Imprimer dans un fichier" in French.
            settings.set_printer(&gtk::glib::dgettext(Some("gtk30"), "Print to File"));
            settings.set(gtk::PRINT_SETTINGS_OUTPUT_FILE_FORMAT, Some("pdf"));
            settings.set(gtk::PRINT_SETTINGS_OUTPUT_URI, Some(&uri));

            let operation = PrintOperation::new(&webview.inner());
            operation.set_print_settings(&settings);
            // "failed" comes before "finished", which always comes: the
            // first answer wins.
            let failed_sender = sender.clone();
            operation.connect_failed(move |_, error| {
                let _ = failed_sender.try_send(Err(failed(format!(
                    "Cannot print to {}: {error}.",
                    target.display()
                ))));
            });
            operation.connect_finished(move |_| {
                let _ = sender.try_send(Ok(()));
            });
            operation.print();
        })
        .map_err(failed)?;

    receiver
        .recv_timeout(TIMEOUT)
        .map_err(|_| timeout(format!("Printing to {} took too long.", path.display())))?
}

#[cfg(target_os = "macos")]
pub(crate) fn print(window: &WebviewWindow, path: &Path) -> Result<(), NativeError> {
    absolute(path)?;
    let path_string = path
        .to_str()
        .ok_or_else(|| invalid_payload(format!("{} is no valid file name.", path.display())))?
        .to_owned();

    let (sender, receiver) = mpsc::sync_channel::<bool>(1);
    window
        .with_webview(move |webview| {
            macos::print(
                webview.inner().cast(),
                webview.ns_window().cast(),
                &path_string,
                sender,
            );
        })
        .map_err(failed)?;

    match receiver.recv_timeout(TIMEOUT) {
        Ok(true) => Ok(()),
        Ok(false) => Err(failed(format!("Cannot print to {}.", path.display()))),
        Err(_) => Err(timeout(format!(
            "Printing to {} took too long.",
            path.display()
        ))),
    }
}

/// AppKit's print operation, as WKWebView makes it. It runs modal for the
/// window, with neither its print panel nor its progress panel: the page
/// prints while the run loop goes on, as the web process draws it. Run
/// otherwise, it never ends.
#[cfg(target_os = "macos")]
mod macos {
    use std::cell::RefCell;
    use std::ffi::c_void;
    use std::sync::mpsc::SyncSender;

    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, Bool, NSObject, ProtocolObject};
    use objc2::{
        AllocAnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel,
    };
    use objc2_app_kit::{
        NSPrintInfo, NSPrintJobSavingURL, NSPrintOperation, NSPrintSaveJob, NSWindow,
    };
    use objc2_foundation::{NSString, NSURL};
    use objc2_web_kit::WKWebView;

    thread_local! {
        /// The delegates of the operations running, which nothing else
        /// keeps: AppKit does not retain them.
        static PRINTING: RefCell<Vec<Retained<Delegate>>> = const { RefCell::new(Vec::new()) };
    }

    struct Ivars {
        /// Whether the operation printed, once.
        done: RefCell<Option<SyncSender<bool>>>,
    }

    define_class!(
        // SAFETY: NSObject has no subclassing requirements, and Delegate
        // does not implement Drop.
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "NativellaPrintToPdfDelegate"]
        #[ivars = Ivars]
        struct Delegate;

        impl Delegate {
            #[unsafe(method(printOperationDidRun:success:contextInfo:))]
            fn did_run(&self, _operation: &NSPrintOperation, success: Bool, _context: *mut c_void) {
                if let Some(done) = self.ivars().done.borrow_mut().take() {
                    let _ = done.try_send(success.as_bool());
                }
                let this: *const Self = self;
                let delegate = PRINTING.with(|printing| {
                    let mut printing = printing.borrow_mut();
                    let index = printing
                        .iter()
                        .position(|delegate| std::ptr::eq(Retained::as_ptr(delegate), this))?;
                    Some(printing.swap_remove(index))
                });
                // Released once this method returns, with the autorelease
                // pool of this turn of the run loop.
                if let Some(delegate) = delegate {
                    let _ = Retained::autorelease_ptr(delegate);
                }
            }
        }
    );

    /// Starts printing the page of `webview`, in `window`, to the file
    /// `path`, as the print panel's "Save as PDF" does: with the paper and
    /// the margins of the user's print settings. `done` gets whether it
    /// printed.
    pub(super) fn print(
        webview: *mut WKWebView,
        window: *mut NSWindow,
        path: &str,
        done: SyncSender<bool>,
    ) {
        let Some(main_thread) = MainThreadMarker::new() else {
            let _ = done.try_send(false);
            return;
        };
        // SAFETY: Tauri hands the WKWebView of the window, and the window,
        // on the main thread, which it keeps alive while the closure runs.
        let (webview, window) = unsafe { (&*webview, &*window) };

        // A copy: the shared settings are the print panel's.
        // SAFETY: the dictionary of the shared settings is one of print
        // settings, which a new NSPrintInfo takes.
        let info = unsafe {
            NSPrintInfo::initWithDictionary(
                NSPrintInfo::alloc(),
                &NSPrintInfo::sharedPrintInfo().dictionary(),
            )
        };
        let url = NSURL::fileURLWithPath(&NSString::from_str(path));
        // SAFETY: AppKit's constants, and an NSURL for NSPrintJobSavingURL,
        // as AppKit documents it.
        unsafe {
            info.setJobDisposition(NSPrintSaveJob);
            info.dictionary()
                .setObject_forKey(&url, ProtocolObject::from_ref(NSPrintJobSavingURL));
        }

        // SAFETY: a print operation of the page, as WKWebView makes it.
        let operation = unsafe { webview.printOperationWithPrintInfo(&info) };
        operation.setShowsPrintPanel(false);
        operation.setShowsProgressPanel(false);
        // The view of the operation has no size of its own, and prints blank
        // pages without one.
        if let Some(view) = operation.view() {
            view.setFrame(webview.bounds());
        }

        let delegate = Delegate::alloc(main_thread).set_ivars(Ivars {
            done: RefCell::new(Some(done)),
        });
        // SAFETY: NSObject's init.
        let delegate: Retained<Delegate> = unsafe { msg_send![super(delegate), init] };
        let delegate_object: &AnyObject = &delegate;
        // SAFETY: the delegate answers the selector, with the arguments
        // AppKit gives it, and lives until it does.
        unsafe {
            operation.runOperationModalForWindow_delegate_didRunSelector_contextInfo(
                window,
                Some(delegate_object),
                Some(sel!(printOperationDidRun:success:contextInfo:)),
                std::ptr::null_mut(),
            );
        }
        PRINTING.with(|printing| printing.borrow_mut().push(delegate));
    }
}
