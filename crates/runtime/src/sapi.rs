//! The custom SAPI: how PHP sees a request from the webview, and how its
//! output, status and headers come back.
//!
//! Everything here runs on the PHP thread.

use std::env;
use std::ffi::{CString, c_char, c_int};
use std::mem;
use std::ptr;

use ext_php_rs::embed::{
    Embed, EmbedError, RequestInfo, Sapi, SapiHeader, SapiHeaders, SapiModule, SendHeadersResult,
    ServerContext, ServerVarRegistrar, cleanup_sapi_allocations, ext_php_rs_sapi_shutdown,
    ext_php_rs_sapi_startup,
};
use ext_php_rs::ffi::{
    ZEND_RESULT_CODE_SUCCESS, ext_php_rs_sapi_globals, php_module_shutdown, php_module_startup,
    php_request_shutdown, php_request_startup, sapi_request_info, sapi_shutdown, sapi_startup,
};
use http::header::{CONTENT_LENGTH, CONTENT_TYPE, COOKIE, HOST, ORIGIN, REFERER};
use http::{HeaderName, HeaderValue, Request, Response, StatusCode};

use crate::bridge;
use crate::runtime::{PhpApp, RequestOrigin};

unsafe extern "C" {
    /// Zend/zend.h: what PHP's own SAPIs report their errors with (`php_error`).
    fn zend_error(kind: c_int, format: *const c_char, ...);
}

/// `SG(request_info).proto_num` for HTTP/1.1, encoded the way PHP's own SAPIs do.
const HTTP_1_1: c_int = 1001;

/// The server variables that tell PHP who sent a request (`RequestOrigin`).
const ORIGIN_VARIABLES: [&str; 2] = ["NATIVELLA_INTERNAL", "NATIVELLA_WINDOW"];

/// The php.ini of Nativella, which PHP reads before the settings of the
/// application.
const PHP_INI: &str = include_str!("../php.ini");

/// The PHP engine, started once on the PHP thread.
pub(crate) struct Engine {
    module: *mut SapiModule,
    /// `module.ini_entries`, which PHP reads at startup and never frees.
    ini_entries: Option<CString>,
}

impl Engine {
    /// `ini` settings apply on top of the php.ini of Nativella, as `php -d`
    /// would.
    pub(crate) fn start(ini: &[(String, String)]) -> Self {
        let mut module = NativeSapi::build_module().expect("The SAPI names contain no NUL byte.");
        // ext-php-rs answers with a new C string for every request and PHP
        // never frees it: hand PHP the one the request context owns instead.
        module.read_cookies = Some(read_cookies);
        module.activate = Some(activate);
        // The one of ext-php-rs hands zend_error() its va_list where the
        // values go, and prints the message to the standard output: "headers
        // already sent by (output started at %s:%d)" named no file.
        module.sapi_error = Some(zend_error);

        // The static PHP looks for /usr/local/etc/php/php.ini and its conf.d/,
        // where static-php-cli built it to, and for PHPRC and PHP_INI_SCAN_DIR:
        // on the machine of a user, they belong to another PHP, with
        // extensions this one does not have. The PHP of the system needs its
        // own php.ini, which loads its extensions.
        if cfg!(nativella_static_php) {
            module.php_ini_ignore = 1;
        }

        // The php.ini of Nativella, then the settings of the application,
        // which override it
        let entries: String = ini
            .iter()
            .map(|(name, value)| format!("{name}=\"{}\"\n", value.replace('"', "")))
            .collect();
        let ini_entries = Some(
            CString::new(format!("{PHP_INI}\n{entries}"))
                .expect("INI settings contain no NUL byte."),
        );
        let module = Box::into_raw(Box::new(module));

        unsafe {
            ext_php_rs_sapi_startup();
            sapi_startup(module);
            // sapi_startup() resets them: php_module_startup() reads them,
            // after php.ini, as the embed SAPI does.
            if let Some(entries) = &ini_entries {
                (*module).ini_entries = entries.as_ptr();
            }
        }

        // PHP 8 starts in the C locale, as its functions expect, whatever the
        // environment says. The host may have taken the locale of the user
        // (GTK does): sprintf('%.1f') would write "12,5" in French.
        unsafe { libc::setlocale(libc::LC_ALL, c"C".as_ptr()) };

        let started = unsafe { php_module_startup(module, bridge::module_entry()) };
        assert!(
            started == ZEND_RESULT_CODE_SUCCESS,
            "PHP failed to start: check the errors above (php.ini, extensions)."
        );

        Self {
            module,
            ini_entries,
        }
    }

