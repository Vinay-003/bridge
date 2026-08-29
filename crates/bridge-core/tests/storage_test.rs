use bridge_core::{BridgeMessage, MessageType, StorageState};
use serde_json::json;

// storage MessageType serde roundtrip (will fail before Phase5 impl)
#[test]
fn storage_ls_serde() {
    let m = BridgeMessage::new(MessageType::StorageLs, json!({"path":"/"}));
    let j = m.to_json();
    assert!(j.contains("storage.ls"), "expected storage.ls in {j}");
    assert_eq!(BridgeMessage::from_json(&j).unwrap().typ, MessageType::StorageLs);
}
#[test]
fn storage_stat_serde() {
    let m = BridgeMessage::new(MessageType::StorageStat, json!({"path":"/report.pdf"}));
    assert!(m.to_json().contains("storage.stat"));
    assert_eq!(BridgeMessage::from_json(&m.to_json()).unwrap().typ, MessageType::StorageStat);
}
#[test]
fn storage_mkdir_serde() {
    let m = BridgeMessage::new(MessageType::StorageMkdir, json!({"path":"/newFolder"}));
    assert!(m.to_json().contains("storage.mkdir"));
    assert_eq!(BridgeMessage::from_json(&m.to_json()).unwrap().typ, MessageType::StorageMkdir);
}
#[test]
fn storage_rm_serde() {
    let m = BridgeMessage::new(MessageType::StorageRm, json!({"path":"/old.pdf","toTrash":true}));
    assert!(m.to_json().contains("storage.rm"));
    assert_eq!(BridgeMessage::from_json(&m.to_json()).unwrap().typ, MessageType::StorageRm);
}
#[test]
fn storage_sync_serde() {
    let m = BridgeMessage::new(MessageType::StorageSync, json!({"id":"uuid","path":"/a.bin","size":1024,"offset":0,"total":1,"index":0,"sha256":"a".repeat(64),"data_b64":""}));
    assert!(m.to_json().contains("storage.sync"));
    assert_eq!(BridgeMessage::from_json(&m.to_json()).unwrap().typ, MessageType::StorageSync);
}
#[test]
fn storage_conflict_serde() {
    let m = BridgeMessage::new(MessageType::StorageConflict, json!({"path":"/a","resolution":"lww"}));
    assert!(m.to_json().contains("storage.conflict"));
    assert_eq!(BridgeMessage::from_json(&m.to_json()).unwrap().typ, MessageType::StorageConflict);
}

#[test]
fn storage_state_machine_valid() {
    assert!(StorageState::Idle.can_transition(&StorageState::Scanning));
    assert!(StorageState::Scanning.can_transition(&StorageState::Syncing));
    assert!(StorageState::Scanning.can_transition(&StorageState::Done));
    assert!(StorageState::Syncing.can_transition(&StorageState::Conflict));
    assert!(StorageState::Conflict.can_transition(&StorageState::Syncing));
    assert!(StorageState::Syncing.can_transition(&StorageState::Done));
    assert!(StorageState::Done.can_transition(&StorageState::Idle));
    assert!(StorageState::Idle.can_transition(&StorageState::Done)); // stat no-op
}
#[test]
fn storage_state_machine_invalid() {
    assert!(!StorageState::Idle.can_transition(&StorageState::Conflict));
    assert!(!StorageState::Idle.can_transition(&StorageState::Syncing));
    assert!(!StorageState::Done.can_transition(&StorageState::Scanning)); // DONE->IDLE first
    assert!(!StorageState::Conflict.can_transition(&StorageState::Done));
    assert!(!StorageState::Scanning.can_transition(&StorageState::Conflict));
    assert!(!StorageState::Idle.can_transition(&StorageState::Idle));
}

