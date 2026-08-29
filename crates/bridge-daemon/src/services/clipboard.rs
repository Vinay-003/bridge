use serde_json::Value;
use tracing::info;
use std::sync::{OnceLock, Mutex};

static LAST: OnceLock<Mutex<String>> = OnceLock::new();

pub async fn handle(payload: Value) {
    let b64 = payload.get("data_b64").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let text = {
        use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
        let decoded = B64.decode(b64.as_bytes()).unwrap_or_else(|_| Vec::new());
        String::from_utf8(decoded).unwrap_or_else(|_| b64.clone())
    };
    {
        let lock = LAST.get_or_init(|| Mutex::new(String::new()));
        *lock.lock().unwrap() = text.clone();
    }
    info!("clipboard sync received len {}", text.len());
    if let Ok(mut cb) = arboard::Clipboard::new() {
        let _ = cb.set_text(text);
    }
}

pub fn get_last() -> String {
    LAST.get().map(|m| m.lock().unwrap().clone()).unwrap_or_default()
}
pub fn get_last_b64() -> String {
    use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
    B64.encode(get_last())
}
