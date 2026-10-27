// The webview cannot download from the application: a link with a
// `download` attribute that points to it is fetched instead, with a header
// that tells Nativella to save the response. See downloads.rs.
(() => {
    // The download the click of `anchor` starts, if it is one of the
    // application. Keep the header in sync with HEADER in downloads.rs.
    const download = (anchor) => {
        if (!anchor.hasAttribute('download')) {
            return false;
        }
        let url;
        try {
            url = new URL(anchor.href);
        } catch {
            return false;
        }
        if (url.protocol !== location.protocol || url.host !== location.host) {
            return false;
        }

        fetch(url, { headers: { 'X-Nativella-Download': encodeURIComponent(anchor.getAttribute('download')) } })
            .catch((error) => console.error(`[nativella] Could not download ${url}:`, error));

        return true;
    };

    // After the handlers of the page, which may handle the click themselves.
    addEventListener('click', (event) => {
        const anchor = event.target instanceof Element ? event.target.closest('a[download]') : null;
        if (!event.defaultPrevented && anchor && download(anchor)) {
            event.preventDefault();
        }
    });

    // A link that is not in the document, as scripts make to download: its
    // click never reaches the listener above.
    const { click } = HTMLAnchorElement.prototype;
    HTMLAnchorElement.prototype.click = function () {
        if (this.isConnected || !download(this)) {
            click.call(this);
        }
    };
})();