    /// Runs PHP for `request`. Its body goes to PHP and comes back, for the
    /// redirects that send it again.
    pub(crate) fn handle(
        &self,
        app: &PhpApp,
        request: &mut Request<Vec<u8>>,
        origin: &RequestOrigin,
    ) -> Response<Vec<u8>> {
        let body = mem::take(request.body_mut());
        let mut context = RequestContext::new(app, request, body, origin);
        let strings = RequestInfoStrings::new(&context.info);

        unsafe {
            let globals = &mut *ext_php_rs_sapi_globals();
            globals.server_context = (&raw mut context).cast();
            strings.apply(&mut globals.request_info);
            // PHP keeps the status of the previous request otherwise, and
            // only turns a fatal error into a 500 from a 200, as php-fpm
            // and the built-in server start every request with.
            globals.sapi_headers.http_response_code = 200;
        }

        if unsafe { php_request_startup() } == ZEND_RESULT_CODE_SUCCESS {
            match Embed::run_script(&app.front_controller) {
                // exit() unwinds the script and makes PHP report a failure:
                // front controllers built on symfony/runtime always call it.
                Ok(()) | Err(EmbedError::ExecuteScriptError) => {}
                Err(error) => log!(
                    "[php] {} did not run to completion: {error}.",
                    app.front_controller.display()
                ),
            }

            unsafe { php_request_shutdown(ptr::null_mut()) };
        } else {
            log!("[php] The request could not start.");
            context.status = 500;
        }

        unsafe {
            let globals = &mut *ext_php_rs_sapi_globals();
            globals.server_context = ptr::null_mut();
            RequestInfoStrings::clear(&mut globals.request_info);
        }
        drop(strings);

        *request.body_mut() = mem::take(&mut context.body);
        context.into_response()
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        unsafe {
            php_module_shutdown();
            sapi_shutdown();
            ext_php_rs_sapi_shutdown();
            cleanup_sapi_allocations(self.module);
            drop(Box::from_raw(self.module));
        }
        drop(self.ini_entries.take());
    }
}

struct NativeSapi;

impl Sapi for NativeSapi {
    type Context = RequestContext;

    fn name() -> &'static str {
        "nativella"
    }

    fn pretty_name() -> &'static str {
        "Nativella"
    }

    fn ub_write(context: &mut RequestContext, buf: &[u8]) -> usize {
        context.output.extend_from_slice(buf);

        buf.len()
    }

    fn log_message(message: &str, _syslog_type: i32) {
        log!("[php] {message}");
    }

    fn send_headers(context: &mut RequestContext, headers: &SapiHeaders) -> SendHeadersResult {
        // 0 when the script set none: PHP leaves the default to the SAPI.
        context.status = match headers.http_response_code() {
            0 => 200,
            code => code,
        };

        SendHeadersResult::DoSend
    }

    fn send_header(context: &mut RequestContext, header: &SapiHeader) {
        // PHP sends the status line first, as a header. The status code itself
        // already came with send_headers().
        if header
            .as_str()
            .is_some_and(|header| header.starts_with("HTTP/"))
        {
            return;
        }

        let Some((name, value)) = header.as_name_value() else {
            log!("[php] Dropped a response header that is not valid UTF-8 or has no colon.");

            return;
        };

        // Headers come before the first output: the warning comes in time,
        // even for a stream that never ends.
        if name.eq_ignore_ascii_case("content-type")
            && value.to_ascii_lowercase().starts_with("text/event-stream")
        {
            log!(
                "[php] {} streams server-sent events: the page gets them all at once, when the script ends, and never if the stream does not end. See docs/limitations.md.",
                context.info.request_uri.as_deref().unwrap_or("/")
            );
        }

        context.headers.push((name.to_owned(), value.to_owned()));
    }

    fn register_server_variables(context: &mut RequestContext, registrar: &mut ServerVarRegistrar) {
        for (name, value) in &context.server_variables {
            registrar.register(name, value);
        }
    }
}

