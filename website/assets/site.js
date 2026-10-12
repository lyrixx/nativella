// A few enhancements; every page works without them. The layout adds the
// "js" class to <html>, before the first paint.
document.addEventListener('DOMContentLoaded', () => {
    // Copy buttons on code blocks.
    if (navigator.clipboard) {
        for (const figure of document.querySelectorAll('figure.code')) {
            let caption = figure.querySelector('.code-caption');
            if (!caption) {
                caption = document.createElement('figcaption');
                caption.className = 'code-caption';
                figure.prepend(caption);
            }
            const button = document.createElement('button');
            button.type = 'button';
            button.className = 'code-copy';
            button.textContent = 'Copy';
            button.addEventListener('click', async () => {
                const code = figure.querySelector('pre');
                try {
                    await navigator.clipboard.writeText(code.innerText.replace(/\n$/, ''));
                    button.textContent = 'Copied';
                } catch {
                    // The browser refused: select the code, for Ctrl+C.
                    getSelection().selectAllChildren(code);
                    button.textContent = 'Selected';
                }
                setTimeout(() => { button.textContent = 'Copy'; }, 1500);
            });
            caption.append(button);
        }
    }

    // Tabs: a role="tablist" of buttons, each controlling a panel.
    for (const list of document.querySelectorAll('[role="tablist"]')) {
        const tabs = [...list.querySelectorAll('[role="tab"]')];
        const select = (selected) => {
            for (const tab of tabs) {
                const active = tab === selected;
                tab.setAttribute('aria-selected', String(active));
                tab.tabIndex = active ? 0 : -1;
                document.getElementById(tab.getAttribute('aria-controls')).hidden = !active;
            }
        };
        list.addEventListener('click', (event) => {
            const tab = event.target.closest('[role="tab"]');
            if (tab) select(tab);
        });
        list.addEventListener('keydown', (event) => {
            const step = { ArrowRight: 1, ArrowLeft: -1 }[event.key];
            if (!step) return;
            const next = tabs[(tabs.indexOf(document.activeElement) + step + tabs.length) % tabs.length];
            select(next);
            next.focus();
        });
        select(tabs.find((tab) => tab.getAttribute('aria-selected') === 'true') ?? tabs[0]);
    }
});
