#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![pairing_start, list_devices])
        .run(tauri::generate_context!())
        .expect("error while running bridge desktop");
}

#[tauri::command]
fn pairing_start() -> serde_json::Value {
    // In MVP returns mock; real delegates to bridge-daemon http
    serde_json::json!({
        "qr": "bridge://pair?v=1&id=mock&ecdh=mock&fp=aabbcc&port=8443",
        "fp": "aa:bb:cc:dd:ee:ff",
        "sas": "123456"
    })
}

#[tauri::command]
fn list_devices() -> Vec<serde_json::Value> {
    vec![serde_json::json!({"id":"demo","name":"Pixel 7","connected":false})]
}
