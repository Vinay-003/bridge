use serde::{Deserialize, Serialize};
use uuid::Uuid;
use thiserror::Error;
use base64::Engine as _;

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
    // telephony — Phase 3
    #[serde(rename = "sms.list")] SmsList,
    #[serde(rename = "sms.send")] SmsSend,
    #[serde(rename = "sms.received")] SmsReceived,
    #[serde(rename = "call.start")] CallStart,
    #[serde(rename = "call.answer")] CallAnswer,
    #[serde(rename = "call.hangup")] CallHangup,
    #[serde(rename = "call.audio")] CallAudio,
    #[serde(rename = "call.log")] CallLog,
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
    // relay + mesh — Phase 6 Global Relay + Multi-device Mesh
    #[serde(rename = "relay.announce")] RelayAnnounce,
    #[serde(rename = "relay.relay")] RelayRelay,
    #[serde(rename = "mesh.sync")] MeshSync,
    #[serde(rename = "mesh.conflict")] MeshConflict,
    // plugin — Phase 7 Plugin Platform
    #[serde(rename = "plugin.list")] PluginList,
    #[serde(rename = "plugin.load")] PluginLoad,
    #[serde(rename = "plugin.emit")] PluginEmit,
    // ai — Phase 7 AI
    #[serde(rename = "ai.summarize")] AiSummarize,
    #[serde(rename = "ai.transcribe")] AiTranscribe,
    #[serde(rename = "ai.result")] AiResult,
    // error
    #[serde(rename = "error")] Error,
}

// Telephony helpers (validation, state machine)
#[derive(Debug, Clone, PartialEq)]
pub enum CallState {
    Idle,
    Ringing,
    Offhook,
    Hungup,
}

impl CallState {
    pub fn can_transition(&self, next: &CallState) -> bool {
        matches!(
            (self, next),
            (CallState::Idle, CallState::Ringing)
                | (CallState::Ringing, CallState::Offhook)
                | (CallState::Ringing, CallState::Hungup)
                | (CallState::Offhook, CallState::Hungup)
                | (CallState::Hungup, CallState::Idle)
                | (CallState::Idle, CallState::Offhook) // emergency fallback
        )
    }
}

pub fn is_valid_phone_number(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return false;
    }
    let digits: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() < 7 || digits.len() > 15 {
        return false;
    }
    // allow +, spaces, dashes, parentheses
    trimmed
        .chars()
        .all(|c| c.is_ascii_digit() || c == '+' || c == ' ' || c == '-' || c == '(' || c == ')')
        && trimmed.chars().filter(|c| c.is_ascii_digit()).count() >= 7
}

pub fn is_valid_sms_body(s: &str) -> bool {
    !s.is_empty() && s.chars().count() <= 918 && s.len() <= 4096
}

pub fn validate_sms_send_payload(payload: &serde_json::Value) -> Result<(), BridgeError> {
    let addr = payload.get("address").and_then(|v| v.as_str()).unwrap_or("");
    let body = payload.get("body").and_then(|v| v.as_str()).unwrap_or("");
    if !is_valid_phone_number(addr) {
        return Err(BridgeError::Validation(format!("invalid number: {}", addr)));
    }
    if !is_valid_sms_body(body) {
        return Err(BridgeError::Validation(format!(
            "invalid body len {}",
            body.chars().count()
        )));
    }
    if let Some(sub) = payload.get("subscriptionId").and_then(|v| v.as_i64()) {
        if sub < 0 {
            return Err(BridgeError::Validation("invalid subscriptionId".into()));
        }
    }
    Ok(())
}

pub fn validate_call_start_payload(payload: &serde_json::Value) -> Result<(), BridgeError> {
    let number = payload.get("number").and_then(|v| v.as_str()).unwrap_or("");
    if !is_valid_phone_number(number) {
        return Err(BridgeError::Validation(format!("invalid number: {}", number)));
    }
    if let Some(sub) = payload.get("subscriptionId").and_then(|v| v.as_i64()) {
        if sub < 0 {
            return Err(BridgeError::Validation("invalid subscriptionId".into()));
        }
    }
    Ok(())
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

// ── Relay — Phase 6 ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum RelayState {
    Disconnected,
    Announcing,
    HolePunching,
    RelayReady,
    ConnectedDirect,
    ConnectedViaRelay,
    Failed,
}

