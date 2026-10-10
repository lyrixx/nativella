use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Instant;

use http::{Request, Response, StatusCode};

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
        })
    }
}

/// Who sent a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestOrigin {
    /// A page, form or asset loaded by the webview.
    Webview,
    /// The host itself, to notify PHP of a native event. PHP sees it as
    /// `$_SERVER['NATIVE_PHP_INTERNAL'] === '1'`, which a webview request
    /// cannot forge: headers only ever become `HTTP_*` variables.
    Internal,
}

type Responder = Box<dyn FnOnce(Response<Vec<u8>>) + Send>;

struct Job {
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
}

impl PhpRuntime {
    /// Starts PHP on its own thread. `native_handler` answers the
    /// `native_call()` function PHP code can call.
    pub fn start(app: PhpApp, native_handler: Box<NativeHandler>) -> Self {
        bridge::install(native_handler);

        let app = Arc::new(app);
        let (jobs, queue) = mpsc::channel();

        let php_app = Arc::clone(&app);
        thread::Builder::new()
            .name("php".to_owned())
            .spawn(move || run(&php_app, &queue))
            .expect("The OS can spawn the PHP thread.");

        Self { app, jobs }
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

        let job = Job {
            request,
            origin,
            respond: Box::new(respond),
        };

        if let Err(mpsc::SendError(job)) = self.jobs.send(job) {
            eprintln!(
                "[php] The PHP thread is gone, {} {} is not served.",
                job.request.method(),
                job.request.uri()
            );
            (job.respond)(
                Response::builder()
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .body(Vec::new())
                    .expect("A response with a status and no header is valid."),
            );
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

fn run(app: &PhpApp, queue: &mpsc::Receiver<Job>) {
    let engine = Engine::start();
    let mut cookies = CookieJar::default();

    for job in queue {
        let response = serve(&engine, app, &mut cookies, job.request, job.origin);
        (job.respond)(response);
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
        let response = engine.handle(app, request, origin);
        eprintln!(
            "[php] {} {} {} {:.1} ms",
            copy.method(),
            copy.uri(),
            response.status().as_u16(),
            started.elapsed().as_secs_f64() * 1000.0
        );

        cookies.store(copy.uri().path(), &response);

        match redirects::next(copy, response, navigation) {
            Next::Respond(response) => return response,
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
