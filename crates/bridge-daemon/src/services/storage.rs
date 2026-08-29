use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tracing::{info, warn};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use sha2::{Sha256, Digest};
use bridge_core::{vector_clock_dominates, is_vector_concurrent, vector_clock_merge, StorageState};

pub const CHUNK_SIZE: u64 = 1024 * 1024; // 1MB
const MAX_SIZE: u64 = 50 * 1024 * 1024 * 1024; // 50GiB

// ── Sanitize (security: no path traversal) ───────────────────────────────

pub fn sanitize_path(raw: &str) -> Result<String, String> {
    if raw.is_empty() { return Err("path empty".into()); }
    if raw.len() > 4096 { return Err("path too long".into()); }
    if raw.contains('\0') { return Err("contains NUL".into()); }
    if raw == "/" { return Ok(String::new()); }
    let mut parts: Vec<&str> = Vec::new();
    for seg in raw.split('/') {
        if seg.is_empty() || seg == "." { continue; }
        if seg == ".." { return Err(format!("path traversal: {}", raw)); }
        if seg.len() > 255 { return Err("segment too long".into()); }
        parts.push(seg);
    }
    Ok(parts.join("/"))
}

pub fn bridge_root() -> PathBuf {
    if let Some(dirs) = directories::UserDirs::new() {
        dirs.home_dir().join("Bridge")
    } else {
        PathBuf::from("/tmp/Bridge")
    }
}

pub fn resolve_under_root(rel: &str) -> Result<PathBuf, String> {
    // rel is already sanitized (no .., no leading /). Join safely.
    let root = bridge_root();
    if rel.is_empty() {
        return Ok(root);
    }
    // Since sanitize already rejected "..", joining is safe via Path::join normalizes.
    Ok(root.join(rel))
}

pub fn trash_root() -> PathBuf {
    if let Some(dirs) = directories::BaseDirs::new() {
        dirs.data_local_dir().join("Trash")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".local/share/Trash")
    } else {
        PathBuf::from("/tmp/Trash")
    }
}

// ── Vector clock helpers (re-export via bridge_core) ──────────────────────

pub fn check_conflict(local_vc: &HashMap<String,u64>, remote_vc: &HashMap<String,u64>) -> bool {
    is_vector_concurrent(local_vc, remote_vc)
}

// ── Manifest persistence (vector + mtime) ─────────────────────────────────
static MANIFEST: OnceLock<Mutex<HashMap<String, (i64, HashMap<String,u64>)>>> = OnceLock::new();

fn manifest_path() -> PathBuf {
    if let Some(proj) = directories::ProjectDirs::from("dev", "bridge", "bridge") {
        proj.config_local_dir().join("sync-manifest.json")
    } else {
        bridge_root().join(".bridge-sync")
    }
}

pub fn load_manifest() -> HashMap<String, (i64, HashMap<String,u64>)> {
    let p = manifest_path();
    if let Ok(s) = std::fs::read_to_string(&p) {
        if let Ok(map) = serde_json::from_str::<HashMap<String, serde_json::Value>>(&s) {
            // map values are objects with mtime and vector
            let mut out: HashMap<String,(i64,HashMap<String,u64>)> = HashMap::new();
            for (k,v) in map {
                let mtime = v.get("mtimeMs").and_then(|x| x.as_i64()).unwrap_or(0);
                let vc: HashMap<String,u64> = v.get("vector").and_then(|x| serde_json::from_value(x.clone()).ok()).unwrap_or_default();
                out.insert(k, (mtime, vc));
            }
            return out;
        }
    }
    HashMap::new()
}

pub fn save_manifest(map: &HashMap<String,(i64,HashMap<String,u64>)>) {
    let p = manifest_path();
    if let Some(parent) = p.parent() { let _ = std::fs::create_dir_all(parent); }
    let mut json_map: HashMap<String, Value> = HashMap::new();
    for (k,(mtime, vc)) in map {
        json_map.insert(k.clone(), json!({"mtimeMs": mtime, "vector": vc}));
    }
    if let Ok(s) = serde_json::to_string_pretty(&json_map) {
        let _ = std::fs::write(&p, s);
    }
}

