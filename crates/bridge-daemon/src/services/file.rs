use serde_json::{Value, json};
use std::path::PathBuf;
use directories::UserDirs;
use base64::{engine::general_purpose::STANDARD as b64, Engine as _};
use tracing::info;

pub async fn handle_chunk(payload: Value) -> Value {
    let id = payload.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
    let name = payload.get("name").and_then(|v| v.as_str()).unwrap_or("file.bin");
    let data_b64 = payload.get("data_b64").and_then(|v| v.as_str()).unwrap_or("");
    let offset = payload.get("offset").and_then(|v| v.as_u64()).unwrap_or(0);
    if let Ok(bytes) = b64.decode(data_b64) {
        let dir = UserDirs::new().map(|d| d.home_dir().join("Bridge")).unwrap_or(PathBuf::from("/tmp/Bridge"));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(name);
        // append or write at offset — simplified append for sequential chunks
        let mut opts = std::fs::OpenOptions::new();
        opts.create(true).append(offset==0).write(true);
        if offset != 0 { opts.append(false); opts.write(true); }
        if let Ok(mut f) = opts.open(&path) {
            use std::io::{Seek, Write};
            let _ = f.seek(std::io::SeekFrom::Start(offset));
            let _ = f.write_all(&bytes);
            info!("file chunk {}@{} -> {}", name, offset, path.display());
        }
    }
    json!({"id": id, "received": true, "offset": payload.get("offset")})
}