impl RelayState {
    pub fn can_transition(&self, next: &RelayState) -> bool {
        matches!(
            (self, next),
            (RelayState::Disconnected, RelayState::Announcing)
                | (RelayState::Announcing, RelayState::HolePunching)
                | (RelayState::Announcing, RelayState::RelayReady)
                | (RelayState::Announcing, RelayState::Failed)
                | (RelayState::Announcing, RelayState::Disconnected)
                | (RelayState::HolePunching, RelayState::ConnectedDirect)
                | (RelayState::HolePunching, RelayState::RelayReady)
                | (RelayState::HolePunching, RelayState::Failed)
                | (RelayState::RelayReady, RelayState::ConnectedViaRelay)
                | (RelayState::RelayReady, RelayState::Disconnected)
                | (RelayState::ConnectedDirect, RelayState::Disconnected)
                | (RelayState::ConnectedDirect, RelayState::RelayReady)
                | (RelayState::ConnectedViaRelay, RelayState::Disconnected)
                | (RelayState::Failed, RelayState::Disconnected)
                | (RelayState::Disconnected, RelayState::Failed)
        )
    }
}

pub const RELAY_ANNOUNCE_URL: &str = "https://relay.bridge.dev/v1/announce";
pub const STUN_SERVER: &str = "stun.l.google.com:19302";

pub fn is_valid_device_id(s: &str) -> bool {
    if s.is_empty() || s.len() > 64 {
        return false;
    }
    s.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
}

pub fn is_valid_stun_server(s: &str) -> bool {
    // host:port
    if let Some((host, port_str)) = s.rsplit_once(':') {
        if host.is_empty() || host.len() > 253 { return false; }
        if let Ok(port) = port_str.parse::<u16>() {
            if port == 0 { return false; }
            // host must be dot-separated or single label, no spaces
            if host.contains(' ') || host.contains('\0') { return false; }
            return true;
        }
    }
    false
}

pub fn is_opaque_blob(s: &str) -> bool {
    // base64 opaque, 16..1M chars (decoded ≤ 1MB is checked elsewhere)
    if s.len() < 16 || s.len() > 1_400_000 { return false; }
    // allow base64 chars + padding
    s.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
}

pub fn validate_relay_announce_payload(payload: &serde_json::Value) -> Result<(), BridgeError> {
    let device_id = payload.get("deviceId").and_then(|v| v.as_str()).ok_or_else(|| BridgeError::Validation("missing deviceId".into()))?;
    if !is_valid_device_id(device_id) {
        return Err(BridgeError::Validation(format!("invalid deviceId: {}", device_id)));
    }
    let blob = payload.get("blob").and_then(|v| v.as_str()).ok_or_else(|| BridgeError::Validation("missing blob".into()))?;
    if !is_opaque_blob(blob) {
        return Err(BridgeError::Validation("invalid blob (must be base64 16..1M)".into()));
    }
    // blob decoded size check ≤ 1MB
    if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(blob) {
        if decoded.len() > 1024 * 1024 { return Err(BridgeError::Validation("blob too large >1MB decoded".into())); }
        if decoded.len() < 12 { return Err(BridgeError::Validation("blob too small".into())); }
    } else {
        // try without padding variation via STANDARD; also allow URL_SAFE? we enforce STANDARD.
        return Err(BridgeError::Validation("invalid blob base64".into()));
    }
    if let Some(ts) = payload.get("ts").and_then(|v| v.as_i64()) {
        let now = chrono::Utc::now().timestamp_millis();
        if (now - ts).abs() > 5 * 60 * 1000 {
            return Err(BridgeError::Validation(format!("clock skew ts {} vs now {}", ts, now)));
        }
    }
    if let Some(stun) = payload.get("stunServer").and_then(|v| v.as_str()) {
        if !is_valid_stun_server(stun) {
            return Err(BridgeError::Validation(format!("invalid stunServer: {}", stun)));
        }
    }
    if let Some(mapped) = payload.get("mappedAddr").and_then(|v| v.as_str()) {
        if mapped.parse::<std::net::SocketAddr>().is_err() {
            return Err(BridgeError::Validation(format!("invalid mappedAddr: {}", mapped)));
        }
    }
    if let Some(fp) = payload.get("fp").and_then(|v| v.as_str()) {
        if fp.len() != 12 || !fp.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(BridgeError::Validation(format!("invalid fp: {}", fp)));
        }
    }
    Ok(())
}

