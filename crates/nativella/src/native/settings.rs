//! `settings.*`: a small key-value store for preferences, kept as JSON in the
//! data directory. Application data belongs in the application's database.

use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::{Call, lock};

/// The settings file, read once and written after every change.
pub(crate) struct Store {
    path: PathBuf,
    values: Mutex<Option<Map<String, Value>>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Key {
    key: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Set {
    key: String,
    value: Value,
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, String>> {
    let store = &call.state().settings;

    Some(match method {
        // No "value" at all for a missing key, which a null value is not.
        "get" => call.payload::<Key>().and_then(|payload| {
            store.read(|values| match values.get(&payload.key) {
                Some(value) => json!({ "value": value }),
                None => json!({}),
            })
        }),
        "has" => call.payload::<Key>().and_then(|payload| {
            store.read(|values| json!({ "has": values.contains_key(&payload.key) }))
        }),
        "all" => store.read(|values| json!({ "settings": values })),
        "set" => call.payload::<Set>().and_then(|payload| {
            store.write(|values| {
                values.insert(payload.key, payload.value);
            })
        }),
        "forget" => call.payload::<Key>().and_then(|payload| {
            store.write(|values| {
                values.remove(&payload.key);
            })
        }),
        // Without reading the file first: clearing is how PHP recovers from
        // a corrupted one.
        "clear" => store.replace(Map::new()),
        _ => return None,
    })
}

impl Store {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self {
            path,
            values: Mutex::default(),
        }
    }

    fn read(&self, read: impl FnOnce(&Map<String, Value>) -> Value) -> Result<Value, String> {
        let mut values = lock(&self.values);

        Ok(read(self.load(&mut values)?))
    }

    fn write(&self, change: impl FnOnce(&mut Map<String, Value>)) -> Result<Value, String> {
        let mut guard = lock(&self.values);
        // Changed on a copy: the settings in memory stay as on disk if the
        // file cannot be written.
        let mut values = self.load(&mut guard)?.clone();
        change(&mut values);
        self.save(&values)?;
        *guard = Some(values);

        Ok(json!({}))
    }

    fn replace(&self, values: Map<String, Value>) -> Result<Value, String> {
        let mut guard = lock(&self.values);
        self.save(&values)?;
        *guard = Some(values);

        Ok(json!({}))
    }

    fn save(&self, values: &Map<String, Value>) -> Result<(), String> {
        // Written aside then renamed, so that a crash never leaves half a file.
        let content = serde_json::to_vec_pretty(values).expect("A JSON map serializes.");
        let temporary = self.path.with_extension("json.tmp");
        fs::write(&temporary, content)
            .and_then(|()| fs::rename(&temporary, &self.path))
            .map_err(|error| {
                format!(
                    "Could not save the settings to {}: {error}.",
                    self.path.display()
                )
            })
    }

    fn load<'a>(
        &self,
        values: &'a mut Option<Map<String, Value>>,
    ) -> Result<&'a mut Map<String, Value>, String> {
        if values.is_none() {
            let loaded = match fs::read(&self.path) {
                Ok(content) => serde_json::from_slice(&content).map_err(|error| {
                    format!(
                        "The settings file {} is corrupted: {error}.",
                        self.path.display()
                    )
                })?,
                Err(error) if error.kind() == io::ErrorKind::NotFound => Map::new(),
                Err(error) => {
                    return Err(format!(
                        "Could not read the settings from {}: {error}.",
                        self.path.display()
                    ));
                }
            };
            *values = Some(loaded);
        }

        Ok(values.as_mut().expect("The settings were just loaded."))
    }
}