/// One request, from the moment PHP reads it to the moment its response is
/// built.
struct RequestContext {
    info: RequestInfo,
    cookie: Option<CString>,
    body: Vec<u8>,
    body_read: usize,
    server_variables: Vec<(String, String)>,
    finished: bool,
    status: i32,
    headers: Vec<(String, String)>,
    output: Vec<u8>,
}

impl RequestContext {
    fn new(
        app: &PhpApp,
        request: &Request<Vec<u8>>,
        body: Vec<u8>,
        origin: &RequestOrigin,
    ) -> Self {
        let (method, uri, headers) = (request.method(), request.uri(), request.headers());

        let request_uri = uri
            .path_and_query()
            .map_or("/", |path| path.as_str())
            .to_owned();
        let query_string = uri.query().unwrap_or_default().to_owned();
        let content_type = header_str(headers, &CONTENT_TYPE);
        // An HTTP header value cannot contain a NUL byte.
        let cookie = header_str(headers, &COOKIE).and_then(|cookie| CString::new(cookie).ok());
        let script_filename = app.front_controller.to_string_lossy().into_owned();
        let script_name = format!(
            "/{}",
            app.front_controller
                .strip_prefix(&app.document_root)
                .expect("The front controller is in the document root.")
                .to_string_lossy()
        );

        // Like the CLI and php-fpm without clear_env, $_SERVER starts with the
        // process environment: that is where APP_ENV and friends come from.
        let mut server_variables: Vec<(String, String)> = env::vars_os()
            // Only the runtime says who sent the request: an application
            // started with one of them in its environment would take every
            // page for the runtime, or for another window.
            .filter(|(name, _)| !ORIGIN_VARIABLES.iter().any(|variable| name == variable))
            .filter_map(|(name, value)| {
                if let (Some(name), Some(value)) = (name.to_str(), value.to_str()) {
                    return Some((name.to_owned(), value.to_owned()));
                }
                log!(
                    "[php] Left the environment variable {} out of $_SERVER: it is not valid UTF-8.",
                    name.to_string_lossy()
                );

                None
            })
            .collect();

        server_variables.extend(
            [
                ("SERVER_SOFTWARE", "nativella".to_owned()),
                ("SERVER_NAME", uri.host().unwrap_or("localhost").to_owned()),
                ("SERVER_PORT", "80".to_owned()),
                ("SERVER_PROTOCOL", "HTTP/1.1".to_owned()),
                ("GATEWAY_INTERFACE", "CGI/1.1".to_owned()),
                ("REMOTE_ADDR", "127.0.0.1".to_owned()),
                (
                    "DOCUMENT_ROOT",
                    app.document_root.to_string_lossy().into_owned(),
                ),
                ("SCRIPT_FILENAME", script_filename.clone()),
                ("SCRIPT_NAME", script_name.clone()),
                ("PHP_SELF", script_name),
                ("REQUEST_METHOD", method.to_string()),
                ("REQUEST_URI", request_uri.clone()),
                ("QUERY_STRING", query_string.clone()),
            ]
            .map(|(name, value)| (name.to_owned(), value)),
        );

        if let Some(content_type) = &content_type {
            server_variables.push(("CONTENT_TYPE".to_owned(), content_type.clone()));
        }
        if !body.is_empty() {
            server_variables.push(("CONTENT_LENGTH".to_owned(), body.len().to_string()));
        }

        for (name, value) in headers {
            let Ok(value) = value.to_str() else {
                log!("[php] Dropped the request header {name}: its value is not printable ASCII.");
                continue;
            };
            if name == CONTENT_TYPE || name == CONTENT_LENGTH {
                continue;
            }

            let value = if name == ORIGIN || name == REFERER {
                as_php_origin(uri, value)
            } else {
                value.to_owned()
            };

            server_variables.push((
                format!(
                    "HTTP_{}",
                    name.as_str().to_ascii_uppercase().replace('-', "_")
                ),
                value,
            ));
        }

        // WebKit sends no Host header for a custom scheme.
        if !headers.contains_key(HOST) {
            server_variables.push((
                "HTTP_HOST".to_owned(),
                uri.host().unwrap_or("localhost").to_owned(),
            ));
        }

        server_variables.extend(app.server_variables.iter().cloned());

        match origin {
            RequestOrigin::Webview { window } => {
                server_variables.push(("NATIVELLA_WINDOW".to_owned(), window.clone()));
            }
            RequestOrigin::Internal => {
                server_variables.push(("NATIVELLA_INTERNAL".to_owned(), "1".to_owned()));
            }
        }

        let info = RequestInfo {
            request_method: Some(method.to_string()),
            query_string: Some(query_string),
            request_uri: Some(request_uri),
            path_translated: Some(script_filename),
            content_type,
            content_length: i64::try_from(body.len())
                .expect("A request body fits in memory, so in an i64."),
            // PHP resets the protocol of every request: see activate().
            ..RequestInfo::default()
        };

        Self {
            info,
            cookie,
            body,
            body_read: 0,
            server_variables,
            finished: false,
            status: 200,
            headers: Vec::new(),
            output: Vec::new(),
        }
    }

