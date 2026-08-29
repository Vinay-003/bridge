use serde::{Deserialize, Serialize};
use uuid::Uuid;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("unknown message type: {0}")]
    UnknownType(String),
    #[error("validation: {0}")]
    Validation(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    // pairing
    #[serde(rename = "pairing.hello")] PairingHello,
    #[serde(rename = "pairing.sas")] PairingSas,
    #[serde(rename = "pairing.trusted")] PairingTrusted,
    // heartbeat
    #[serde(rename = "heartbeat.ping")] Ping,
    #[serde(rename = "heartbeat.pong")] Pong,
    // file
    #[serde(rename = "file.chunk")] FileChunk,
    #[serde(rename = "file.ack")] FileAck,
    #[serde(rename = "file.resume")] FileResume,
    // clipboard
    #[serde(rename = "clipboard.sync")] ClipboardSync,
    // notify
    #[serde(rename = "notify.new")] NotifyNew,
    #[serde(rename = "notify.action")] NotifyAction,
    // status
    #[serde(rename = "status.push")] StatusPush,
    // webrtc signalling
    #[serde(rename = "webrtc.offer")] WebrtcOffer,
    #[serde(rename = "webrtc.answer")] WebrtcAnswer,
    #[serde(rename = "webrtc.ice")] WebrtcIce,
    // control — Phase 4 Remote Control
    #[serde(rename = "input.event")] InputEvent,
    #[serde(rename = "input.ack")] InputAck,
    #[serde(rename = "display.info")] DisplayInfo,
    #[serde(rename = "display.frame")] DisplayFrame,
    #[serde(rename = "control.start")] ControlStart,
    #[serde(rename = "control.stop")] ControlStop,
    // storage — Phase 5 Storage Deep
    #[serde(rename = "storage.ls")] StorageLs,
    #[serde(rename = "storage.stat")] StorageStat,
    #[serde(rename = "storage.mkdir")] StorageMkdir,
    #[serde(rename = "storage.rm")] StorageRm,
    #[serde(rename = "storage.sync")] StorageSync,
    #[serde(rename = "storage.conflict")] StorageConflict,
    // error
    #[serde(rename = "error")] Error,
}

// Control state machine: DISABLED -> ENABLED -> CONTROLLING -> PAUSED
#[derive(Debug, Clone, PartialEq)]
pub enum ControlState {
    Disabled,
    Enabled,
    Controlling,
    Paused,
}

impl ControlState {
    pub fn can_transition(&self, next: &ControlState) -> bool {
        matches!(
            (self, next),
            (ControlState::Disabled, ControlState::Enabled)
                | (ControlState::Enabled, ControlState::Controlling)
                | (ControlState::Controlling, ControlState::Paused)
                | (ControlState::Paused, ControlState::Enabled)
                | (ControlState::Controlling, ControlState::Enabled)
                | (ControlState::Paused, ControlState::Disabled)
                | (ControlState::Enabled, ControlState::Disabled)
                | (ControlState::Controlling, ControlState::Disabled)
        )
    }
}

pub fn is_valid_input_action(s: &str) -> bool {
    matches!(
        s,
        "tap" | "down" | "move" | "up" | "swipe" | "pinch" | "drag" | "key" | "home" | "back"
    )
}

pub fn clamp_xy(v: f64) -> Option<f64> {
    if !v.is_finite() {
        return None;
    }
    if !(0.0..=1.0).contains(&v) {
        return None;
    }
    Some(v)
}

