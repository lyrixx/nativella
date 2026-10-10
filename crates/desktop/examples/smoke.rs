//! Drives the demo page from JavaScript to check, in WebKit itself, what the
//! `request` example of php-runtime cannot: form POST bodies, redirects,
//! cookies and `fetch()` through the `php://` scheme.
//!
//! ```text
//! cargo run -p desktop --example smoke
//! ```
//!
//! The runtime logs every request, including the `/?probe=` ones the script
//! makes to report what the page sees.

use tauri::webview::PageLoadEvent;

/// One step per page load, counted in `window.name`, which survives
/// navigations. Each step reports what the page sees as a `/?probe=` request,
/// which shows up in the runtime log.
const SCRIPT: &str = r"
(async () => {
    const step = Number(window.name.replace('smoke:', '')) || 0;
    window.name = 'smoke:' + (step + 1);
    const probe = (data) => fetch('/?probe=' + encodeURIComponent(JSON.stringify({ step, ...data })));
    const visits = () => document.querySelectorAll('dd')[4]?.textContent;

    if (step === 0) {
        const response = await fetch('/native/title', { method: 'POST', body: new URLSearchParams({ title: 'Set by fetch()' }) });
        await probe({ fetchStatus: response.status, fetchRedirected: response.redirected, fetchUrl: response.url, visits: visits() });
        const input = document.querySelector('input[name=title]');
        input.value = 'Set by a form POST';
        input.form.submit();
    } else if (step === 1) {
        await probe({ href: location.href, flash: document.querySelector('.flash')?.textContent ?? null, visits: visits() });
        if (location.pathname !== '/') {
            location.href = '/';
        }
    } else if (step === 2) {
        await probe({ href: location.href, visits: visits() });
    }
})();
";

fn main() {
    desktop::run(|builder| {
        builder.on_page_load(|webview, payload| {
            if payload.event() == PageLoadEvent::Finished {
                eprintln!("[smoke] Loaded {}", payload.url());
                if let Err(error) = webview.eval(SCRIPT) {
                    eprintln!("[smoke] Could not run the script: {error}.");
                }
            }
        })
    });
}
