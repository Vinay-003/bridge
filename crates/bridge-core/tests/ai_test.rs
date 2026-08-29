use bridge_core::{AiState, validate_ai_summarize_payload, validate_ai_transcribe_payload, validate_ai_result_payload, should_rate_limit_ai};
use serde_json::json;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};

fn b64_audio(n: usize) -> String { B64.encode(vec![0x42u8; n]) }

#[test]
fn ai_state_valid() {
    assert!(AiState::Idle.can_transition(&AiState::Queued));
    assert!(AiState::Queued.can_transition(&AiState::Local));
    assert!(AiState::Queued.can_transition(&AiState::Cloud));
    assert!(AiState::Local.can_transition(&AiState::Done));
    assert!(AiState::Local.can_transition(&AiState::Cloud));
    assert!(AiState::Cloud.can_transition(&AiState::Done));
    assert!(AiState::Done.can_transition(&AiState::Idle));
}
#[test]
fn ai_state_invalid() {
    assert!(!AiState::Idle.can_transition(&AiState::Done));
    assert!(!AiState::Done.can_transition(&AiState::Queued));
}
#[test]
fn validate_summarize_ok() {
    let p = json!({"notifications":[{"app":"WhatsApp","body":"hello"}],"maxLen":200,"requestId":"req1"});
    assert!(validate_ai_summarize_payload(&p).is_ok());
}
#[test]
fn validate_summarize_empty() {
    let p = json!({"notifications":[],"maxLen":200});
    assert!(validate_ai_summarize_payload(&p).is_err());
}
#[test]
fn validate_summarize_too_many() {
    let many: Vec<_> = (0..21).map(|_| json!({"app":"A","body":"hi"})).collect();
    let p = json!({"notifications": many});
    assert!(validate_ai_summarize_payload(&p).is_err());
}
#[test]
fn validate_summarize_body_too_long() {
    let p = json!({"notifications":[{"app":"A","body": "a".repeat(501)}]});
    assert!(validate_ai_summarize_payload(&p).is_err());
}
#[test]
fn validate_transcribe_ok() {
    let b64 = b64_audio(100);
    let p = json!({"audio_b64":b64,"format":"opus","lang":"en","requestId":"req1"});
    assert!(validate_ai_transcribe_payload(&p).is_ok());
}
#[test]
fn validate_transcribe_invalid_format() {
    let b64 = b64_audio(10);
    let p = json!({"audio_b64":b64,"format":"evil"});
    assert!(validate_ai_transcribe_payload(&p).is_err());
}
#[test]
fn validate_transcribe_empty() {
    let p = json!({"audio_b64":"","format":"opus"});
    assert!(validate_ai_transcribe_payload(&p).is_err());
}
#[test]
fn validate_result_ok() {
    let p = json!({"kind":"summarize","text":"hi","model":"llama.cpp-local"});
    assert!(validate_ai_result_payload(&p).is_ok());
}
#[test]
fn validate_result_invalid_kind() {
    let p = json!({"kind":"evil","text":"hi","model":"m"});
    assert!(validate_ai_result_payload(&p).is_err());
}
#[test]
fn rate_limit_ai() {
    let mut v: Vec<i64> = Vec::new();
    for _ in 0..10 { assert!(!should_rate_limit_ai(&mut v, 1000, 10, 60000)); }
    assert!(should_rate_limit_ai(&mut v, 1000, 10, 60000));
    assert!(!should_rate_limit_ai(&mut v, 70000, 10, 60000));
}
#[test]
fn message_type_serde_ai() {
    let m = bridge_core::BridgeMessage::new(bridge_core::MessageType::AiSummarize, json!({"notifications":[{"app":"A","body":"hi"}]}));
    assert!(m.to_json().contains("ai.summarize"));
    let m2 = bridge_core::BridgeMessage::new(bridge_core::MessageType::AiTranscribe, json!({"audio_b64":b64_audio(10),"format":"opus"}));
    assert!(m2.to_json().contains("ai.transcribe"));
    let m3 = bridge_core::BridgeMessage::new(bridge_core::MessageType::AiResult, json!({"kind":"summarize","text":"hi","model":"x"}));
    assert!(m3.to_json().contains("ai.result"));
    let m4 = bridge_core::BridgeMessage::new(bridge_core::MessageType::PluginList, json!({}));
    assert!(m4.to_json().contains("plugin.list"));
}