    fn into_response(self) -> Response<Vec<u8>> {
        // HTTP has no status beyond 599, which webviews refuse: Android's
        // sends the request to the network then.
        let status = u16::try_from(self.status)
            .ok()
            .filter(|code| (100..=599).contains(code))
            .and_then(|code| StatusCode::from_u16(code).ok())
            .unwrap_or_else(|| {
                log!(
                    "[php] {} is not an HTTP status code, answering 500 instead.",
                    self.status
                );

                StatusCode::INTERNAL_SERVER_ERROR
            });

        let mut response = Response::new(self.output);
        *response.status_mut() = status;

        for (name, value) in self.headers {
            match (
                HeaderName::try_from(name.as_str()),
                HeaderValue::try_from(value.as_str()),
            ) {
                (Ok(name), Ok(value)) => {
                    response.headers_mut().append(name, value);
                }
                _ => log!("[php] Dropped the invalid response header \"{name}: {value}\"."),
            }
        }

        response
    }
}

/// ext-php-rs only calls `read_post()`: the trait asks for the others, which
/// nothing calls. The SAPI fills `SG(request_info)` itself (`Engine::handle`),
/// has its own `read_cookies` callback, and no `fastcgi_finish_request()`.
impl ServerContext for RequestContext {
    fn init_request_info(&self, info: &mut RequestInfo) {
        info.clone_from(&self.info);
    }

    fn read_post(&mut self, buf: &mut [u8]) -> usize {
        let remaining = &self.body[self.body_read..];
        let length = remaining.len().min(buf.len());
        buf[..length].copy_from_slice(&remaining[..length]);
        self.body_read += length;

        length
    }

    fn read_cookies(&self) -> Option<&str> {
        self.cookie.as_ref().and_then(|cookie| cookie.to_str().ok())
    }

    fn finish_request(&mut self) -> bool {
        !std::mem::replace(&mut self.finished, true)
    }

    fn is_request_finished(&self) -> bool {
        self.finished
    }
}

/// What `SG(request_info)` gets of a request: the C strings must outlive the
/// request, PHP only borrows them.
struct RequestInfoStrings {
    request_method: Option<CString>,
    query_string: Option<CString>,
    request_uri: Option<CString>,
    path_translated: Option<CString>,
    content_type: Option<CString>,
    content_length: i64,
}

