//! Drives the Symfony demo (Nativella Notes) from JavaScript to check, in
//! WebKit itself, what the `request` example of nativella-runtime cannot: form
//! POST bodies and CSRF tokens, redirects, cookies and `fetch()` through the
//! `php://` scheme, native events reaching the page.
//!
//! ```text
//! NATIVELLA_DATA_DIR=target/smoke cargo run -p nativella --example smoke
//! ```
//!
//! Each step reports what the page sees as a `/?probe=` request, which shows
//! up in the runtime log; the last one is `"done":true`.

use tauri::webview::PageLoadEvent;

/// One step per page load, counted in `window.name`, which survives
/// navigations.
const SCRIPT: &str = r"
(async () => {
    // The page the runtime answers a redirect with has no <main>.
    if (!document.querySelector('main')) {
        return;
    }
    const step = Number(window.name.replace('smoke:', '')) || 0;
    window.name = 'smoke:' + (step + 1);
    const probe = (data) => fetch('/?probe=' + encodeURIComponent(JSON.stringify({ step, href: location.href, ...data })));

    if (step === 0) {
        await probe({ native: document.documentElement.dataset.native ?? null });
        location.href = '/notes/new';
    } else if (step === 1) {
        document.querySelector('#note_title').value = 'Smoke test';
        document.querySelector('#note_body').value = 'Written by **examples/smoke.rs**.';
        document.querySelector('form.editor').submit();
    } else if (step === 2) {
        const events = [];
        window.addEventListener('nativella:smoke.pong', (event) => events.push(event.detail));
        const response = await fetch(location.pathname + '/pin', { method: 'POST' });
        await probe({ title: document.querySelector('h1')?.textContent, flash: document.querySelector('.flash')?.textContent ?? null, fetchStatus: response.status, fetchRedirected: response.redirected });
        location.href = '/settings';
    } else if (step === 3) {
        await probe({ facts: [...document.querySelectorAll('.facts dd')].map((dd) => dd.textContent.trim()).slice(0, 6), done: true });
    }
})();
";

fn main() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../demos/symfony");
    let project = nativella_lib::Project::load(&dir).expect("The Symfony demo exists.");

    let result = nativella_lib::run(project, |builder| {
        builder.on_page_load(|webview, payload| {
            if payload.event() == PageLoadEvent::Finished {
                eprintln!("[smoke] Loaded {}", payload.url());
                if let Err(error) = webview.eval(SCRIPT) {
                    eprintln!("[smoke] Could not run the script: {error}.");
                }
            }
        })
    });

    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
