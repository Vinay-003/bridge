use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use base64::{engine::general_purpose::STANDARD as b64, Engine as _};

pub const CHUNK_SIZE: usize = 1024 * 1024; // 1 MB

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChunk {
    pub id: String,       // transfer uuid
    pub name: String,
    pub size: u64,
    pub offset: u64,
    pub total: u32,       // total chunks
    pub index: u32,
    pub sha256: String,   // per-chunk
    pub data_b64: String,
}

pub fn chunk_file(id: &str, name: &str, data: &[u8]) -> Vec<FileChunk> {
    let total = ((data.len() + CHUNK_SIZE - 1) / CHUNK_SIZE) as u32;
    let size = data.len() as u64;
    let mut out = Vec::new();
    for (idx, chunk) in data.chunks(CHUNK_SIZE).enumerate() {
        let mut hasher = Sha256::new();
        hasher.update(chunk);
        let sha = hex::encode(hasher.finalize());
        out.push(FileChunk {
            id: id.to_string(),
            name: name.to_string(),
            size,
            offset: (idx * CHUNK_SIZE) as u64,
            total,
            index: idx as u32,
            sha256: sha,
            data_b64: b64.encode(chunk),
        });
    }
    out
}

pub fn verify_chunk(chunk: &FileChunk) -> bool {
    if let Ok(bytes) = b64.decode(&chunk.data_b64) {
        let mut h = Sha256::new();
        h.update(&bytes);
        hex::encode(h.finalize()) == chunk.sha256
    } else {
        false
    }
}