pub fn validate_relay_relay_payload(payload: &serde_json::Value) -> Result<(), BridgeError> {
    let to = payload.get("to").and_then(|v| v.as_str()).ok_or_else(|| BridgeError::Validation("missing to".into()))?;
    if !is_valid_device_id(to) { return Err(BridgeError::Validation(format!("invalid to: {}", to))); }
    let from = payload.get("from").and_then(|v| v.as_str()).ok_or_else(|| BridgeError::Validation("missing from".into()))?;
    if !is_valid_device_id(from) { return Err(BridgeError::Validation(format!("invalid from: {}", from))); }
    let blob = payload.get("blob").and_then(|v| v.as_str()).ok_or_else(|| BridgeError::Validation("missing blob".into()))?;
    if !is_opaque_blob(blob) { return Err(BridgeError::Validation("invalid blob".into())); }
    if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(blob) {
        if decoded.len() > 1024 * 1024 { return Err(BridgeError::Validation("blob too large".into())); }
    } else {
        return Err(BridgeError::Validation("invalid blob base64".into()));
    }
    if let Some(ts) = payload.get("ts").and_then(|v| v.as_i64()) {
        let now = chrono::Utc::now().timestamp_millis();
        if (now - ts).abs() > 5 * 60 * 1000 {
            return Err(BridgeError::Validation(format!("clock skew ts {}", ts)));
        }
    }
    if let Some(nonce) = payload.get("nonce").and_then(|v| v.as_str()) {
        if nonce.len() != 8 || !nonce.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(BridgeError::Validation(format!("invalid nonce: {}", nonce)));
        }
    }
    Ok(())
}

// ── Mesh — Phase 6 ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum MeshState {
    Idle,
    Syncing,
    Conflict,
}

impl MeshState {
    pub fn can_transition(&self, next: &MeshState) -> bool {
        matches!(
            (self, next),
            (MeshState::Idle, MeshState::Syncing)
                | (MeshState::Syncing, MeshState::Idle)
                | (MeshState::Syncing, MeshState::Conflict)
                | (MeshState::Conflict, MeshState::Syncing)
                | (MeshState::Conflict, MeshState::Idle)
        )
    }
}

// LWW for clipboard
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LwwClipboard {
    pub text: String,
    pub mime: String,
    pub ts: i64,
    pub device_id: String,
}

pub fn lww_clipboard_merge(a: &LwwClipboard, b: &LwwClipboard) -> LwwClipboard {
    if b.ts > a.ts { b.clone() }
    else if b.ts < a.ts { a.clone() }
    else {
        // tie break device_id lex
        if b.device_id > a.device_id { b.clone() } else { a.clone() }
    }
}

pub fn validate_mesh_sync_payload(payload: &serde_json::Value) -> Result<(), BridgeError> {
    let device_id = payload.get("deviceId").and_then(|v| v.as_str()).ok_or_else(|| BridgeError::Validation("missing deviceId".into()))?;
    if !is_valid_device_id(device_id) { return Err(BridgeError::Validation(format!("invalid deviceId: {}", device_id))); }
    // vectors is optional? spec says required
    if let Some(vectors) = payload.get("vectors").and_then(|v| v.as_object()) {
        for (k, v) in vectors {
            if !is_valid_device_id(k) { return Err(BridgeError::Validation(format!("invalid vector key: {}", k))); }
            if v.as_u64().is_none() { return Err(BridgeError::Validation(format!("invalid vector value for {}: {}", k, v))); }
        }
    } else if payload.get("vectors").is_some() {
        return Err(BridgeError::Validation("invalid vectors".into()));
    } else {
        return Err(BridgeError::Validation("missing vectors".into()));
    }
    if let Some(entries) = payload.get("entries").and_then(|v| v.as_array()) {
        if entries.len() > 100 {
            return Err(BridgeError::Validation("entries >100".into()));
        }
        for e in entries {
            if let Some(path) = e.get("path").and_then(|v| v.as_str()) {
                validate_storage_path(path)?;
            } else {
                return Err(BridgeError::Validation("entry missing path".into()));
            }
            if let Some(vc) = e.get("vector").and_then(|v| v.as_object()) {
                for (k, v) in vc {
                    if !is_valid_device_id(k) { return Err(BridgeError::Validation(format!("invalid entry vector key: {}", k))); }
                    if v.as_u64().is_none() { return Err(BridgeError::Validation(format!("invalid entry vector value for {}: {}", k, v))); }
                }
            }
            if let Some(lww) = e.get("lww").and_then(|v| v.as_object()) {
                let text = lww.get("text").and_then(|v| v.as_str()).unwrap_or("");
                if text.len() > 1024 * 1024 { return Err(BridgeError::Validation("lww text too large".into())); }
                let ts = lww.get("ts").and_then(|v| v.as_i64()).ok_or_else(|| BridgeError::Validation("missing lww ts".into()))?;
                let now = chrono::Utc::now().timestamp_millis();
                if (now - ts).abs() > 5 * 60 * 1000 { return Err(BridgeError::Validation("lww clock skew".into())); }
            }
            if let Some(sha) = e.get("sha256").and_then(|v| v.as_str()) {
                if sha.len() != 64 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(BridgeError::Validation(format!("invalid sha256: {}", sha)));
                }
            }
        }
    }
    if let Some(ts) = payload.get("ts").and_then(|v| v.as_i64()) {
        let now = chrono::Utc::now().timestamp_millis();
        if (now - ts).abs() > 5 * 60 * 1000 {
            return Err(BridgeError::Validation(format!("clock skew ts {}", ts)));
        }
    }
    Ok(())
}

