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
        if (url === input.url) {
            return fetch.call(this, input, init);
        }
        // Not `new Request(url, input)`: it hands over the body as a stream,
        // which WebKit does not upload.
        const moved = (body) => new Request(url, {
            body,
            cache: input.cache,
            credentials: input.credentials,
            headers: input.headers,
            integrity: input.integrity,
            keepalive: input.keepalive,
            method: input.method,
            mode: input.mode,
            redirect: input.redirect,
            referrer: input.referrer,
            referrerPolicy: input.referrerPolicy,
            signal: input.signal,
        });
        if (['GET', 'HEAD'].includes(input.method)) {
            return fetch.call(this, moved(undefined), init);
        }

        return input.clone().arrayBuffer().then((body) => fetch.call(this, moved(body), init));
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
