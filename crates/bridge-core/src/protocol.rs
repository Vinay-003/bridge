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
    // telephony — Phase 3
    #[serde(rename = "sms.list")] SmsList,
    #[serde(rename = "sms.send")] SmsSend,
    #[serde(rename = "sms.received")] SmsReceived,
    #[serde(rename = "call.start")] CallStart,
    #[serde(rename = "call.answer")] CallAnswer,
    #[serde(rename = "call.hangup")] CallHangup,
    #[serde(rename = "call.audio")] CallAudio,
    #[serde(rename = "call.log")] CallLog,
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
