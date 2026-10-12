//! `secrets.*`: tokens and passwords in the keychain of the system (Secret
//! Service on Linux, the Keychain on macOS, the Credential Manager on
//! Windows), encrypted by it, rather than in the database.

use serde::Deserialize;
use serde_json::{Value, json};

use super::{Call, failed};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Key {
    key: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Set {
    key: String,
    value: String,
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, String>> {
    // One service per application: its identifier.
    let service = call.app.config().identifier.clone();
    let entry = |key: &str| keyring::Entry::new(&service, key).map_err(failed);

    Some(match method {
        "get" => {
            call.payload::<Key>()
                .and_then(|payload| match entry(&payload.key)?.get_password() {
                    Ok(value) => Ok(json!({ "value": value })),
                    Err(keyring::Error::NoEntry) => Ok(json!({ "value": null })),
                    Err(error) => Err(failed(error)),
                })
        }
        "set" => call.payload::<Set>().and_then(|payload| {
            entry(&payload.key)?
                .set_password(&payload.value)
                .map_err(failed)?;
            Ok(json!({}))
        }),
        "delete" => call.payload::<Key>().and_then(|payload| {
            match entry(&payload.key)?.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(json!({})),
                Err(error) => Err(failed(error)),
            }
        }),
        _ => return None,
    })
}
