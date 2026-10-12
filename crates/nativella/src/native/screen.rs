//! `screen.*`: the displays and the cursor.

use serde_json::{Value, json};
use tauri::Monitor;

use super::{Call, failed};

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, String>> {
    let app = call.app;

    Some(match method {
        "all" => app.available_monitors().map_err(failed).map(|monitors| {
            let primary = app.primary_monitor().ok().flatten();
            let screens: Vec<Value> = monitors
                .iter()
                .map(|monitor| display(monitor, primary.as_ref()))
                .collect();

            json!({ "screens": screens })
        }),
        "primary" => app.primary_monitor().map_err(failed).map(|monitor| {
            json!({ "screen": monitor.as_ref().map(|monitor| display(monitor, Some(monitor))) })
        }),
        "cursorPosition" => app.cursor_position().map_err(failed).map(|position| {
            json!({ "x": position.x, "y": position.y })
        }),
        _ => return None,
    })
}

/// In logical pixels, as CSS and window sizes are.
fn display(monitor: &Monitor, primary: Option<&Monitor>) -> Value {
    let scale = monitor.scale_factor();
    let size = monitor.size().to_logical::<f64>(scale);
    let position = monitor.position().to_logical::<f64>(scale);
    let work_area = monitor.work_area();
    let work_size = work_area.size.to_logical::<f64>(scale);
    let work_position = work_area.position.to_logical::<f64>(scale);

    json!({
        "name": monitor.name(),
        "primary": primary.is_some_and(|primary| primary.name() == monitor.name() && primary.position() == monitor.position()),
        "x": position.x,
        "y": position.y,
        "width": size.width,
        "height": size.height,
        "scaleFactor": scale,
        "workArea": {
            "x": work_position.x,
            "y": work_position.y,
            "width": work_size.width,
            "height": work_size.height,
        },
    })
}
