//! `window.printToPdf`: the page of a window as a PDF file, without a dialog,
//! as the system would print it, `@media print` styles included.
//!
//! WebKitGTK prints to a file through the "Print to File" printer of GTK.
//! macOS (WKWebView's createPDF) and Windows (WebView2's PrintToPdf) are not
//! done yet.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use gtk::PrintSettings;
use tauri::WebviewWindow;
use webkit2gtk::{PrintOperation, PrintOperationExt};

use super::failed;

/// A page with many images may take a while; a stuck one must not hold PHP
/// forever.
const TIMEOUT: Duration = Duration::from_secs(60);

pub(crate) fn print(window: &WebviewWindow, path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err(format!(
            "The PDF goes to an absolute path, not \"{}\".",
            path.display()
        ));
    }
    let uri = gtk::glib::filename_to_uri(path, None)
        .map_err(|error| format!("{} is no valid file name: {error}.", path.display()))?
        .to_string();

    let (sender, receiver) = mpsc::sync_channel::<Result<(), String>>(2);
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
                let _ = failed_sender.try_send(Err(format!(
                    "Cannot print to {}: {error}.",
                    target.display()
                )));
            });
            operation.connect_finished(move |_| {
                let _ = sender.try_send(Ok(()));
            });
            operation.print();
        })
        .map_err(failed)?;

    receiver
        .recv_timeout(TIMEOUT)
        .map_err(|_| format!("Printing to {} took too long.", path.display()))?
}
