//! What the pages may ask of the webview: the camera and the microphone, for
//! `getUserMedia()`, and the location, for `navigator.geolocation`, when the
//! application asks for the `camera`, `microphone` and `geolocation`
//! permissions. Nothing else is decided here.
//!
//! On a phone, allowing is the start: the webview then asks the user, with
//! the question of the system (Android's runtime permissions, iOS's alert),
//! for which the manifest and the Info.plist hold the declarations and the
//! reasons (see apk/mod.rs and ios/mod.rs).

use tauri::webview::{PermissionKind, PermissionResponse};

use crate::project::Permission;

/// The answer to a page that asks for `kind`, in an application that asks
/// for the permissions `asked`.
pub(crate) fn respond(asked: &[Permission], kind: PermissionKind) -> PermissionResponse {
    let needed = match kind {
        PermissionKind::Camera => Permission::Camera,
        PermissionKind::Geolocation => Permission::Geolocation,
        PermissionKind::Microphone => Permission::Microphone,
        _ => return PermissionResponse::Default,
    };

    if asked.contains(&needed) {
        PermissionResponse::Allow
    } else {
        PermissionResponse::Deny
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_what_the_application_asks_for_is_allowed() {
        let asked = [Permission::Microphone];

        assert_eq!(
            respond(&asked, PermissionKind::Microphone),
            PermissionResponse::Allow
        );
        assert_eq!(
            respond(&asked, PermissionKind::Camera),
            PermissionResponse::Deny
        );
        assert_eq!(
            respond(&[], PermissionKind::Microphone),
            PermissionResponse::Deny
        );
        assert_eq!(
            respond(&[Permission::Geolocation], PermissionKind::Geolocation),
            PermissionResponse::Allow
        );
    }

    #[test]
    fn the_rest_is_left_to_the_webview() {
        let asked = [Permission::Camera, Permission::Microphone];

        for kind in [
            PermissionKind::Notifications,
            PermissionKind::DisplayCapture,
        ] {
            assert_eq!(respond(&asked, kind), PermissionResponse::Default);
        }
    }
}