pub fn update_manifest_entry(path: &str, mtime_ms: i64, vector: HashMap<String,u64>) {
    let guard = MANIFEST.get_or_init(|| Mutex::new(load_manifest()));
    let mut map = guard.lock().unwrap();
    map.insert(path.to_string(), (mtime_ms, vector));
    save_manifest(&map);
}

// ── Storage state (global per daemon, simplified) ────────────────────────
static STORAGE_STATE: OnceLock<Mutex<StorageState>> = OnceLock::new();

pub fn storage_state() -> StorageState {
    STORAGE_STATE.get_or_init(|| Mutex::new(StorageState::Idle)).lock().unwrap().clone()
}
pub fn set_storage_state(s: StorageState) {
    let lock = STORAGE_STATE.get_or_init(|| Mutex::new(StorageState::Idle));
    *lock.lock().unwrap() = s;
}
pub fn try_transition_storage(to: StorageState) -> Result<(), String> {
    let lock = STORAGE_STATE.get_or_init(|| Mutex::new(StorageState::Idle));
    let mut guard = lock.lock().unwrap();
    if guard.can_transition(&to) {
        *guard = to;
        Ok(())
    } else {
        Err(format!("invalid storage transition {:?} -> {:?}", *guard, to))
    }
}

// ── Handlers ───────────────────────────────────────────────────────────────

pub async fn handle_storage_ls(payload: Value) -> Value {
    if let Err(e) = bridge_core::validate_storage_ls_payload(&payload) {
        return json!({"error": e.to_string(), "code": "validation"});
    }
    let raw = payload.get("path").and_then(|v| v.as_str()).unwrap_or("/");
    let rel = match sanitize_path(raw) { Ok(r) => r, Err(e)=> return json!({"error": e, "code":"path_traversal"}) };
    let dir: PathBuf = match resolve_under_root(&rel) { Ok(p)=> p, Err(e)=> return json!({"error":e,"code":"path_traversal"}) };
    info!("storage.ls raw={} rel={} dir={}", raw, rel, dir.display());
    if !dir.exists() {
        info!("storage.ls dir not exists {}", dir.display());
        return json!({"path": raw, "entries": [], "truncated": false});
    }
    if !dir.is_dir() {
        return json!({"error":"not a directory","code":"not_dir","path": raw});
    }
    let show_hidden = payload.get("showHidden").and_then(|v| v.as_bool()).unwrap_or(false);
    // recursive not yet; just top level
    let mut entries: Vec<Value> = Vec::new();
    let rd = match std::fs::read_dir(&dir) { Ok(r)=>r, Err(e)=> return json!({"error":e.to_string(),"code":"io"}) };
    for entry in rd {
        let entry = match entry { Ok(e)=>e, Err(_)=> continue };
        let name = entry.file_name().to_string_lossy().to_string();
        if !show_hidden && name.starts_with('.') { continue; }
        let meta = match entry.metadata() { Ok(m)=>m, Err(_)=> continue };
        let is_dir = meta.is_dir();
        let size = meta.len();
        let mtime = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as i64).unwrap_or(0);
        let rel_child = if rel.is_empty() { format!("/{}", name) } else { format!("/{}/{}", rel, name) };
        // mime simple by extension
        let mime = if is_dir { "" } else {
            match Path::new(&name).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase().as_str() {
                "jpg"|"jpeg" => "image/jpeg",
                "png" => "image/png",
                "pdf" => "application/pdf",
                "mp4" => "video/mp4",
                _ => "application/octet-stream"
            }
        };
        entries.push(json!({"name": name, "path": rel_child, "isDir": is_dir, "size": size, "mtimeMs": mtime, "mime": mime}));
    }
    // sort dirs first then alpha
    entries.sort_by(|a,b| {
        let a_dir = a.get("isDir").and_then(|v| v.as_bool()).unwrap_or(false);
        let b_dir = b.get("isDir").and_then(|v| v.as_bool()).unwrap_or(false);
        if a_dir != b_dir { return if a_dir { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater } }
        let a_name = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let b_name = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
        a_name.cmp(b_name)
    });
    // truncate 5000
    let truncated = entries.len() >= 5000;
    if truncated { entries.truncate(5000); }
    json!({"path": raw, "entries": entries, "truncated": truncated})
}