impl RequestInfoStrings {
    fn new(info: &RequestInfo) -> Self {
        let c_string =
            |value: &Option<String>| value.as_deref().and_then(|value| CString::new(value).ok());

        Self {
            request_method: c_string(&info.request_method),
            query_string: c_string(&info.query_string),
            request_uri: c_string(&info.request_uri),
            path_translated: c_string(&info.path_translated),
            content_type: c_string(&info.content_type),
            content_length: info.content_length,
        }
    }

    fn apply(&self, target: &mut sapi_request_info) {
        target.request_method = as_ptr(&self.request_method);
        target.query_string = as_ptr(&self.query_string).cast_mut();
        target.request_uri = as_ptr(&self.request_uri).cast_mut();
        target.path_translated = as_ptr(&self.path_translated).cast_mut();
        target.content_type = as_ptr(&self.content_type);
        target.content_length = self.content_length;
    }

    /// Forgets the strings of the request, and its cookies, which the
    /// request context owned.
    fn clear(target: &mut sapi_request_info) {
        target.request_method = ptr::null();
        target.query_string = ptr::null_mut();
        target.request_uri = ptr::null_mut();
        target.path_translated = ptr::null_mut();
        target.content_type = ptr::null();
        target.content_length = 0;
        target.cookie_data = ptr::null_mut();
    }
}

/// Called by `sapi_activate()`, at the start of every request, once it has
/// set `SG(request_info).proto_num` to HTTP/1.0. HTTP/1.1 makes
/// `header('Location: …')` answer a POST, a PUT or a DELETE with a 303, as
/// under PHP-FPM: the redirect is followed with a GET, not the same method.
extern "C" fn activate() -> c_int {
    unsafe { (*ext_php_rs_sapi_globals()).request_info.proto_num = HTTP_1_1 };

    ZEND_RESULT_CODE_SUCCESS
}

/// `SG(request_info).cookie_data`: PHP reads it during the request and never
/// frees it, the request context owns it.
extern "C" fn read_cookies() -> *mut c_char {
    let context = unsafe {
        (*ext_php_rs_sapi_globals())
            .server_context
            .cast::<RequestContext>()
    };

    unsafe { context.as_ref() }
        .and_then(|context| context.cookie.as_ref())
        .map_or(ptr::null_mut(), |cookie| cookie.as_ptr().cast_mut())
}

fn as_ptr(value: &Option<CString>) -> *const c_char {
    value.as_ref().map_or(ptr::null(), |value| value.as_ptr())
}

/// PHP believes it is served over plain HTTP (see SERVER_PORT above): an
/// `Origin` or `Referer` of the webview's own scheme ("php://localhost") is
/// rewritten to match, or same-origin checks (CSRF protections) would fail.
fn as_php_origin(uri: &http::Uri, value: &str) -> String {
    let (Some(scheme), Some(host)) = (uri.scheme_str(), uri.host()) else {
        return value.to_owned();
    };
    let webview_origin = format!("{scheme}://{host}");

    match value.strip_prefix(&webview_origin) {
        Some(rest) if rest.is_empty() || rest.starts_with('/') => format!("http://{host}{rest}"),
        _ => value.to_owned(),
    }
}

fn header_str(headers: &http::HeaderMap, name: &HeaderName) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_webview_origin_becomes_the_php_one() {
        let uri: http::Uri = "php://localhost/notes".parse().unwrap();

        assert_eq!(as_php_origin(&uri, "php://localhost"), "http://localhost");
        assert_eq!(
            as_php_origin(&uri, "php://localhost/notes/1?a=b"),
            "http://localhost/notes/1?a=b"
        );
        assert_eq!(
            as_php_origin(&uri, "php://localhost.evil.com"),
            "php://localhost.evil.com"
        );
        assert_eq!(
            as_php_origin(&uri, "https://example.com/"),
            "https://example.com/"
        );
    }
}
