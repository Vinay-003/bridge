use serde_json::json;

// Simulate daemon storage validation without importing binary crate internals (pure contract tests).
// These mirror the handlers that will be added in crates/bridge-daemon/src/services/storage.rs

fn sanitize_path(p: &str) -> Result<String, String> {
    if p.contains('\0') { return Err("nul".into()); }
    if p.is_empty() { return Err("empty".into()); }
    // collapse
    let mut parts: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        if seg.is_empty() || seg=="." { continue; }
        if seg==".." { return Err("traversal".into()); }
        if seg.len() > 255 { return Err("segment too long".into()); }
        parts.push(seg);
    }
    Ok(parts.join("/"))
}

fn validate_ls(payload: &serde_json::Value) -> bool {
    let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("");
    sanitize_path(path).is_ok() || path=="/"
}

fn is_valid_sync(payload: &serde_json::Value) -> bool {
    let sha = payload.get("sha256").and_then(|v| v.as_str()).unwrap_or("");
    if sha.len()!=64 || !sha.chars().all(|c| c.is_ascii_hexdigit()) { return false; }
    let offset = payload.get("offset").and_then(|v| v.as_u64()).unwrap_or(u64::MAX);
    let size = payload.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
    if offset >= size && size !=0 { return false; }
    let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("");
    sanitize_path(path).is_ok()
}

#[test]
fn storage_ls_contract_valid() {
    assert!(validate_ls(&json!({"path":"/"})));
    assert!(validate_ls(&json!({"path":"/Photos"})));
    assert!(!validate_ls(&json!({"path":"../etc"})));
    assert!(!validate_ls(&json!({"path":""})));
}

#[test]
fn storage_sanitize_traversal() {
    assert!(sanitize_path("/Photos/img.jpg").is_ok());
    assert_eq!(sanitize_path("/Photos/img.jpg").unwrap(), "Photos/img.jpg");
    assert!(sanitize_path("../secret").is_err());
    assert!(sanitize_path("/a/../../b").is_err());
}

#[test]
fn storage_sync_sha_validation() {
    let good = json!({"path":"/a.bin","size":1024,"offset":0,"total":1,"index":0,"sha256":"a".repeat(64),"data_b64":""});
    assert!(is_valid_sync(&good));
    let bad = json!({"path":"/a.bin","size":1024,"offset":0,"total":1,"index":0,"sha256":"bad","data_b64":""});
    assert!(!is_valid_sync(&bad));
}

#[test]
fn storage_trash_vs_perm_delete() {
    // toTrash default true, must be explicit for permanent
    let trash = json!({"path":"/old.pdf","toTrash":true});
    assert_eq!(trash.get("toTrash").and_then(|v| v.as_bool()), Some(true));
    let perm = json!({"path":"/old.pdf","toTrash":false});
    assert_eq!(perm.get("toTrash").and_then(|v| v.as_bool()), Some(false));
    // missing defaults to true in handler (not tested here but contract)
}

#[test]
fn storage_4gb_offset() {
    let size: u64 = 5_000_000_000;
    let offset: u64 = 3_221_225_472; // 3072 * 1MB
    assert!(offset < size);
    let chunk_size: u64 = 1_048_576;
    let idx = (offset / chunk_size) as u64;
    assert_eq!(idx, 3072);
    assert_eq!(idx * chunk_size, offset);
}

#[test]
fn storage_message_type_contract() {
    let cases = [
        ("storage.ls", json!({"path":"/"})),
        ("storage.stat", json!({"path":"/a"})),
        ("storage.mkdir", json!({"path":"/new"})),
        ("storage.rm", json!({"path":"/old"})),
        ("storage.sync", json!({"path":"/a.bin","size":1,"offset":0,"total":1,"index":0,"sha256":"a".repeat(64),"data_b64":""})),
        ("storage.conflict", json!({"path":"/a","resolution":"lww"})),
    ];
    for (typ, payload) in cases {
        let msg = json!({"v":1,"id":"test","type":typ,"ts":0,"nonce":"a","payload":payload});
        let s = serde_json::to_string(&msg).unwrap();
        assert!(s.contains(typ), "msg missing type {typ}");
    }
}

#[test]
fn vector_clock_contract() {
    fn dominates(a: &std::collections::HashMap<String,u64>, b: &std::collections::HashMap<String,u64>) -> bool {
        let mut ge = true;
        let mut gt = false;
        for (k, bv) in b {
            let av = a.get(k).copied().unwrap_or(0);
            if av < *bv { ge = false; }
            if av > *bv { gt = true; }
        }
        for (k, av) in a {
            if !b.contains_key(k) && *av > 0 { gt = true; }
        }
        ge && gt
    }
    let mut a = std::collections::HashMap::new();
    a.insert("daemon".into(), 3);
    let mut b = std::collections::HashMap::new();
    b.insert("daemon".into(), 2);
    assert!(dominates(&a,&b));
    assert!(!dominates(&b,&a));
}

#[test]
fn notify_debounce_simulation() {
    // simulate notify debounce 250ms coalesce
    let mut last: Option<i64> = None;
    let events = vec![(0,"/a"), (100,"/a"), (200,"/a"), (600,"/a")];
    let mut coalesced = 0;
    let mut emitted = 0;
    for (ts,_path) in events {
        let should_debounce = last.map(|l| ts - l < 250).unwrap_or(false);
        if should_debounce { coalesced += 1; } else { emitted += 1; }
        last = Some(ts);
    }
    assert_eq!(emitted, 2); // 0 and 600
    assert_eq!(coalesced, 2); // 100,200
}