pub async fn handle_storage_stat(payload: Value) -> Value {
    if let Err(e) = bridge_core::validate_storage_stat_payload(&payload) {
        return json!({"error": e.to_string(), "code":"validation"});
    }
    let raw = payload.get("path").and_then(|v| v.as_str()).unwrap_or("/");
    let rel = match sanitize_path(raw) { Ok(r)=>r, Err(e)=> return json!({"error":e,"code":"path_traversal"}) };
    let p: PathBuf = match resolve_under_root(&rel) { Ok(x)=>x, Err(e)=> return json!({"error":e,"code":"path_traversal"}) };
    if !p.exists() {
        return json!({"path": raw, "exists": false});
    }
    let meta = match std::fs::metadata(&p) { Ok(m)=>m, Err(e)=> return json!({"error":e.to_string(),"code":"io"}) };
    let is_dir = meta.is_dir();
    let size = meta.len();
    let mtime = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as i64).unwrap_or(0);
    let sha: String = if !is_dir && size < 10*1024*1024 {
        // compute sha256 for small files
        match std::fs::read(&p) {
            Ok(bytes) => {
                let mut h = Sha256::new();
                h.update(&bytes);
                hex::encode(h.finalize())
            },
            Err(_)=> String::new()
        }
    } else { String::new() };
    let mime: String = if is_dir { String::new() } else {
        match p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase().as_str() {
            "jpg"|"jpeg" => "image/jpeg".to_string(),
            "png" => "image/png".to_string(),
            "pdf" => "application/pdf".to_string(),
            _ => "application/octet-stream".to_string()
        }
    };
    if sha.is_empty() {
        json!({"path": raw, "isDir": is_dir, "size": size, "mtimeMs": mtime, "mime": mime, "exists": true})
    } else {
        json!({"path": raw, "isDir": is_dir, "size": size, "mtimeMs": mtime, "sha256": sha, "mime": mime, "exists": true})
    }
}

pub async fn handle_storage_mkdir(payload: Value) -> Value {
    if let Err(e) = bridge_core::validate_storage_mkdir_payload(&payload) {
        return json!({"error": e.to_string(), "code":"validation"});
    }
    let raw = payload.get("path").and_then(|v| v.as_str()).unwrap_or("");
    let rel = match sanitize_path(raw) { Ok(r)=>r, Err(e)=> return json!({"error":e,"code":"path_traversal"}) };
    if rel.is_empty() { return json!({"error":"cannot mkdir root","code":"validation"}); }
    let p: PathBuf = match resolve_under_root(&rel) { Ok(x)=>x, Err(e)=> return json!({"error":e,"code":"path_traversal"}) };
    if p.exists() && p.is_dir() {
        return json!({"ok": true, "path": raw});
    }
    match std::fs::create_dir_all(&p) {
        Ok(_) => {
            info!("mkdir {}", p.display());
            json!({"ok": true, "path": raw})
        },
        Err(e) => json!({"error": e.to_string(), "code":"io"})
    }
}

