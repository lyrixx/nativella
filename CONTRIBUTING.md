# Contributing

Issues and pull requests go to this repository, for the PHP packages too: their own repositories are read-only splits of `packages/`.

- A bug, or an idea: open an issue first, with its template.
- A vulnerability: not in an issue, see [SECURITY.md](SECURITY.md).

[Developing Nativella](docs/development.md) sets up the Docker stack, which builds and tests everything without PHP, Composer or Rust on your machine, and lists the checks. Before a pull request, they pass:

```bash
castor qa --dry-run
```

A change of behavior comes with its tests, and with its documentation, in `docs/`.

Everyone here follows the [code of conduct](CODE_OF_CONDUCT.md).
