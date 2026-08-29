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
    // error
    #[serde(rename = "error")] Error,
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
