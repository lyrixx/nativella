// Whether the page asks to stay, as `beforeunload` asks a browser: a
// listener cancels the event or sets its `returnValue`, or `onbeforeunload`
// returns something. Evaluated when its window closes: see before_unload.rs.
(() => {
    let asked = false;
    const event = new Event('beforeunload', { cancelable: true });
    Object.defineProperty(event, 'returnValue', {
        get: () => !asked,
        set: (value) => {
            asked ||= value === false || (typeof value === 'string' && value !== '');
        },
    });

    // An event handler's string answer only counts for a BeforeUnloadEvent,
    // which pages cannot make: the property is called by hand.
    const handler = window.onbeforeunload;
    window.onbeforeunload = null;
    try {
        window.dispatchEvent(event);
        if (typeof handler === 'function') {
            const answer = handler.call(window, event);
            asked ||= answer !== undefined && answer !== null;
        }
    } catch (error) {
        console.error('[nativella] A beforeunload listener failed:', error);
    } finally {
        window.onbeforeunload = handler;
    }

    return asked || event.defaultPrevented;
})()
