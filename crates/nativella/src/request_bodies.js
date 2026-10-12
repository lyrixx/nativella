// Android's WebView drops request bodies on their way to the app: this sends
// them through Tauri's IPC first. See request_bodies.rs.
(() => {
    if (window.__nativellaRequestBodies || !window.__TAURI_INTERNALS__) {
        return;
    }
    window.__nativellaRequestBodies = true;

    const encoder = new TextEncoder();
    // Android's IPC carries JSON only.
    const base64 = (buffer) => {
        const bytes = new Uint8Array(buffer);
        let binary = '';
        for (let index = 0; index < bytes.length; index += 0x8000) {
            binary += String.fromCharCode(...bytes.subarray(index, index + 0x8000));
        }

        return btoa(binary);
    };
    const store = (key, contentType, buffer) => window.__TAURI_INTERNALS__.invoke('nativella_store_body', {
        key,
        contentType: contentType || '',
        body: base64(buffer),
    });
    // Unique across page loads: a body is never given to another request.
    const newKey = () => crypto.randomUUID();
    const absolute = (url) => new URL(url, document.baseURI).href.split('#')[0];
    const sameOrigin = (url) => {
        try {
            return new URL(url, document.baseURI).origin === location.origin;
        } catch {
            return false;
        }
    };
    const hasBody = (method) => !['GET', 'HEAD'].includes(String(method).toUpperCase());

    // fetch(): the body goes first, the request follows with its key.
    const fetch = window.fetch;
    window.fetch = async function (input, init) {
        // Built once: building it took the body of `input` or `init`.
        const request = new Request(input, init);
        if (!hasBody(request.method) || request.body === null || !sameOrigin(request.url)) {
            return fetch.call(this, request);
        }

        const key = newKey();
        await store(key, request.headers.get('Content-Type'), await request.clone().arrayBuffer());
        const headers = new Headers(request.headers);
        headers.set('X-Nativella-Body', key);

        return fetch.call(this, new Request(request, { headers }));
    };

    // XMLHttpRequest, asynchronous only: the body has to be read first.
    const { open, send, setRequestHeader } = XMLHttpRequest.prototype;
    XMLHttpRequest.prototype.open = function (method, url, async = true) {
        this.__nativella = { method, url: String(url), async, contentType: null };

        return open.apply(this, arguments);
    };
    XMLHttpRequest.prototype.setRequestHeader = function (name, value) {
        if (this.__nativella && name.toLowerCase() === 'content-type') {
            this.__nativella.contentType = value;
        }

        return setRequestHeader.apply(this, arguments);
    };
    XMLHttpRequest.prototype.send = function (body) {
        const request = this.__nativella;
        if (!request || body == null || !hasBody(request.method) || !sameOrigin(request.url)) {
            return send.apply(this, arguments);
        }
        if (!request.async) {
            console.error(`Nativella cannot send the body of a synchronous request to ${request.url}.`);

            return send.apply(this, arguments);
        }

        const encoded = new Request(absolute(request.url), {
            method: 'POST',
            body,
            headers: request.contentType ? { 'Content-Type': request.contentType } : {},
        });
        encoded.arrayBuffer().then(async (buffer) => {
            const key = newKey();
            const contentType = encoded.headers.get('Content-Type');
            await store(key, contentType, buffer);
            setRequestHeader.call(this, 'X-Nativella-Body', key);
            if (!request.contentType && contentType) {
                setRequestHeader.call(this, 'Content-Type', contentType);
            }
            send.call(this, buffer);
        });
    };

    // Forms navigate, without custom headers: the key of the body goes in
    // the URL, which the runtime removes before PHP sees it. The form is then
    // submitted again, without the event. Listening in the bubble phase lets
    // the page's own handlers prevent the submission first.
    window.addEventListener('submit', (event) => {
        const form = event.target;
        const submitter = event.submitter;
        const method = submitter?.hasAttribute('formmethod') ? submitter.formMethod : form.method;
        const action = submitter?.hasAttribute('formaction') ? submitter.formAction : form.action;
        if (event.defaultPrevented || method.toLowerCase() !== 'post' || !sameOrigin(action)) {
            return;
        }
        event.preventDefault();

        const data = new FormData(form, submitter);
        const enctype = submitter?.hasAttribute('formenctype') ? submitter.formEnctype : form.enctype;
        const encoded = enctype === 'multipart/form-data'
            ? new Request(action, { method: 'POST', body: data })
            : new Request(action, {
                method: 'POST',
                body: encoder.encode(new URLSearchParams([...data].map(([name, value]) => [name, typeof value === 'string' ? value : value.name])).toString()),
                headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
            });

        encoded.arrayBuffer().then(async (buffer) => {
            const key = newKey();
            await store(key, encoded.headers.get('Content-Type'), buffer);

            const target = new URL(action, document.baseURI);
            target.searchParams.set('nativella-body', key);

            // form.submit() ignores the submitter: carry its overrides, and
            // the keyed action.
            const overrides = { action: target.href };
            for (const name of ['method', 'enctype', 'target']) {
                if (submitter?.hasAttribute(`form${name}`)) {
                    overrides[name] = submitter.getAttribute(`form${name}`);
                }
            }
            const previous = {};
            for (const [name, value] of Object.entries(overrides)) {
                previous[name] = form.getAttribute(name);
                form.setAttribute(name, value);
            }
            HTMLFormElement.prototype.submit.call(form);
            for (const [name, value] of Object.entries(previous)) {
                value === null ? form.removeAttribute(name) : form.setAttribute(name, value);
            }
        });
    });
})();