pub fn validate_mesh_conflict_payload(payload: &serde_json::Value) -> Result<(), BridgeError> {
    let path = payload.get("path").and_then(|v| v.as_str()).ok_or_else(|| BridgeError::Validation("missing path".into()))?;
    validate_storage_path(path)?;
    let res = payload.get("resolution").and_then(|v| v.as_str()).unwrap_or("");
    if !matches!(res, "lww" | "rename" | "manual") {
        return Err(BridgeError::Validation(format!("invalid resolution: {}", res)));
    }
    let winner = payload.get("winner").and_then(|v| v.as_str()).unwrap_or("");
    if !matches!(winner, "local" | "remote") {
        return Err(BridgeError::Validation(format!("invalid winner: {}", winner)));
    }
    if let Some(lr) = payload.get("loserRename").and_then(|v| v.as_str()) {
        if !lr.is_empty() {
            // loserRename may be path-like, validate not traversal absolute? allow "/file.conflict-..."
            // Ensure no ".."
            if lr.contains("..") { return Err(BridgeError::Validation(format!("invalid loserRename: {}", lr))); }
            if lr.len() > 4096 { return Err(BridgeError::Validation("loserRename too long".into())); }
        }
    }
    Ok(())
}

// ── Plugin — Phase 7 ──────────────────────────────────────────────────────

pub const ALLOWED_PLUGIN_CAPS: &[&str] = &["notify", "clipboard", "storage", "ai.summarize", "ai.transcribe"];

#[derive(Debug, Clone, PartialEq)]
pub enum PluginState {
    Unloaded,
    Loading,
    Loaded,
    Running,
    Reloading,
    Failed,
    Disabled,
}

impl PluginState {
    pub fn can_transition(&self, next: &PluginState) -> bool {
        matches!(
            (self, next),
            (PluginState::Unloaded, PluginState::Loading)
                | (PluginState::Loading, PluginState::Loaded)
                | (PluginState::Loading, PluginState::Failed)
                | (PluginState::Loaded, PluginState::Running)
                | (PluginState::Running, PluginState::Reloading)
                | (PluginState::Reloading, PluginState::Running)
                | (PluginState::Reloading, PluginState::Failed)
                | (PluginState::Running, PluginState::Failed)
                | (PluginState::Failed, PluginState::Loading)
                | (PluginState::Running, PluginState::Disabled)
                | (PluginState::Disabled, PluginState::Loading)
                | (PluginState::Running, PluginState::Unloaded)
                | (PluginState::Loaded, PluginState::Failed)
                | (PluginState::Failed, PluginState::Unloaded)
        )
    }
}