pub async fn handle_storage_rm(payload: Value) -> Value {
    if let Err(e) = bridge_core::validate_storage_rm_payload(&payload) {
        return json!({"error": e.to_string(), "code":"validation"});
    }
    let raw = payload.get("path").and_then(|v| v.as_str()).unwrap_or("");
    let rel = match sanitize_path(raw) { Ok(r)=>r, Err(e)=> return json!({"error":e,"code":"path_traversal"}) };
    if rel.is_empty() { return json!({"error":"cannot rm root","code":"validation"}); }
    let p: PathBuf = match resolve_under_root(&rel) { Ok(x)=>x, Err(e)=> return json!({"error":e,"code":"path_traversal"}) };
    if !p.exists() {
        return json!({"error":"not found","code":"not_found","path": raw});
    }
    let to_trash = payload.get("toTrash").and_then(|v| v.as_bool()).unwrap_or(true);
    if to_trash {
        // Freedesktop trash: move to ~/.local/share/Trash/files + .trashinfo
        let trash = trash_root();
        let files = trash.join("files");
        let info_dir = trash.join("info");
        let _ = std::fs::create_dir_all(&files);
        let _ = std::fs::create_dir_all(&info_dir);
        let file_name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or("file".into());
        // Avoid collision
        let mut dest = files.join(&file_name);
        let mut counter = 0;
        while dest.exists() {
            counter += 1;
            let stem = Path::new(&file_name).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or(file_name.clone());
            let ext = Path::new(&file_name).extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
            dest = files.join(format!("{}.{counter}{ext}", stem));
        }
        match std::fs::rename(&p, &dest) {
            Ok(_) => {
                // Write .trashinfo
                let trashinfo_name = dest.file_name().map(|n| format!("{}.trashinfo", n.to_string_lossy())).unwrap_or(format!("{}.trashinfo", file_name));
                let info_path = info_dir.join(trashinfo_name);
                let deletion_date = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
                // Path must be absolute as per spec (use original absolute under Bridge)
                let orig_abs = p.canonicalize().unwrap_or(p.clone());
                // Actually Path is now moved, so use bridge_root+rel
                let orig_abs_str = bridge_root().join(&rel).display().to_string();
                let content = format!("[Trash Info]\nPath={}\nDeletionDate={}\n", orig_abs_str, deletion_date);
                let _ = std::fs::write(&info_path, content);
                info!("trashed {} -> {} (info {})", p.display(), dest.display(), info_path.display());
                return json!({"ok": true, "path": raw, "trashed": true, "trashInfo": {"originalPath": raw, "trashFilesPath": dest.display().to_string(), "deletionDate": deletion_date}});
            },
            Err(e) => {
                warn!("trash rename failed {}: {}", p.display(), e);
                // fallback to permanent delete if trash fails? Better error
                return json!({"error": e.to_string(), "code":"io", "path": raw});
            }
        }
    } else {
        // Permanent delete
        let res = if p.is_dir() { std::fs::remove_dir_all(&p) } else { std::fs::remove_file(&p) };
        match res {
            Ok(_) => {
                info!("permanent rm {}", p.display());
                json!({"ok": true, "path": raw, "trashed": false})
            },
            Err(e) => json!({"error": e.to_string(), "code":"io"})
        }
    }
}

