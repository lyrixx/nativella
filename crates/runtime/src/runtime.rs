use std::io;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Instant;

use http::{Request, Response, StatusCode};

use crate::absolute_urls;
use crate::bridge::{self, NativeHandler};
use crate::cookies::CookieJar;
use crate::redirects::{self, Next};
use crate::sapi::Engine;
use crate::static_files;

/// The PHP application to serve: any project with a `public/index.php` front
/// controller, whatever its framework.
#[derive(Debug)]
pub struct PhpApp {
    pub(crate) document_root: PathBuf,
    pub(crate) front_controller: PathBuf,
    pub(crate) server_variables: Vec<(String, String)>,
    pub(crate) ini: Vec<(String, String)>,
    pub(crate) cookie_file: Option<PathBuf>,
}

impl PhpApp {
    pub fn from_project_dir(project_dir: &Path) -> io::Result<Self> {
        let document_root = project_dir.join("public").canonicalize()?;
        let front_controller = document_root.join("index.php");

        if !front_controller.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "The front controller {} does not exist.",
                    front_controller.display()
                ),
            ));
        }

        Ok(Self {
            document_root,
            front_controller,
            server_variables: Vec::new(),
            ini: Vec::new(),
            cookie_file: None,
        })
    }

    /// Keeps the cookies that have an expiry date in `file`, so that they
    /// outlive the application, as in a browser. Without it, cookies are
    /// lost when the application quits.
    #[must_use]
    pub fn with_cookie_file(mut self, file: impl Into<PathBuf>) -> Self {
        self.cookie_file = Some(file.into());

        self
    }

    /// Sets a php.ini setting, over the php.ini file if there is one.
    #[must_use]
    pub fn with_ini(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.ini.push((name.into(), value.into()));

        self
    }

    /// Adds a variable to `$_SERVER` for every request. It wins over the
    /// process environment, and a request header cannot override it.
    #[must_use]
    pub fn with_server_variable(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.server_variables.push((name.into(), value.into()));

        self
    }
}

/// Who sent a request.
///
/// PHP sees it in server variables, which a webview request cannot forge:
/// headers only ever become `HTTP_*` variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestOrigin {
    /// A page, form or asset loaded by a webview. PHP sees the label of its
    /// window as `$_SERVER['NATIVELLA_WINDOW']`.
    Webview { window: String },
    /// The host itself, to notify PHP of a native event. PHP sees it as
    /// `$_SERVER['NATIVELLA_INTERNAL'] === '1'`.
    Internal,
}

impl RequestOrigin {
    pub(crate) fn window(&self) -> Option<&str> {
        match self {
            Self::Webview { window } => Some(window),
            Self::Internal => None,
        }
    }
}

type Responder = Box<dyn FnOnce(Response<Vec<u8>>) + Send>;

enum Job {
    Request(Box<RequestJob>),
    /// Runs on the PHP thread, after the jobs queued before it.
    Callback(Box<dyn FnOnce() + Send>),
}

struct RequestJob {
    request: Request<Vec<u8>>,
    origin: RequestOrigin,
    respond: Responder,
}

/// Handle to the PHP thread. Cheap to clone; PHP shuts down once every handle
/// is dropped.
#[derive(Clone)]
pub struct PhpRuntime {
    app: Arc<PhpApp>,
    jobs: mpsc::Sender<Job>,
    /// The requests queued or being served.
    pending: Arc<AtomicUsize>,
}

impl PhpRuntime {
    /// Starts PHP on its own thread. `native_handler` answers the
    /// `nativella_call()` function PHP code can call.
    pub fn start(app: PhpApp, native_handler: Box<NativeHandler>) -> Self {
        bridge::install(native_handler);

        let app = Arc::new(app);
        let (jobs, queue) = mpsc::channel();

        let pending = Arc::new(AtomicUsize::new(0));

        let php_app = Arc::clone(&app);
        let php_pending = Arc::clone(&pending);
        thread::Builder::new()
            .name("php".to_owned())
            .spawn(move || run(&php_app, &queue, &php_pending))
            .expect("The OS can spawn the PHP thread.");

        Self { app, jobs, pending }
    }

    /// Whether PHP has no request to serve: a page that waited for a slow
    /// one may load again.
    pub fn is_idle(&self) -> bool {
        self.pending.load(Ordering::SeqCst) == 0
    }