pub fn validate_input_event_payload(payload: &serde_json::Value) -> Result<(), BridgeError> {
    let action = payload.get("action").and_then(|v| v.as_str()).unwrap_or("");
    if !is_valid_input_action(action) {
        return Err(BridgeError::Validation(format!("invalid action: {}", action)));
    }

    // home/back don't require coords
    let needs_coords = !matches!(action, "home" | "back" | "key");
    // for key, coords optional but keyCode required
    if action == "key" {
        let key_code = payload.get("keyCode").and_then(|v| v.as_i64());
        if key_code.is_none() {
            return Err(BridgeError::Validation("key action requires keyCode".into()));
        }
        if let Some(k) = key_code {
            if k < 0 || k > 1000 {
                return Err(BridgeError::Validation(format!("invalid keyCode: {}", k)));
            }
        }
        // key may have no x/y, that's ok
    }

    if needs_coords {
        let x = payload.get("x").and_then(|v| v.as_f64());
        let y = payload.get("y").and_then(|v| v.as_f64());
        match (x, y) {
            (Some(xv), Some(yv)) => {
                if clamp_xy(xv).is_none() {
                    return Err(BridgeError::Validation(format!("invalid x: {}", xv)));
                }
                if clamp_xy(yv).is_none() {
                    return Err(BridgeError::Validation(format!("invalid y: {}", yv)));
                }
            }
            _ => return Err(BridgeError::Validation("missing x/y for action requiring coords".into())),
        }
    } else if action != "key" {
        // for home/back, if x/y present, validate them but don't require
        if let Some(x) = payload.get("x").and_then(|v| v.as_f64()) {
            if clamp_xy(x).is_none() {
                return Err(BridgeError::Validation(format!("invalid x: {}", x)));
            }
        }
        if let Some(y) = payload.get("y").and_then(|v| v.as_f64()) {
            if clamp_xy(y).is_none() {
                return Err(BridgeError::Validation(format!("invalid y: {}", y)));
            }
        }
    } else {
        // key action: if x/y present, validate
        if let Some(x) = payload.get("x").and_then(|v| v.as_f64()) {
            if clamp_xy(x).is_none() {
                return Err(BridgeError::Validation(format!("invalid x: {}", x)));
            }
        }
        if let Some(y) = payload.get("y").and_then(|v| v.as_f64()) {
            if clamp_xy(y).is_none() {
                return Err(BridgeError::Validation(format!("invalid y: {}", y)));
            }
        }
    }

    if let Some(pid) = payload.get("pointerId").and_then(|v| v.as_i64()) {
        if pid < 0 || pid > 9 {
            return Err(BridgeError::Validation(format!("invalid pointerId: {}", pid)));
        }
    }
    if let Some(p) = payload.get("pressure").and_then(|v| v.as_f64()) {
        if !p.is_finite() || p < 0.0 || p > 1.0 {
            return Err(BridgeError::Validation(format!("invalid pressure: {}", p)));
        }
    }
    if let Some(d) = payload.get("durationMs").and_then(|v| v.as_i64()) {
        if d < 0 || d > 5000 {
            return Err(BridgeError::Validation(format!("invalid durationMs: {}", d)));
        }
    }
    if let Some(s) = payload.get("scale").and_then(|v| v.as_f64()) {
        if !s.is_finite() || s < 0.1 || s > 5.0 {
            return Err(BridgeError::Validation(format!("invalid scale: {}", s)));
        }
        if action != "pinch" {
            // scale only valid for pinch, but allow ignore? strict: error if scale present but not pinch
            // For TDD, we consider scale invalid unless pinch
            // However test expects scale valid for pinch only, so we already check above.
        } else if !(0.1..=5.0).contains(&s) {
            return Err(BridgeError::Validation(format!("invalid scale: {}", s)));
        }
    }
    if let Some(scale) = payload.get("scale").and_then(|v| v.as_f64()) {
        if action != "pinch" {
            return Err(BridgeError::Validation("scale only valid for pinch".into()));
        }
        let _ = scale;
    }
    if let Some(did) = payload.get("displayId").and_then(|v| v.as_i64()) {
        if did < 0 {
            return Err(BridgeError::Validation(format!("invalid displayId: {}", did)));
        }
    }
    // displayId is optional, defaults 0, so missing is ok

    Ok(())
}