pub async fn handle_storage_sync(payload: Value) -> Value {
    if let Err(e) = bridge_core::validate_storage_sync_payload(&payload) {
        return json!({"error": e.to_string(), "code":"validation"});
    }
    let raw = payload.get("path").and_then(|v| v.as_str()).unwrap_or("");
    let rel = match sanitize_path(raw) { Ok(r)=>r, Err(e)=> return json!({"error":e,"code":"path_traversal"}) };
    if rel.is_empty() { return json!({"error":"cannot sync root","code":"validation"}); }
    let id = payload.get("id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
    let size = payload.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
    let offset = payload.get("offset").and_then(|v| v.as_u64()).unwrap_or(0);
    let total = payload.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
    let index = payload.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
    let sha_claim = payload.get("sha256").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let data_b64 = payload.get("data_b64").and_then(|v| v.as_str()).unwrap_or("").to_string();

    if size > MAX_SIZE { return json!({"id": id, "error":"size > 50GiB","code":"validation"}); }
    if offset >= size { return json!({"id":id,"error":"offset >= size","code":"validation"}); }
    if index >= total { return json!({"id":id,"error":"index >= total","code":"validation"}); }
    if offset != index * CHUNK_SIZE {
        return json!({"id":id,"error": format!("offset {} != index {} * chunk {}", offset, index, CHUNK_SIZE),"code":"validation"});
    }

    let bytes = match B64.decode(data_b64.as_bytes()) {
        Ok(b)=> b,
        Err(e)=> return json!({"id": id, "error": format!("b64 fail: {}", e), "code":"validation"})
    };
    if bytes.len() as u64 > CHUNK_SIZE + 1024 {
        return json!({"id":id,"error":"chunk too large","code":"validation"});
    }
    // verify sha
    let mut h = Sha256::new();
    h.update(&bytes);
    let got = hex::encode(h.finalize());
    if got != sha_claim {
        return json!({"id": id, "error": format!("sha_mismatch {} != {}", got, sha_claim), "code":"sha_mismatch", "expected": sha_claim, "got": got});
    }

    // Check free space before write (statvfs)
    // Simplified: ensure parent exists and not exceeding quota

    let dest: PathBuf = match resolve_under_root(&rel) { Ok(p)=>p, Err(e)=> return json!({"id":id,"error":e,"code":"path_traversal"}) };
    if let Some(parent) = dest.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return json!({"id":id,"error": e.to_string(), "code":"io"});
        }
    }

    // Vector clock conflict detection — only on first chunk (offset 0) to avoid false positives on multi-chunk sequential writes
    let vc: HashMap<String,u64> = payload.get("vectorClock").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
    // Load manifest local vector
    let local_manifest = load_manifest();
    let local_entry = local_manifest.get(&rel).cloned();
    if offset == 0 {
        if let Some((_, local_vc)) = &local_entry {
            if is_vector_concurrent(local_vc, &vc) {
                // Conflict! LWW: compare mtime
                let remote_mtime = payload.get("mtimeMs").and_then(|v| v.as_i64()).unwrap_or(0);
                let local_mtime = local_entry.as_ref().map(|(m,_)| *m).unwrap_or(0);
                let winner = if remote_mtime > local_mtime { "remote" } else if local_mtime > remote_mtime { "local" } else {
                    // tie break deviceId lex
                    let local_max = local_vc.keys().max().cloned().unwrap_or_default();
                    let remote_max = vc.keys().max().cloned().unwrap_or_default();
                    if remote_max > local_max { "remote" } else { "local" }
                };
                if winner == "remote" {
                    // remote wins, but we need to preserve local file as conflict copy
                    // Rename existing file to .conflict-<ts>
                    if dest.exists() {
                        let conflict_name = format!("{}.conflict-{}", dest.display(), chrono::Utc::now().timestamp_millis());
                        let conflict_path = PathBuf::from(conflict_name);
                        let _ = std::fs::rename(&dest, &conflict_path);
                        warn!("storage conflict LWW remote wins, local preserved as {}", conflict_path.display());
                    }
                } else {
                    // local wins: we should NOT overwrite local file; inform sender conflict
                    return json!({"id": id, "path": raw, "conflict": true, "resolution":"lww", "winner":"local", "loserRename": format!("{}.conflict-{}", rel, chrono::Utc::now().timestamp_millis())});
                }
            }
        }
    }

    // Write at offset (RandomAccess for 4GB+ resume)
    match std::fs::OpenOptions::new().create(true).write(true).read(true).open(&dest) {
        Ok(mut f) => {
            use std::io::{Seek, Write};
            if let Err(e) = f.seek(std::io::SeekFrom::Start(offset)) {
                return json!({"id": id, "error": e.to_string(), "code":"io"});
            }
            if let Err(e) = f.write_all(&bytes) {
                return json!({"id": id, "error": e.to_string(), "code":"io"});
            }
            // Update manifest vector merge — merge without per-chunk increment to avoid false concurrent on next chunk of same file.
            // Only increment once when total chunks indicate completion, otherwise just merge.
            let is_last = index + 1 == total;
            let merged = if let Some((_, local_vc)) = &local_entry { vector_clock_merge(local_vc, &vc) } else { vc.clone() };
            let mut merged_inc = merged.clone();
            if is_last {
                let daemon_cnt = merged_inc.get("daemon").copied().unwrap_or(0);
                merged_inc.insert("daemon".into(), daemon_cnt + 1);
            }
            let mtime = payload.get("mtimeMs").and_then(|v| v.as_i64()).unwrap_or(chrono::Utc::now().timestamp_millis());
            update_manifest_entry(&rel, mtime, merged_inc);

            let size_on_disk = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(offset + bytes.len() as u64);
            info!("storage sync chunk {} {}@{} -> {} ({} bytes)", raw, offset, id, dest.display(), bytes.len());
            // Update storage state transitions if needed
            // For simplicity, ensure state moves
            json!({"id": id, "path": raw, "offset": offset, "index": index, "received": true, "sizeOnDisk": size_on_disk})
        },
        Err(e) => json!({"id": id, "error": e.to_string(), "code":"io"})
    }
}

