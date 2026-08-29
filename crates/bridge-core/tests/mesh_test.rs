use bridge_core::{MeshState, vector_clock_dominates, is_vector_concurrent, vector_clock_merge, LwwClipboard, lww_clipboard_merge, validate_mesh_sync_payload, validate_mesh_conflict_payload};
use serde_json::json;

#[test]
fn mesh_state_valid() {
    assert!(MeshState::Idle.can_transition(&MeshState::Syncing));
    assert!(MeshState::Syncing.can_transition(&MeshState::Conflict));
    assert!(MeshState::Conflict.can_transition(&MeshState::Syncing));
    assert!(MeshState::Syncing.can_transition(&MeshState::Idle));
    assert!(MeshState::Conflict.can_transition(&MeshState::Idle));
}
#[test]
fn mesh_state_invalid() {
    assert!(!MeshState::Idle.can_transition(&MeshState::Conflict));
    assert!(!MeshState::Idle.can_transition(&MeshState::Idle));
    assert!(!MeshState::Conflict.can_transition(&MeshState::Conflict));
}
#[test]
fn mesh_vector_dominates() {
    let mut a = std::collections::HashMap::new();
    a.insert("desktop".into(), 3);
    a.insert("phone".into(), 2);
    let mut b = std::collections::HashMap::new();
    b.insert("desktop".into(), 2);
    b.insert("phone".into(), 2);
    assert!(vector_clock_dominates(&a,&b));
    assert!(!vector_clock_dominates(&b,&a));
}
#[test]
fn mesh_vector_concurrent() {
    let mut a = std::collections::HashMap::new();
    a.insert("desktop".into(), 3);
    a.insert("phone".into(), 1);
    let mut b = std::collections::HashMap::new();
    b.insert("desktop".into(), 2);
    b.insert("phone".into(), 2);
    assert!(is_vector_concurrent(&a,&b));
}
#[test]
fn mesh_merge() {
    let mut a = std::collections::HashMap::new(); a.insert("desktop".into(), 1);
    let mut b = std::collections::HashMap::new(); b.insert("phone".into(), 2);
    let m = vector_clock_merge(&a,&b);
    assert_eq!(m.get("desktop"), Some(&1));
    assert_eq!(m.get("phone"), Some(&2));
}
#[test]
fn mesh_lww_merge() {
    let a = LwwClipboard{text:"hello".into(), mime:"text/plain".into(), ts:1000, device_id:"a".into()};
    let b = LwwClipboard{text:"world".into(), mime:"text/plain".into(), ts:2000, device_id:"b".into()};
    assert_eq!(lww_clipboard_merge(&a,&b).text, "world");
    assert_eq!(lww_clipboard_merge(&b,&a).text, "world");
    let c = LwwClipboard{text:"aaa".into(), mime:"text/plain".into(), ts:2000, device_id:"a".into()};
    let d = LwwClipboard{text:"bbb".into(), mime:"text/plain".into(), ts:2000, device_id:"b".into()};
    assert_eq!(lww_clipboard_merge(&c,&d).device_id, "b");
}
#[test]
fn validate_mesh_sync_ok() {
    let p = json!({"deviceId":"phone-xyz","vectors":{"phone-xyz":1},"entries":[{"path":"/report.pdf","vector":{"phone-xyz":1}}],"ts":chrono::Utc::now().timestamp_millis()});
    assert!(validate_mesh_sync_payload(&p).is_ok());
}
#[test]
fn validate_mesh_sync_missing_device() {
    let p = json!({"vectors":{},"entries":[]});
    assert!(validate_mesh_sync_payload(&p).is_err());
}
#[test]
fn validate_mesh_sync_bad_path() {
    let p = json!({"deviceId":"phone-xyz","vectors":{},"entries":[{"path":"../bad"}]});
    assert!(validate_mesh_sync_payload(&p).is_err());
}
#[test]
fn validate_mesh_conflict_ok() {
    let p = json!({"path":"/report.pdf","resolution":"lww","winner":"local","loserRename":"/report.pdf.mesh-conflict"});
    assert!(validate_mesh_conflict_payload(&p).is_ok());
}
#[test]
fn validate_mesh_conflict_invalid() {
    let p = json!({"path":"/a","resolution":"bad","winner":"local"});
    assert!(validate_mesh_conflict_payload(&p).is_err());
    let p2 = json!({"path":"../a","resolution":"lww","winner":"local"});
    assert!(validate_mesh_conflict_payload(&p2).is_err());
}