pub fn should_throttle(last_ts: i64, now_ts: i64, throttle_ms: i64) -> bool {
    now_ts - last_ts < throttle_ms
}

pub fn validate_display_info_payload(payload: &serde_json::Value) -> Result<(), BridgeError> {
    // Accept either single object or with displays array
    if let Some(arr) = payload.get("displays").and_then(|v| v.as_array()) {
        if arr.is_empty() {
            return Err(BridgeError::Validation("displays empty".into()));
        }
        for d in arr {
            if d.get("displayId").and_then(|v| v.as_i64()).is_none() {
                return Err(BridgeError::Validation("missing displayId".into()));
            }
            if d.get("width").and_then(|v| v.as_i64()).is_none() || d.get("height").and_then(|v| v.as_i64()).is_none() {
                return Err(BridgeError::Validation("missing width/height".into()));
            }
        }
        return Ok(());
    }
    let did = payload.get("displayId").and_then(|v| v.as_i64());
    if did.is_none() && payload.get("displays").is_none() {
        // If payload is empty, allow? but for strict we require at least displayId or displays
        // For display.info single, require displayId
        return Err(BridgeError::Validation("missing displayId".into()));
    }
    Ok(())
}

pub fn validate_control_start_payload(payload: &serde_json::Value) -> Result<(), BridgeError> {
    if let Some(did) = payload.get("displayId").and_then(|v| v.as_i64()) {
        if did < 0 {
            return Err(BridgeError::Validation(format!("invalid displayId: {}", did)));
        }
    }
    if let Some(q) = payload.get("quality").and_then(|v| v.as_i64()) {
        if q < 1 || q > 100 {
            return Err(BridgeError::Validation(format!("invalid quality: {}", q)));
        }
    }
    if let Some(fps) = payload.get("fps").and_then(|v| v.as_i64()) {
        if fps < 1 || fps > 120 {
            return Err(BridgeError::Validation(format!("invalid fps: {}", fps)));
        }
    }
    Ok(())
}

// ── Storage Deep — Phase 5 ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum StorageState {
    Idle,
    Scanning,
    Syncing,
    Conflict,
    Done,
}

impl StorageState {
    pub fn can_transition(&self, next: &StorageState) -> bool {
        matches!(
            (self, next),
            (StorageState::Idle, StorageState::Scanning)
                | (StorageState::Idle, StorageState::Done)
                | (StorageState::Scanning, StorageState::Syncing)
                | (StorageState::Scanning, StorageState::Done)
                | (StorageState::Syncing, StorageState::Conflict)
                | (StorageState::Syncing, StorageState::Done)
                | (StorageState::Syncing, StorageState::Idle)
                | (StorageState::Conflict, StorageState::Syncing)
                | (StorageState::Conflict, StorageState::Idle)
                | (StorageState::Done, StorageState::Idle)
        )
    }
}

pub fn validate_storage_path(path: &str) -> Result<(), BridgeError> {
    if path.is_empty() {
        return Err(BridgeError::Validation("path empty".into()));
    }
    if path.len() > 4096 {
        return Err(BridgeError::Validation("path too long".into()));
    }
    if path.contains('\0') {
        return Err(BridgeError::Validation("path contains NUL".into()));
    }
    // allow "/" root; otherwise check for traversal
    if path != "/" {
        for seg in path.split('/') {
            if seg.is_empty() || seg == "." {
                continue;
            }
            if seg == ".." {
                return Err(BridgeError::Validation(format!("path traversal: {}", path)));
            }
            if seg.len() > 255 {
                return Err(BridgeError::Validation("segment too long".into()));
            }
        }
        // reject double traversal like "/a/../../b"
        // Already covered by seg check; additionally check ".." substring not inside?
        if path.contains("..") {
            // Ensure no ".." as segment already, but be strict
            let mut segs: Vec<&str> = Vec::new();
            for seg in path.split('/') {
                if seg.is_empty() || seg == "." { continue; }
                if seg == ".." {
                    return Err(BridgeError::Validation(format!("path traversal: {}", path)));
                }
                segs.push(seg);
            }
        }
    }
    Ok(())
}