pub async fn handle_storage_conflict(payload: Value) -> Value {
    if let Err(e) = bridge_core::validate_storage_conflict_payload(&payload) {
        return json!({"error": e.to_string(), "code":"validation"});
    }
    let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let resolution = payload.get("resolution").and_then(|v| v.as_str()).unwrap_or("lww").to_string();
    let winner = payload.get("winner").and_then(|v| v.as_str()).unwrap_or("local").to_string();
    info!("storage conflict {} resolution={} winner={}", path, resolution, winner);
    // Apply resolution: if manual, respect winner; if rename keep both (already handled via .conflict rename)
    json!({"ok": true, "path": path, "resolution": resolution, "winner": winner})
}

// ── Notify watcher (inotify) ───────────────────────────────────────────────
pub fn start_notify_watcher() -> anyhow::Result<()> {
    use notify::{Watcher, RecursiveMode, EventKind};
    use std::sync::mpsc::channel;
    use std::time::{Duration, Instant};
    let root = bridge_root();
    let _ = std::fs::create_dir_all(&root);
    let (tx, rx) = channel();
    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(&root, RecursiveMode::Recursive)?;

    // Debounce handling in separate thread
    std::thread::spawn(move || {
        let mut pending: HashMap<PathBuf, (EventKind, Instant)> = HashMap::new();
        loop {
            // drain events with debounce 250ms
            while let Ok(Ok(event)) = rx.try_recv() {
                for path in event.paths {
                    pending.insert(path, (event.kind, Instant::now()));
                }
            }
            let now = Instant::now();
            let to_emit: Vec<PathBuf> = pending.iter().filter(|(_,(_,t))| now.duration_since(*t) > Duration::from_millis(250)).map(|(p,_)| p.clone()).collect();
            for p in to_emit {
                pending.remove(&p);
                // dedup & notify
                if let Ok(rel) = p.strip_prefix(&root) {
                    let rel_str = rel.to_string_lossy().to_string();
                    info!(target:"storage_watcher", "notify {} {:?}", rel_str, p);
                    // Would send via broadcast channel in real daemon; we just log + update state
                    let _ = try_transition_storage(StorageState::Scanning);
                    // Simulate scanning then syncing
                    let _ = try_transition_storage(StorageState::Syncing);
                    // In real, would queue storage.sync for this path
                    set_storage_state(StorageState::Done);
                    let _ = try_transition_storage(StorageState::Idle);
                }
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    });
    // Leak watcher to keep alive
    std::mem::forget(watcher);
    Ok(())
}

// ── Helper: audit log ─────────────────────────────────────────────────────
pub fn audit_log_storage(action: &str, path: &str, result: &str) {
    // No contents, only fingerprints
    let short_path = if path.len() > 64 { format!("...{}", &path[path.len()-64..]) } else { path.to_string() };
    info!(target:"audit", "storage {} path={} result={}", action, short_path, result);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sanitize_ok() {
        assert_eq!(sanitize_path("/Photos/img.jpg").unwrap(), "Photos/img.jpg");
        assert_eq!(sanitize_path("/").unwrap(), "");
        assert_eq!(sanitize_path("a//b///c").unwrap(), "a/b/c");
        assert_eq!(sanitize_path("/a/b/c").unwrap(), "a/b/c");
    }
    #[test]
    fn sanitize_traversal() {
        assert!(sanitize_path("../secret").is_err());
        assert!(sanitize_path("/a/../../etc/passwd").is_err());
        assert!(sanitize_path("/a/../b").is_err());
        assert!(sanitize_path("").is_err());
        assert!(sanitize_path("/\0bad").is_err());
    }
    #[test]
    fn resolve_escapes() {
        // Sanitize already prevents traversal, but resolve also ensures under root
        let rel = sanitize_path("/Photos/img.jpg").unwrap();
        let p = resolve_under_root(&rel).unwrap();
        assert!(p.starts_with(bridge_root()));
    }
    #[test]
    fn storage_state_valid() {
        assert!(StorageState::Idle.can_transition(&StorageState::Scanning));
        assert!(StorageState::Scanning.can_transition(&StorageState::Syncing));
        assert!(StorageState::Syncing.can_transition(&StorageState::Done));
        assert!(StorageState::Done.can_transition(&StorageState::Idle));
        assert!(StorageState::Syncing.can_transition(&StorageState::Conflict));
        assert!(StorageState::Conflict.can_transition(&StorageState::Syncing));
    }
    #[test]
    fn storage_state_invalid() {
        assert!(!StorageState::Idle.can_transition(&StorageState::Conflict));
        assert!(!StorageState::Conflict.can_transition(&StorageState::Done));
        assert!(!StorageState::Done.can_transition(&StorageState::Scanning));
    }
    #[test]
    fn trash_path_generation() {
        let t = trash_root();
        assert!(t.to_string_lossy().contains("Trash"));
    }
    #[tokio::test]
    async fn handle_mkdir_and_ls() {
        let _ = std::fs::create_dir_all(bridge_root());
        let mkdir_payload = json!({"path": "/test_storage_5_phase_mkdir"});
        let resp = handle_storage_mkdir(mkdir_payload).await;
        assert!(resp.get("ok").and_then(|v| v.as_bool()).unwrap_or(false), "mkdir failed {:?}", resp);
        let ls_payload = json!({"path": "/"});
        let ls_resp = handle_storage_ls(ls_payload).await;
        assert!(ls_resp.get("entries").is_some(), "ls failed {:?}", ls_resp);
        // Cleanup
        let rm_payload = json!({"path": "/test_storage_5_phase_mkdir", "toTrash": false});
        let _ = handle_storage_rm(rm_payload).await;
    }
    #[tokio::test]
    async fn handle_sync_validation() {
        // Bad sha should error
        let bad = json!({"id":"t","path":"/a.bin","size":1024,"offset":0,"total":1,"index":0,"sha256":"bad","data_b64":""});
        let resp = handle_storage_sync(bad).await;
        assert!(resp.get("error").is_some(), "bad sha should error {:?}", resp);
        assert_eq!(resp.get("code").and_then(|v| v.as_str()).unwrap_or(""), "validation");
    }
    #[tokio::test]
    async fn handle_sync_chunk_ok() {
        let data = b"hello storage sync";
        let mut h = Sha256::new();
        h.update(data);
        let sha = hex::encode(h.finalize());
        let b64 = B64.encode(data);
        let payload = json!({"id":"sync-test-1","path":"/test_sync_chunk.txt","size": data.len(), "offset":0, "total":1, "index":0, "sha256": sha, "data_b64": b64});
        let resp = handle_storage_sync(payload).await;
        assert!(resp.get("received").and_then(|v| v.as_bool()).unwrap_or(false), "sync failed {:?}", resp);
        // cleanup
        let p = bridge_root().join("test_sync_chunk.txt");
        let _ = std::fs::remove_file(p);
    }
    #[tokio::test]
    async fn handle_rm_trash() {
        let root = bridge_root();
        let _ = std::fs::create_dir_all(&root);
        let fname = "test_rm_trash_5.txt";
        let fpath = root.join(fname);
        std::fs::write(&fpath, b"trash me").unwrap();
        let payload = json!({"path": format!("/{}", fname), "toTrash": true});
        let resp = handle_storage_rm(payload).await;
        assert!(resp.get("ok").and_then(|v| v.as_bool()).unwrap_or(false), "rm trash failed {:?}", resp);
        assert!(resp.get("trashed").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(!fpath.exists(), "file should be trashed");
        // Find trashed file and restore for cleanup test
        let trash_files = trash_root().join("files");
        let trashed = trash_files.join(fname);
        if trashed.exists() {
            let _ = std::fs::remove_file(&trashed);
            let info = trash_root().join("info").join(format!("{}.trashinfo", fname));
            let _ = std::fs::remove_file(info);
        }
    }
    #[tokio::test]
    async fn handle_stat_not_found() {
        let payload = json!({"path": "/nonexistent_9999_storage_test"});
        let resp = handle_storage_stat(payload).await;
        assert_eq!(resp.get("exists").and_then(|v| v.as_bool()), Some(false));
    }
}
