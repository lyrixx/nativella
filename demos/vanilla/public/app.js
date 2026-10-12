// Snippets: enhancements on top of pages that work without JavaScript. No
// build step, no dependency.

const toast = document.getElementById('toast');
let toastTimer;

function showToast(message) {
    toast.textContent = message;
    toast.hidden = false;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => { toast.hidden = true; }, 3000);
}

// PHP writes dates in UTC: show them in the time zone and language of the user.
const dateFormat = new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' });

function localizeDates(root) {
    for (const time of root.querySelectorAll('time[datetime]')) {
        time.textContent = dateFormat.format(new Date(time.dateTime));
    }
}

localizeDates(document);

// Replaces the list with the one PHP renders for `url`.
async function refreshList(url) {
    const list = document.getElementById('snippets');
    if (list === null) {
        return;
    }

    const response = await fetch(url);
    const page = new DOMParser().parseFromString(await response.text(), 'text/html');
    const fresh = page.getElementById('snippets');
    if (fresh !== null) {
        localizeDates(fresh);
        list.replaceWith(fresh);
    }
}

// Searches as the user types.
const search = document.getElementById('search');
let searchTimer;
search?.addEventListener('input', () => {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
        const url = search.value.trim() === '' ? '/' : '/?' + new URLSearchParams({ q: search.value.trim() });
        history.replaceState(null, '', url);
        refreshList(url);
    }, 150);
});

document.addEventListener('submit', async (event) => {
    const form = event.target;

    // Outside Nativella, PHP has no native dialog to ask with.
    if (form.dataset.confirm !== undefined && !confirm(form.dataset.confirm)) {
        event.preventDefault();

        return;
    }

    // Copies without leaving the page: PHP answers JSON when asked to.
    if (form.dataset.copy !== undefined) {
        event.preventDefault();
        const response = await fetch(form.action, { method: 'POST', headers: { Accept: 'application/json' } });
        showToast(response.ok ? (await response.json()).message : 'This snippet does not exist anymore.');
    }
});

// Ctrl+Enter (⌘+Enter on macOS) saves the form.
document.addEventListener('keydown', (event) => {
    if (event.key === 'Enter' && (event.ctrlKey || event.metaKey) && event.target.form) {
        event.preventDefault();
        event.target.form.requestSubmit();
    }
});

// Native events reach the page as DOM events, prefixed with "nativella:".

// PHP imported dropped files, or the file picked from the File menu.
// Keep the name in sync with NativeEventHandler::IMPORTED_EVENT.
window.addEventListener('nativella:snippets.imported', async (event) => {
    await refreshList(location.pathname + location.search);
    showToast(event.detail.message);
});

// PHP focuses the window on the global shortcut; the page focuses the search.
window.addEventListener('nativella:globalShortcut.pressed', () => {
    search?.focus();
    search?.select();
});