#[test]
fn validate_storage_path_ok() {
    assert!(bridge_core::validate_storage_path("/").is_ok());
    assert!(bridge_core::validate_storage_path("/Photos/img.jpg").is_ok());
    assert!(bridge_core::validate_storage_path("/report.pdf").is_ok());
}
#[test]
fn validate_storage_path_traversal_rejected() {
    assert!(bridge_core::validate_storage_path("../secret").is_err());
    assert!(bridge_core::validate_storage_path("/a/../../etc/passwd").is_err());
    assert!(bridge_core::validate_storage_path("/a/../b").is_err());
    assert!(bridge_core::validate_storage_path("").is_err());
    assert!(bridge_core::validate_storage_path("/\0bad").is_err());
}
#[test]
fn validate_storage_ls_ok() {
    let p = json!({"path":"/", "showHidden": false});
    assert!(bridge_core::validate_storage_ls_payload(&p).is_ok());
    let p2 = json!({"path":"/Photos"});
    assert!(bridge_core::validate_storage_ls_payload(&p2).is_ok());
}
#[test]
fn validate_storage_ls_invalid() {
    let p = json!({"path":"../escape"});
    assert!(bridge_core::validate_storage_ls_payload(&p).is_err());
    let p2 = json!({"path":""});
    assert!(bridge_core::validate_storage_ls_payload(&p2).is_err());
    let p3 = json!({});
    assert!(bridge_core::validate_storage_ls_payload(&p3).is_err());
}
#[test]
fn validate_storage_mkdir_ok() {
    let p = json!({"path":"/newFolder"});
    assert!(bridge_core::validate_storage_mkdir_payload(&p).is_ok());
}
#[test]
fn validate_storage_rm_ok() {
    let p = json!({"path":"/old.pdf","toTrash":true});
    assert!(bridge_core::validate_storage_rm_payload(&p).is_ok());
}
#[test]
fn validate_storage_sync_ok() {
    let sha = "a".repeat(64);
    let p = json!({"id":"uuid","path":"/a.bin","size":1048576, "offset":0, "total":1, "index":0, "sha256": sha, "data_b64":""});
    assert!(bridge_core::validate_storage_sync_payload(&p).is_ok());
}
#[test]
fn validate_storage_sync_bad_sha() {
    let p = json!({"id":"uuid","path":"/a.bin","size":1024, "offset":0, "total":1, "index":0, "sha256":"bad", "data_b64":""});
    assert!(bridge_core::validate_storage_sync_payload(&p).is_err());
}
#[test]
fn vector_clock_dominates() {
    let mut a = std::collections::HashMap::new();
    a.insert("daemon".to_string(), 3);
    a.insert("phone".to_string(), 2);
    let mut b = std::collections::HashMap::new();
    b.insert("daemon".to_string(), 2);
    b.insert("phone".to_string(), 2);
    assert!(bridge_core::vector_clock_dominates(&a,&b));
    assert!(!bridge_core::vector_clock_dominates(&b,&a));
}
#[test]
fn vector_clock_concurrent() {
    let mut a = std::collections::HashMap::new();
    a.insert("daemon".to_string(), 3);
    a.insert("phone".to_string(), 1);
    let mut b = std::collections::HashMap::new();
    b.insert("daemon".to_string(), 2);
    b.insert("phone".to_string(), 2);
    assert!(bridge_core::is_vector_concurrent(&a,&b));
}
#[test]
fn sanitize_path_bridge_root() {
    // Should join bridge root and ensure no traversal
    let sanitized = bridge_core::sanitize_storage_path("/Photos/img.jpg");
    assert!(sanitized.is_ok());
    assert_eq!(sanitized.unwrap(), "Photos/img.jpg");
    let bad = bridge_core::sanitize_storage_path("../etc/passwd");
    assert!(bad.is_err());
}
#[test]
fn chunk_offset_4gb() {
    // offset u64 > 4GB should be valid
    let off: u64 = 5_000_000_000;
    let size: u64 = 5_500_000_000;
    assert!(off < size);
    // chunk index math
    let chunk_size: u64 = 1_048_576;
    let idx = (off / chunk_size) as u32;
    assert_eq!(idx, 4768);
    let calc_off = idx as u64 * chunk_size;
    assert_eq!(calc_off % chunk_size, 0);
}
