use bridge_core::{PluginState, validate_plugin_manifest, validate_plugin_load_payload, can_plugin_access, is_valid_plugin_id, is_valid_plugin_version, sanitize_plugin_path, ALLOWED_PLUGIN_CAPS};
use serde_json::json;

#[test]
fn plugin_state_valid() {
    assert!(PluginState::Unloaded.can_transition(&PluginState::Loading));
    assert!(PluginState::Loading.can_transition(&PluginState::Loaded));
    assert!(PluginState::Loaded.can_transition(&PluginState::Running));
    assert!(PluginState::Running.can_transition(&PluginState::Reloading));
    assert!(PluginState::Reloading.can_transition(&PluginState::Running));
    assert!(PluginState::Running.can_transition(&PluginState::Disabled));
}
#[test]
fn plugin_state_invalid() {
    assert!(!PluginState::Unloaded.can_transition(&PluginState::Running));
    assert!(!PluginState::Running.can_transition(&PluginState::Loaded));
}

#[test]
fn is_valid_plugin_id_ok() {
    assert!(is_valid_plugin_id("example-translate"));
    assert!(is_valid_plugin_id("ab-c"));
    assert!(!is_valid_plugin_id("ab"));
    assert!(!is_valid_plugin_id("BadCaps"));
    assert!(!is_valid_plugin_id(&"a".repeat(33)));
}
#[test]
fn is_valid_plugin_version_ok() {
    assert!(is_valid_plugin_version("0.1.0"));
    assert!(is_valid_plugin_version("1.2.3"));
    assert!(!is_valid_plugin_version("0.1"));
    assert!(!is_valid_plugin_version("01.0.0"));
}
#[test]
fn sanitize_plugin_path_ok() {
    assert!(sanitize_plugin_path("index.js").is_ok());
    assert!(sanitize_plugin_path("src/main.wasm").is_ok());
    assert!(sanitize_plugin_path("../escape.js").is_err());
    assert!(sanitize_plugin_path("/abs.js").is_err());
    assert!(sanitize_plugin_path("evil.txt").is_err());
}
#[test]
fn validate_manifest_ok() {
    let m = json!({"name":"example-translate","version":"0.1.0","entry":"index.js","capabilities":["notify","clipboard"],"bridgeVersion":"1"});
    assert!(validate_plugin_manifest(&m).is_ok());
}
#[test]
fn validate_manifest_bad_caps() {
    let m = json!({"name":"bad","version":"0.1.0","entry":"index.js","capabilities":["evil"]});
    assert!(validate_plugin_manifest(&m).is_err());
}
#[test]
fn validate_manifest_traversal() {
    let m = json!({"name":"bad","version":"0.1.0","entry":"../../etc/passwd","capabilities":["notify"]});
    assert!(validate_plugin_manifest(&m).is_err());
}
#[test]
fn validate_load_ok() {
    let p = json!({"pluginId":"example-translate"});
    assert!(validate_plugin_load_payload(&p).is_ok());
}
#[test]
fn validate_load_bad() {
    let p = json!({"pluginId":"bad/cap"});
    assert!(validate_plugin_load_payload(&p).is_err());
}
#[test]
fn can_access() {
    let caps = vec!["notify".to_string(), "clipboard".to_string()];
    assert!(can_plugin_access(&caps, "notify"));
    assert!(!can_plugin_access(&caps, "storage"));
}
#[test]
fn allowed_caps_contains() {
    assert!(ALLOWED_PLUGIN_CAPS.contains(&"notify"));
    assert!(ALLOWED_PLUGIN_CAPS.contains(&"clipboard"));
    assert!(ALLOWED_PLUGIN_CAPS.contains(&"storage"));
    assert!(ALLOWED_PLUGIN_CAPS.contains(&"ai.summarize"));
}