pub fn sanitize_storage_path(path: &str) -> Result<String, BridgeError> {
    validate_storage_path(path)?;
    if path == "/" || path.is_empty() {
        return Ok(String::new());
    }
    let mut parts: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        // already validated no ".."
        parts.push(seg);
    }
    Ok(parts.join("/"))
}

pub fn validate_storage_ls_payload(payload: &serde_json::Value) -> Result<(), BridgeError> {
    let path = payload.get("path").and_then(|v| v.as_str()).ok_or_else(|| BridgeError::Validation("missing path".into()))?;
    validate_storage_path(path)?;
    Ok(())
}

pub fn validate_storage_mkdir_payload(payload: &serde_json::Value) -> Result<(), BridgeError> {
    let path = payload.get("path").and_then(|v| v.as_str()).ok_or_else(|| BridgeError::Validation("missing path".into()))?;
    validate_storage_path(path)?;
    if path == "/" {
        return Err(BridgeError::Validation("cannot mkdir root".into()));
    }
    Ok(())
}

pub fn validate_storage_rm_payload(payload: &serde_json::Value) -> Result<(), BridgeError> {
    let path = payload.get("path").and_then(|v| v.as_str()).ok_or_else(|| BridgeError::Validation("missing path".into()))?;
    validate_storage_path(path)?;
    if path == "/" {
        return Err(BridgeError::Validation("cannot rm root".into()));
    }
    Ok(())
}

pub fn validate_storage_sync_payload(payload: &serde_json::Value) -> Result<(), BridgeError> {
    let path = payload.get("path").and_then(|v| v.as_str()).ok_or_else(|| BridgeError::Validation("missing path".into()))?;
    validate_storage_path(path)?;
    if path == "/" {
        return Err(BridgeError::Validation("cannot sync root".into()));
    }
    let sha = payload.get("sha256").and_then(|v| v.as_str()).ok_or_else(|| BridgeError::Validation("missing sha256".into()))?;
    if sha.len() != 64 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(BridgeError::Validation(format!("invalid sha256: {}", sha)));
    }
    let offset = payload.get("offset").and_then(|v| v.as_u64()).ok_or_else(|| BridgeError::Validation("missing offset".into()))?;
    let size = payload.get("size").and_then(|v| v.as_u64()).ok_or_else(|| BridgeError::Validation("missing size".into()))?;
    if size == 0 {
        return Err(BridgeError::Validation("size 0".into()));
    }
    if offset >= size {
        return Err(BridgeError::Validation(format!("offset {} >= size {}", offset, size)));
    }
    let total = payload.get("total").and_then(|v| v.as_u64()).ok_or_else(|| BridgeError::Validation("missing total".into()))?;
    let index = payload.get("index").and_then(|v| v.as_u64()).ok_or_else(|| BridgeError::Validation("missing index".into()))?;
    if total == 0 {
        return Err(BridgeError::Validation("total 0".into()));
    }
    if index >= total {
        return Err(BridgeError::Validation(format!("index {} >= total {}", index, total)));
    }
    // Validate offset alignment: must be index * CHUNK_SIZE (1MB) except maybe but we enforce
    const CHUNK: u64 = 1024 * 1024;
    if offset != index * CHUNK && !(index == total - 1 && offset % CHUNK == 0) {
        // Allow last chunk offset to be index*CHUNK; but we also enforce offset == index*CHUNK strictly
        if offset != index * CHUNK {
            return Err(BridgeError::Validation(format!("offset {} != index {} * chunk {}", offset, index, CHUNK)));
        }
    }
    if size > 50 * 1024 * 1024 * 1024 {
        return Err(BridgeError::Validation("size > 50GiB".into()));
    }
    // vectorClock optional: validate if present
    if let Some(vc) = payload.get("vectorClock").and_then(|v| v.as_object()) {
        for (k, v) in vc {
            if k.is_empty() || k.len() > 64 {
                return Err(BridgeError::Validation(format!("invalid vector key: {}", k)));
            }
            if !k.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                return Err(BridgeError::Validation(format!("invalid vector key chars: {}", k)));
            }
            if v.as_u64().is_none() {
                return Err(BridgeError::Validation(format!("invalid vector value for {}: {}", k, v)));
            }
        }
    }
    Ok(())
}

