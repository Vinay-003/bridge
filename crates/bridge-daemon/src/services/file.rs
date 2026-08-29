use serde_json::{Value, json};
use std::path::PathBuf;
use directories::UserDirs;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use tracing::info;
use sha2::{Sha256, Digest};

pub async fn handle_chunk(payload: Value) -> Value {
    let id = payload.get("id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
    let name = payload.get("name").and_then(|v| v.as_str()).unwrap_or("file.bin").to_string();
    let data_b64 = payload.get("data_b64").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let offset = payload.get("offset").and_then(|v| v.as_u64()).unwrap_or(0);
    let sha256_claim = payload.get("sha256").and_then(|v| v.as_str()).unwrap_or("").to_string();

    let bytes = match B64.decode(data_b64.as_bytes()) {
        Ok(b) => b,
        Err(e) => { return json!({"id": id, "error": format!("b64 fail: {}", e)}) }
    };

    // verify sha if provided and not placeholder
    if !sha256_claim.is_empty() && sha256_claim != "demo" && sha256_claim != "sha" {
        let mut h = Sha256::new();
        h.update(&bytes);
        let got = hex::encode(h.finalize());
        if got != sha256_claim {
            return json!({"id": id, "error": format!("sha mismatch {} != {}", got, sha256_claim)})
        }
    }

    let dir = UserDirs::new().map(|d| d.home_dir().join("Bridge")).unwrap_or(PathBuf::from("/tmp/Bridge"));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(&name);
    // write at offset
    match std::fs::OpenOptions::new().create(true).write(true).open(&path) {
        Ok(mut f) => {
            use std::io::{Seek, Write};
            let _ = f.seek(std::io::SeekFrom::Start(offset));
            if let Err(e) = f.write_all(&bytes) {
                return json!({"id": id, "error": e.to_string()})
            }
            info!("file chunk {}@{} ({} bytes) -> {}", name, offset, bytes.len(), path.display());
            json!({"id": id, "received": true, "offset": offset, "path": path.display().to_string(), "size_inc": bytes.len()})
        },
        Err(e) => json!({"id": id, "error": e.to_string()})
    }
}
