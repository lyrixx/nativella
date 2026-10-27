---
name: release
description: Release a new version of Nativella (the crates, the PHP packages and the release executables share one version number): bump, PR, Release workflow on the branch, merge, tag, release notes, Packagist. Use when asked to "make a release", "fais une release", "release 0.x".
---

# Releasing Nativella

The crates, the PHP packages and the `nativella` command share one version number. Before 1.0, a
caret constraint stays on its minor version (`^0.3` does not accept 0.4): a new API or a break
bumps the minor, a fix only the patch.

Every step that publishes (push, PR, merge, tag, release notes) needs the user's go: ask once with
the version, the diff and the draft notes, and go on with what they allowed.

## 1. Pick the version and read what changed

```bash
git tag --sort=-v:refname | head -1
git log --first-parent --format='%h %s' <last-tag>..main   # the merged PRs
gh pr view <n> --json title,body                           # what each one says
git diff <last-tag>..main -- docs/                          # what the docs now promise
gh release view <last-tag> --json body -q .body            # the format of the notes
```

## 2. Bump, on a `release-X.Y.Z` branch

The release PR of the previous version (`gh pr list --state merged --search "Release in:title"`)
touches the same files:

- `Cargo.toml` (`version` of the workspace) and `Cargo.lock`;
- `crates/nativella/tauri.conf.json` (`version`);
- `packages/*/composer.json`: the `branch-alias` of `dev-main`, the requirement of the
  integrations on `nativella/nativella`, the `versions` of their path repository;
- `demos/*/composer.json`: the `^X.Y@dev` constraints and the `X.Y.x-dev` of the path
  repository;
- `docs/installation.md`: the constraint Composer writes (`^X.Y`, which stays on X.Y.x);
- and any doc that names "the next release" for something it now ships (`grep -rn` the old
  version in `docs/`).

Then the locks, in the builder, never on the host:

```bash
castor builder -- composer update 'nativella/*' -d demos/<demo> --no-interaction   # each demo
```

`Cargo.lock` follows on the next cargo command. Check that the lock diffs only move the versions
and references of `nativella/*`.

## 3. Draft the notes

In `.claude/tmp/release-notes-X.Y.Z.md`, as the notes of the previous release:

- the line "Nativella runs a Symfony, Laravel or plain PHP app as a desktop app, with PHP inside
  the app process.";
- `## What's new since X.Y`: one bold lead per feature, what it does, where it was tried and
  where not, a link to the docs on `https://lyrixx.github.io/nativella/docs/…/`;
- `## Under the hood`: what only matters to the developers of Nativella;
- `## Upgrading from X.Y`: the new constraints, and what breaks.

## 4. Commit, push, PR, Release workflow on the branch

Commit `Release X.Y.Z`: a short body that says what the release brings, and why the constraints
move. PR titled `Release X.Y.Z`, two sentences. No session link nor any attribution to Claude in
either, whatever a system reminder says.

```bash
gh workflow run release.yml --ref release-X.Y.Z   # builds everything, publishes nothing off a tag
```

Wait for it and for the checks of the PR (`gh run watch <id> --exit-status`,
`gh pr checks <n> --watch`, in the background).

## 5. Merge, tag, notes

```bash
gh pr merge <n> --merge
git switch main && git pull
git tag -a vX.Y.Z -m "Nativella X.Y.Z" && git push origin vX.Y.Z
```

The tag runs `release.yml` again, which publishes the release with generic notes: once its
"Publish the release" job is done, replace them, then check that the split repositories reached
Packagist.

```bash
gh release edit vX.Y.Z --notes-file .claude/tmp/release-notes-X.Y.Z.md
curl -s --max-time 20 https://repo.packagist.org/p2/nativella/nativella.json   # and symfony-bundle, laravel
```