    /// Serves `request` and hands the response to `respond`, from the PHP
    /// thread for PHP requests.
    pub fn dispatch(
        &self,
        request: Request<Vec<u8>>,
        origin: RequestOrigin,
        respond: impl FnOnce(Response<Vec<u8>>) + Send + 'static,
    ) {
        if let Some(response) = static_files::serve(&self.app.document_root, request.uri().path()) {
            eprintln!(
                "[static] {} {} {}",
                request.method(),
                request.uri(),
                response.status().as_u16()
            );
            respond(response);

            return;
        }

        let job = Job::Request(Box::new(RequestJob {
            request,
            origin,
            respond: Box::new(respond),
        }));

        self.pending.fetch_add(1, Ordering::SeqCst);
        if let Err(mpsc::SendError(Job::Request(job))) = self.jobs.send(job) {
            self.pending.fetch_sub(1, Ordering::SeqCst);
            let RequestJob {
                request, respond, ..
            } = *job;
            eprintln!(
                "[php] The PHP thread is gone, {} {} is not served.",
                request.method(),
                request.uri()
            );
            respond(
                Response::builder()
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .body(Vec::new())
                    .expect("A response with a status and no header is valid."),
            );
        }
    }

    /// Runs `callback` on the PHP thread once the requests queued before it
    /// are answered: to quit after the request that asked for it, or after
    /// the last native events.
    pub fn after_queued(&self, callback: impl FnOnce() + Send + 'static) {
        if let Err(mpsc::SendError(Job::Callback(callback))) =
            self.jobs.send(Job::Callback(Box::new(callback)))
        {
            // No PHP left to wait for.
            callback();
        }
    }

    /// Serves `request` and waits for the response. Never call it from the
    /// PHP thread (a `NativeHandler`): it would wait for itself.
    pub fn dispatch_blocking(
        &self,
        request: Request<Vec<u8>>,
        origin: RequestOrigin,
    ) -> Response<Vec<u8>> {
        let (sender, receiver) = mpsc::sync_channel(1);
        self.dispatch(request, origin, move |response| {
            // The receiver below is alive until a response arrives.
            let _ = sender.send(response);
        });

        receiver
            .recv()
            .expect("Every dispatched request gets a response.")
    }
}

fn run(app: &PhpApp, queue: &mpsc::Receiver<Job>, pending: &AtomicUsize) {
    let engine = Engine::start(&app.ini);
    let mut cookies = app
        .cookie_file
        .clone()
        .map_or_else(CookieJar::default, CookieJar::load);

    for job in queue {
        match job {
            Job::Request(job) => {
                let RequestJob {
                    request,
                    origin,
                    respond,
                } = *job;
                // One response that breaks the runtime must not stop the
                // thread: every later request would get no answer.
                let response = panic::catch_unwind(AssertUnwindSafe(|| {
                    serve(&engine, app, &mut cookies, request, origin)
                }))
                .unwrap_or_else(|_| {
                    eprintln!("[php] The runtime failed on a request: answering 500.");

                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Vec::new())
                        .expect("A response with a status and no header is valid.")
                });
                pending.fetch_sub(1, Ordering::SeqCst);
                respond(response);
            }
            Job::Callback(callback) => callback(),
        }
    }
}

/// Runs PHP for `request`, doing what a browser would and WebKit does not for
/// a custom scheme: keeping cookies and following redirects.
fn serve(
    engine: &Engine,
    app: &PhpApp,
    cookies: &mut CookieJar,
    mut request: Request<Vec<u8>>,
    origin: RequestOrigin,
) -> Response<Vec<u8>> {
    let navigation = redirects::is_navigation(&request);

    for _ in 0..=redirects::MAX_REDIRECTS {
        cookies.add_to(&mut request);
        let copy = redirects::copy(&request);

        let started = Instant::now();
        let response =
            bridge::with_window(origin.window(), || engine.handle(app, request, &origin));
        eprintln!(
            "[php] {} {} {} {:.1} ms",
            copy.method(),
            copy.uri(),
            response.status().as_u16(),
            started.elapsed().as_secs_f64() * 1000.0
        );

        cookies.store(copy.uri().path(), &response);
        let uri = copy.uri().clone();

        match redirects::next(copy, response, navigation) {
            Next::Respond(mut response) => {
                absolute_urls::rewrite(&uri, &mut response);

                return response;
            }
            Next::Follow(target) => request = target,
        }
    }

    eprintln!(
        "[redirect] Gave up after {} redirects.",
        redirects::MAX_REDIRECTS
    );

    Response::builder()
        .status(StatusCode::LOOP_DETECTED)
        .body(Vec::new())
        .expect("A response with a status and no header is valid.")
}
