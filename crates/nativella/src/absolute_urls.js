// PHP believes it is served from http://localhost/: the absolute URLs it
// writes into scripts point there, where nothing listens. This brings the
// requests of fetch(), XHR and EventSource back to the application. See
// absolute_urls.rs.
(() => {
    // `url` on the application, or as it was. Keep in sync with
    // crates/runtime/src/absolute_urls.rs: `http://` and the host of the page.
    const toApp = (url) => {
        let parsed;
        try {
            parsed = new URL(url, document.baseURI);
        } catch {
            return url;
        }
        if (parsed.protocol !== 'http:' || parsed.host !== location.host) {
            return url;
        }

        return `${location.protocol}//${location.host}${parsed.pathname}${parsed.search}${parsed.hash}`;
    };

    const fetch = window.fetch;
    window.fetch = function (input, init) {
        if (!(input instanceof Request)) {
            return fetch.call(this, toApp(input), init);
        }

        const url = toApp(input.url);

        return fetch.call(this, url === input.url ? input : new Request(url, input), init);
    };

    const { open } = XMLHttpRequest.prototype;
    // The arguments as given: open(method, url, undefined) is synchronous.
    XMLHttpRequest.prototype.open = function (method, url, ...rest) {
        return open.call(this, method, toApp(url), ...rest);
    };

    const NativeEventSource = window.EventSource;
    window.EventSource = class EventSource extends NativeEventSource {
        constructor(url, options) {
            super(toApp(url), options);
        }
    };
})();
