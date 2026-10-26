// What the page shows while PHP waits for a barcode: a frame and a button to
// cancel. The plugin draws the camera behind the webview, and makes it
// transparent. Called with the label of the button to show it, with null to
// take it away. See barcode_scanner.rs.
(cancelLabel) => {
    const id = 'nativella-barcode-scanner';
    document.getElementById(id)?.remove();
    document.getElementById(`${id}-style`)?.remove();
    if (cancelLabel === null) {
        return;
    }

    const style = document.createElement('style');
    style.id = `${id}-style`;
    // The page is hidden, not removed: it comes back as it was.
    style.textContent = `
        html, body { background: transparent !important; }
        body { visibility: hidden !important; }
        #${id} {
            position: fixed;
            inset: 0;
            z-index: 2147483647;
            overflow: hidden;
            visibility: visible;
            display: flex;
            align-items: center;
            justify-content: center;
            font: 600 16px system-ui, sans-serif;
        }
        #${id} div {
            /* Clear of the button, when the phone lies on its side. */
            width: min(70vw, max(120px, calc(100vh - 200px)), 320px);
            aspect-ratio: 1;
            box-sizing: border-box;
            border: 3px solid #fff;
            border-radius: 16px;
            box-shadow: 0 0 0 100vmax rgba(0, 0, 0, 0.55);
        }
        #${id} button {
            position: absolute;
            bottom: calc(env(safe-area-inset-bottom, 0px) + 32px);
            padding: 12px 28px;
            border: 0;
            border-radius: 999px;
            background: #fff;
            color: #000;
            font: inherit;
        }
    `;

    const frame = document.createElement('div');
    const cancel = document.createElement('button');
    cancel.type = 'button';
    cancel.textContent = cancelLabel;
    cancel.addEventListener('click', () => window.__TAURI_INTERNALS__.invoke('nativella_cancel_scan'));

    const overlay = document.createElement('div');
    overlay.id = id;
    overlay.append(frame, cancel);
    // The root, not the body: the body is hidden.
    document.documentElement.append(style, overlay);
}
