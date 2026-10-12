# The Nativella website

The landing page and the documentation of Nativella, as a static site, published on GitHub Pages:

- the landing page, built by a PHP script, naturally: it renders the templates of `templates/` into `dist/`;
- the documentation, the documents of [`../docs`](../docs), built by [MkDocs](https://www.mkdocs.org) with the [Material](https://squidfunk.github.io/mkdocs-material/) theme into `dist/docs/`.

## Build

From the root of the repository:

```bash
castor website:build
```

The task builds the landing page first, as it empties `dist/`, then the documentation. MkDocs runs in the builder, from a virtual environment of its own, `tools/mkdocs/.venv`, with the versions of [`tools/mkdocs/requirements.txt`](../tools/mkdocs/requirements.txt).

The build fails, with a message that says why, when a document links to a page, a section or a file of the repository that does not exist, or when `docs/README.md` does not list a document.

`dist/` is self-contained, and every link in it is relative: it works at the root of a domain as well as under a path, like `https://lyrixx.github.io/nativella/`. The few URLs that must be absolute (Open Graph, canonical, the sitemap of the documentation) use `NATIVELLA_SITE_URL`, `https://lyrixx.github.io/nativella/` by default. The GitHub Pages workflow sets it to the URL of the site:

```bash
NATIVELLA_SITE_URL=https://nativella.dev/ castor website:build
```

## Preview

`castor start` serves `dist/` on `https://nativella.test`, the documentation on `https://nativella.test/docs/`. Run `castor website:build` again after a change.

## The landing page

| Path | What |
|---|---|
| `bin/build` | The entry point. |
| `src/` | The build: `Site` writes the page, `CodeBlock` highlights code with [tempest/highlight](https://github.com/tempestphp/highlight). |
| `templates/` | Plain PHP templates. `$this` is a `View`: `$this->e()` escapes, `$this->url()` makes a relative URL, `$this->code()` highlights. |
| `assets/` | Copied as is to `dist/assets/`: the stylesheet, a few lines of JavaScript, the font, the icons. |
| `resources/og.html` | The source of `assets/og.png`, the Open Graph image. |

The architecture diagram, `docs/images/architecture.svg`, is shown in the documentation as an image and inlined in the landing page; it carries its own light and dark colours.

The font is [Fraunces](https://github.com/undercasetype/Fraunces) (SIL Open Font License, `assets/fonts/OFL.txt`), self-hosted: the site loads nothing from another domain, and tracks nobody.

To regenerate the Open Graph image after changing `resources/og.html`:

```bash
google-chrome --headless --hide-scrollbars --allow-file-access-from-files \
    --window-size=1200,630 --screenshot=assets/og.png "file://$PWD/resources/og.html"
```

## The documentation

| Path | What |
|---|---|
| `mkdocs.yml` | The configuration of MkDocs. |
| `mkdocs/hooks.py` | What makes the documents of GitHub a site: the navigation, the links to the repository. |
| `mkdocs/theme/` | The overrides of the theme: the colours and the type of the landing page. |

The documentation is written for GitHub first: it must read well there too. On top of Markdown, MkDocs understands:

- links between documents (`windows.md#window-options`), turned into links between pages;
- links to the rest of the repository (`../crates/runtime`), turned into links to GitHub by `mkdocs/hooks.py`;
- GitHub alerts (`> [!NOTE]`, `> [!WARNING]`…), with [markdown-callouts](https://github.com/oprypin/markdown-callouts);
- a title after the language of a code block, shown as its caption, which GitHub ignores: `` ```php title="src/Kernel.php" ``.

The navigation follows `docs/README.md`: one `## Group` per section, one `- [Label](file.md): description` per page. Headings get the anchors GitHub gives them, so that a link to a section works in both places.

The site loads nothing from another domain there either: no Google Fonts (`font: false`), and the link to the repository does not fetch its stars from the GitHub API (`mkdocs/theme/partials/source.html`). The font of the titles comes from the landing page, in `../assets/fonts/`: the documentation is always published under it.
