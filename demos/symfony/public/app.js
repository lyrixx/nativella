// The context menu of a note is a native one, opened by PHP.
document.addEventListener('contextmenu', (event) => {
    const link = event.target.closest('[data-context-menu]');
    if (!link || !document.documentElement.dataset.native) {
        return;
    }

    event.preventDefault();
    fetch(link.dataset.contextMenu, { method: 'POST' });
});

// In Nativella, PHP asks with a native dialog; in a browser, the page does.
document.addEventListener('submit', (event) => {
    const message = event.target.dataset.confirm;
    if (message && !document.documentElement.dataset.native && !window.confirm(message)) {
        event.preventDefault();
    }
});

// Notes changed from a native menu or a dropped file: see App\Note\NoteActions::notifyPages().
window.addEventListener('nativella:notes.changed', (event) => {
    const { created, deleted } = event.detail;

    if (created) {
        location.href = `/notes/${created}`;
    } else if (deleted && location.pathname.startsWith(`/notes/${deleted}`)) {
        location.href = '/';
    } else {
        location.reload();
    }
});