pub fn is_valid_plugin_id(s: &str) -> bool {
    if s.len() < 3 || s.len() > 32 { return false; }
    s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

pub fn is_valid_plugin_version(s: &str) -> bool {
    // semver x.y.z
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 { return false; }
    for p in parts {
        if p.is_empty() || p.len() > 5 { return false; }
        if !p.chars().all(|c| c.is_ascii_digit()) { return false; }
        // no leading zeros unless single zero? allow but stricter: if len>1 and starts with 0 -> false
        if p.len() > 1 && p.starts_with('0') { return false; }
    }
    true
}

pub fn sanitize_plugin_path(entry: &str) -> Result<String, BridgeError> {
    if entry.is_empty() { return Err(BridgeError::Validation("entry empty".into())); }
    if entry.len() > 256 { return Err(BridgeError::Validation("entry too long".into())); }
    if entry.contains('\0') { return Err(BridgeError::Validation("entry contains NUL".into())); }
    if entry.starts_with('/') || entry.starts_with('\\') {
        return Err(BridgeError::Validation("entry must be relative".into()));
    }
    for seg in entry.split('/') {
        if seg == ".." { return Err(BridgeError::Validation(format!("path traversal in entry: {}", entry))); }
        if seg.contains('\\') { return Err(BridgeError::Validation("entry contains backslash".into())); }
    }
    if !(entry.ends_with(".js") || entry.ends_with(".wasm")) {
        return Err(BridgeError::Validation("entry must end with .js or .wasm".into()));
    }
    Ok(entry.to_string())
}

pub fn validate_plugin_manifest(payload: &serde_json::Value) -> Result<(), BridgeError> {
    let name = payload.get("name").and_then(|v| v.as_str()).ok_or_else(|| BridgeError::Validation("missing name".into()))?;
    if !is_valid_plugin_id(name) { return Err(BridgeError::Validation(format!("invalid plugin name: {}", name))); }
    let version = payload.get("version").and_then(|v| v.as_str()).ok_or_else(|| BridgeError::Validation("missing version".into()))?;
    if !is_valid_plugin_version(version) { return Err(BridgeError::Validation(format!("invalid version: {}", version))); }
    let entry = payload.get("entry").and_then(|v| v.as_str()).ok_or_else(|| BridgeError::Validation("missing entry".into()))?;
    sanitize_plugin_path(entry)?;
    let caps = payload.get("capabilities").and_then(|v| v.as_array()).ok_or_else(|| BridgeError::Validation("missing capabilities".into()))?;
    if caps.is_empty() { return Err(BridgeError::Validation("capabilities empty".into())); }
    for c in caps {
        let s = c.as_str().ok_or_else(|| BridgeError::Validation("capability not string".into()))?;
        if !ALLOWED_PLUGIN_CAPS.contains(&s) {
            return Err(BridgeError::Validation(format!("invalid capability: {}", s)));
        }
    }
    // bridgeVersion if present must be "1"
    if let Some(bv) = payload.get("bridgeVersion").and_then(|v| v.as_str()) {
        if bv != "1" { return Err(BridgeError::Validation(format!("invalid bridgeVersion: {}", bv))); }
    }
    Ok(())
}

pub fn validate_plugin_load_payload(payload: &serde_json::Value) -> Result<(), BridgeError> {
    let plugin_id = payload.get("pluginId").and_then(|v| v.as_str()).ok_or_else(|| BridgeError::Validation("missing pluginId".into()))?;
    if !is_valid_plugin_id(plugin_id) { return Err(BridgeError::Validation(format!("invalid pluginId: {}", plugin_id))); }
    Ok(())
}

pub fn can_plugin_access(capabilities: &[String], needed: &str) -> bool {
    capabilities.iter().any(|c| c == needed)
}

// ── AI — Phase 7 ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum AiState {
    Idle,
    Queued,
    Local,
    Cloud,
    Done,
    Failed,
}

impl AiState {
    pub fn can_transition(&self, next: &AiState) -> bool {
        matches!(
            (self, next),
            (AiState::Idle, AiState::Queued)
                | (AiState::Queued, AiState::Local)
                | (AiState::Queued, AiState::Cloud)
                | (AiState::Queued, AiState::Failed)
                | (AiState::Local, AiState::Done)
                | (AiState::Local, AiState::Cloud)
                | (AiState::Local, AiState::Failed)
                | (AiState::Cloud, AiState::Done)
                | (AiState::Cloud, AiState::Failed)
                | (AiState::Done, AiState::Idle)
                | (AiState::Failed, AiState::Idle)
        )
    }
}