pub fn validate_storage_stat_payload(payload: &serde_json::Value) -> Result<(), BridgeError> {
    let path = payload.get("path").and_then(|v| v.as_str()).ok_or_else(|| BridgeError::Validation("missing path".into()))?;
    validate_storage_path(path)?;
    Ok(())
}

pub fn validate_storage_conflict_payload(payload: &serde_json::Value) -> Result<(), BridgeError> {
    let path = payload.get("path").and_then(|v| v.as_str()).ok_or_else(|| BridgeError::Validation("missing path".into()))?;
    validate_storage_path(path)?;
    let res = payload.get("resolution").and_then(|v| v.as_str()).unwrap_or("");
    if !matches!(res, "lww" | "rename" | "manual") {
        return Err(BridgeError::Validation(format!("invalid resolution: {}", res)));
    }
    Ok(())
}

pub fn vector_clock_dominates(a: &std::collections::HashMap<String, u64>, b: &std::collections::HashMap<String, u64>) -> bool {
    // a dominates b if for all keys in b, a[k] >= b[k], and exists at least one strictly greater
    // Keys only in a count as b[k]=0
    let mut all_ge = true;
    let mut strictly_greater = false;
    for (k, bv) in b {
        let av = a.get(k).copied().unwrap_or(0);
        if av < *bv {
            all_ge = false;
            break;
        }
        if av > *bv {
            strictly_greater = true;
        }
    }
    if !all_ge {
        return false;
    }
    // check keys only in a where a[k] > 0 => greater
    for (k, av) in a {
        if !b.contains_key(k) && *av > 0 {
            strictly_greater = true;
            break;
        }
    }
    // also if a has extra keys with >0 it's greater; if sizes equal and all equal, not dominates
    strictly_greater
}

pub fn is_vector_concurrent(a: &std::collections::HashMap<String, u64>, b: &std::collections::HashMap<String, u64>) -> bool {
    if vectors_equal(a, b) {
        return false;
    }
    !vector_clock_dominates(a, b) && !vector_clock_dominates(b, a)
}

fn vectors_equal(a: &std::collections::HashMap<String, u64>, b: &std::collections::HashMap<String, u64>) -> bool {
    let mut keys = std::collections::HashSet::new();
    for k in a.keys() { keys.insert(k); }
    for k in b.keys() { keys.insert(k); }
    for k in keys {
        if a.get(k).copied().unwrap_or(0) != b.get(k).copied().unwrap_or(0) {
            return false;
        }
    }
    true
}

pub fn vector_clock_merge(a: &std::collections::HashMap<String, u64>, b: &std::collections::HashMap<String, u64>) -> std::collections::HashMap<String, u64> {
    let mut out = a.clone();
    for (k, bv) in b {
        let av = out.get(k).copied().unwrap_or(0);
        out.insert(k.clone(), (*bv).max(av));
    }
    out
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeMessage {
    pub v: u8,
    pub id: String,
    #[serde(rename = "type")]
    pub typ: MessageType,
    pub ts: i64,
    pub nonce: String,
    pub payload: serde_json::Value,
}

impl BridgeMessage {
    pub fn new(typ: MessageType, payload: serde_json::Value) -> Self {
        Self {
            v: 1,
            id: Uuid::new_v4().to_string(),
            typ,
            ts: chrono::Utc::now().timestamp_millis(),
            nonce: format!("{:08x}", rand::random::<u32>()),
            payload,
        }
    }
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}
