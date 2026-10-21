// SPDX-License-Identifier: AGPL-3.0-or-later
//
// paint-type desktop host. Boots a Gossamer window, loads the bundled
// web UI, and registers one IPC command. Each
// __gossamer_invoke("dispatch", payload) call deserialises a Command,
// runs host_core::dispatch against the shared document, and returns the
// Response as JSON for the web UI to act on.

use gossamer_rs::App;
use host_core::dispatch::dispatch;
use host_core::document::Document;
use host_core::protocol::Command;
use std::io::Read;
use std::sync::{Arc, Mutex};

/// Read a UTF-8 UI file of at most `max_bytes` bytes.
///
/// Returns `InvalidString` for file I/O errors, invalid UTF-8, or excess size.
fn read_bounded_utf8_file(path: &str, max_bytes: u64) -> Result<String, gossamer_rs::Error> {
    let file = std::fs::File::open(path)
        .map_err(|e| gossamer_rs::Error::InvalidString(format!("{path}: {e}")))?;
    let limit = max_bytes.saturating_add(1);
    let mut raw = Vec::new();
    file.take(limit)
        .read_to_end(&mut raw)
        .map_err(|e| gossamer_rs::Error::InvalidString(format!("{path}: {e}")))?;
    if raw.len() as u64 > max_bytes {
        return Err(gossamer_rs::Error::InvalidString(format!(
            "{path}: file size exceeds PT_UI_FILE_MAX_BYTES ({max_bytes})"
        )));
    }
    String::from_utf8(raw)
        .map_err(|e| gossamer_rs::Error::InvalidString(format!("{path}: invalid UTF-8: {e}")))
}

/// Run the desktop window with document commands exposed through Gossamer IPC.
///
/// `PT_UI_FILE` selects an external UI file; `PT_UI_FILE_MAX_BYTES` sets its byte
/// limit (10 MiB by default). File loading and Gossamer setup errors propagate.
fn main() -> Result<(), gossamer_rs::Error> {
    let mut app = App::new("paint.type", 1024, 768)?;

    let doc: Arc<Mutex<Option<Document>>> = Arc::new(Mutex::new(None));

    let doc_for_cmd = Arc::clone(&doc);
    app.command("dispatch", move |payload| {
        let cmd: Command = match payload {
            serde_json::Value::String(ref encoded) => {
                serde_json::from_str(encoded).map_err(|e| format!("bad command: {e}"))?
            }
            other => serde_json::from_value(other).map_err(|e| format!("bad command: {e}"))?,
        };
        let mut guard = doc_for_cmd
            .lock()
            .map_err(|_| "document lock poisoned".to_string())?;
        let response = dispatch(&mut guard, cmd);
        serde_json::to_value(&response).map_err(|e| e.to_string())
    });

    app.set_csp("default-src 'self'; img-src 'self' data:; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'")?;

    const PT_UI_FILE_MAX_BYTES_DEFAULT: u64 = 10 * 1024 * 1024;
    match std::env::var("PT_UI_FILE") {
        Ok(path) => {
            let max_bytes: u64 = std::env::var("PT_UI_FILE_MAX_BYTES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(PT_UI_FILE_MAX_BYTES_DEFAULT);
            let html = read_bounded_utf8_file(&path, max_bytes)?;
            app.load_html(&html)?;
        }
        Err(_) => app.load_html(UI_HTML)?,
    }

    app.run();
    Ok(())
}

// The UI is embedded at compile time from src/ui/index.html.
const UI_HTML: &str = include_str!("../../ui/index.html");
