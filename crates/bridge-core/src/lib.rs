pub mod crypto;
pub mod file;
pub mod pairing;
pub mod protocol;

pub use protocol::{
    BridgeMessage, BridgeError, ControlState, MessageType, StorageState, clamp_xy,
    is_valid_input_action, is_vector_concurrent, sanitize_storage_path,
    should_throttle, validate_control_start_payload, validate_display_info_payload,
    validate_input_event_payload, validate_storage_conflict_payload, validate_storage_ls_payload,
    validate_storage_mkdir_payload, validate_storage_path, validate_storage_rm_payload,
    validate_storage_stat_payload, validate_storage_sync_payload, vector_clock_dominates,
    vector_clock_merge, RelayState, MeshState, PluginState, AiState, LwwClipboard,
    is_valid_device_id, is_valid_plugin_id, is_valid_plugin_version, is_valid_stun_server,
    is_opaque_blob, sanitize_plugin_path, lww_clipboard_merge, can_plugin_access,
    validate_relay_announce_payload, validate_relay_relay_payload, validate_mesh_sync_payload,
    validate_mesh_conflict_payload, validate_plugin_manifest, validate_plugin_load_payload,
    validate_ai_summarize_payload, validate_ai_transcribe_payload, validate_ai_result_payload,
    should_rate_limit_ai, ALLOWED_PLUGIN_CAPS, RELAY_ANNOUNCE_URL, STUN_SERVER,
};
pub use pairing::{PairingState, SasCode};
pub use file::{FileChunk, chunk_file};
pub use crypto::{generate_keypair, derive_shared, fingerprint, sas_from_secret};