pub fn validate_ai_summarize_payload(payload: &serde_json::Value) -> Result<(), BridgeError> {
    let notifs = payload.get("notifications").and_then(|v| v.as_array()).ok_or_else(|| BridgeError::Validation("missing notifications".into()))?;
    if notifs.is_empty() || notifs.len() > 20 {
        return Err(BridgeError::Validation(format!("notifications len {} invalid 1..20", notifs.len())));
    }
    let mut total_chars: usize = 0;
    for n in notifs {
        let app = n.get("app").and_then(|v| v.as_str()).unwrap_or("");
        let body = n.get("body").and_then(|v| v.as_str()).unwrap_or("");
        if app.is_empty() || app.len() > 64 { return Err(BridgeError::Validation(format!("invalid app: {}", app))); }
        if body.len() > 500 { return Err(BridgeError::Validation(format!("body too long: {}", body.len()))); }
        total_chars += app.len() + body.len() + 50;
    }
    if total_chars > 10 * 1024 {
        return Err(BridgeError::Validation("total chars >10k".into()));
    }
    if let Some(max_len) = payload.get("maxLen").and_then(|v| v.as_u64()) {
        if max_len == 0 || max_len > 1000 {
            return Err(BridgeError::Validation(format!("invalid maxLen: {}", max_len)));
        }
    }
    // requestId if present validate uuid-ish
    if let Some(req) = payload.get("requestId").and_then(|v| v.as_str()) {
        if req.is_empty() || req.len() > 64 { return Err(BridgeError::Validation("invalid requestId".into())); }
    }
    Ok(())
}

pub fn validate_ai_transcribe_payload(payload: &serde_json::Value) -> Result<(), BridgeError> {
    let b64 = payload.get("audio_b64").and_then(|v| v.as_str()).ok_or_else(|| BridgeError::Validation("missing audio_b64".into()))?;
    if b64.is_empty() || b64.len() > 7_000_000 {
        return Err(BridgeError::Validation(format!("invalid audio_b64 len: {}", b64.len())));
    }
    if base64::engine::general_purpose::STANDARD.decode(b64).is_err() {
        return Err(BridgeError::Validation("invalid audio_b64 base64".into()));
    }
    let decoded_len = base64::engine::general_purpose::STANDARD.decode(b64).unwrap().len();
    if decoded_len > 5 * 1024 * 1024 {
        return Err(BridgeError::Validation("audio decoded >5MB".into()));
    }
    if decoded_len == 0 {
        return Err(BridgeError::Validation("audio empty".into()));
    }
    let fmt = payload.get("format").and_then(|v| v.as_str()).unwrap_or("");
    if !matches!(fmt, "opus" | "wav" | "mp3" | "m4a") {
        return Err(BridgeError::Validation(format!("invalid format: {}", fmt)));
    }
    if let Some(lang) = payload.get("lang").and_then(|v| v.as_str()) {
        if lang.len() != 2 || !lang.chars().all(|c| c.is_ascii_lowercase()) {
            return Err(BridgeError::Validation(format!("invalid lang: {}", lang)));
        }
    }
    if let Some(req) = payload.get("requestId").and_then(|v| v.as_str()) {
        if req.is_empty() || req.len() > 64 { return Err(BridgeError::Validation("invalid requestId".into())); }
    }
    Ok(())
}

pub fn validate_ai_result_payload(payload: &serde_json::Value) -> Result<(), BridgeError> {
    let kind = payload.get("kind").and_then(|v| v.as_str()).ok_or_else(|| BridgeError::Validation("missing kind".into()))?;
    if !matches!(kind, "summarize" | "transcribe") { return Err(BridgeError::Validation(format!("invalid kind: {}", kind))); }
    let text = payload.get("text").and_then(|v| v.as_str()).ok_or_else(|| BridgeError::Validation("missing text".into()))?;
    if text.len() > 5000 { return Err(BridgeError::Validation("text too long".into())); }
    let model = payload.get("model").and_then(|v| v.as_str()).ok_or_else(|| BridgeError::Validation("missing model".into()))?;
    if model.is_empty() || model.len() > 64 { return Err(BridgeError::Validation("invalid model".into())); }
    Ok(())
}

pub fn should_rate_limit_ai(timestamps: &mut Vec<i64>, now: i64, limit: usize, window_ms: i64) -> bool {
    timestamps.retain(|&t| now - t < window_ms);
    if timestamps.len() >= limit {
        true
    } else {
        timestamps.push(now);
        false
    }
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
