//! PHP believes it is served from `http://localhost/` (see the server
//! variables of the runtime): the absolute URLs it builds point there, where
//! nothing listens. The runtime rewrites the ones of HTML and CSS, and the
//! window brings navigations back (`allow_navigation`), but URLs PHP writes
//! into scripts stay as they are: Symfony's web debug toolbar loads itself
//! from one.
//!
//! A script injected in every page brings the requests that `fetch()`, XHR
//! and `EventSource` send to `http://localhost/` back to the application.
//! Where the webview itself uses HTTP (Windows, Android:
//! `http://php.localhost/`), PHP builds its URLs right: there is nothing to
//! do.

/// Injected in every page, before its own scripts.
pub(crate) const SCRIPT: &str = include_str!("absolute_urls.js");
