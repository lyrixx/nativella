"""
The hooks of the documentation (website/mkdocs.yml).

The documents of docs/ are written for GitHub first, and read well there: the
navigation is the list of docs/README.md, and links to the rest of the
repository are relative. These hooks make the same documents a site.
"""

import os
import re
from pathlib import Path

from mkdocs.exceptions import PluginError

# docs/README.md lists the pages by group, one per line:
#
#     ## Native APIs
#
#     - [Windows](windows.md): open, move, resize and close windows.
GROUP = re.compile(r'^## (.+)$')
PAGE = re.compile(r'^- \[([^\]]+)\]\(([\w-]+\.md)\)')

# A link out of docs/: [the runtime](../crates/runtime#readme)
REPOSITORY_LINK = re.compile(r'\]\((\.\./[^)#\s]+)(#[^)\s]*)?\)')


def on_config(config):
    site_url = os.environ.get('NATIVELLA_SITE_URL')
    if site_url:
        config.site_url = site_url.rstrip('/') + '/docs/'

    config.nav = navigation(Path(config.docs_dir) / 'README.md')

    return config


def on_page_markdown(markdown, page, config, files):
    """
    Links to the rest of the repository lead to GitHub: they are relative, for
    GitHub, but the site only holds docs/.
    """
    docs_dir = Path(config.docs_dir)
    root = docs_dir.parent
    document = Path(page.file.abs_src_path)

    def to_github(link):
        target = (document.parent / link[1]).resolve()
        if docs_dir in target.parents:
            return link[0]
        if root not in target.parents or not target.exists():
            raise PluginError(f'docs/{page.file.src_uri} links to {link[1]}, which is not a file or a directory of the repository.')

        kind = 'tree' if target.is_dir() else 'blob'
        return f']({config.repo_url}/{kind}/main/{target.relative_to(root).as_posix()}{link[2] or ""})'

    lines = []
    in_code = False
    for line in markdown.split('\n'):
        if line.startswith('```'):
            in_code = not in_code
        elif not in_code:
            line = REPOSITORY_LINK.sub(to_github, line)
        lines.append(line)

    return '\n'.join(lines)


def navigation(readme):
    """
    The navigation of docs/README.md: its title first, as "Overview", then one
    section per group.
    """
    nav = [{'Overview': 'README.md'}]
    pages = None
    for line in readme.read_text().splitlines():
        if group := GROUP.match(line):
            pages = []
            nav.append({group[1].strip(): pages})
        elif page := PAGE.match(line):
            if pages is None:
                raise PluginError(f'docs/README.md lists {page[2]} before its first "## Group".')
            pages.append({page[1]: page[2]})

    for section in nav[1:]:
        for title, pages in section.items():
            if not pages:
                raise PluginError(f'The "{title}" group of docs/README.md lists no page.')

    return nav
