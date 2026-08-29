use serde_json::Value;
use tracing::info;
use std::sync::{OnceLock, Mutex};

static LAST: OnceLock<Mutex<String>> = OnceLock::new();

pub async fn handle(payload: Value) {
    let text = payload.get("data_b64").and_then(|v| v.as_str()).unwrap_or("")
        .to_string();
    // store in memory; real impl would use arboard
    let lock = LAST.get_or_init(|| Mutex::new(String::new()));
    *lock.lock().unwrap() = text.clone();
    info!("clipboard sync received len {}", text.len());
    // try set system clipboard via arboard if available (optional)
    #[cfg(target_os = "linux")]
    {
        // best-effort, ignore errors (Wayland may need extra perms)
        // let mut cb = arboard::Clipboard::new().ok();
    }
}

pub fn get_last() -> String {
    LAST.get().map(|m| m.lock().unwrap().clone()).unwrap_or_default()
}
