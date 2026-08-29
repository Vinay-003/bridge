use serde_json::json;

fn is_valid_plugin_id(s:&str)->bool {
    if s.len()<3 || s.len()>32 { return false; }
    s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c=='-'||c=='_')
}
fn sanitize_entry(s:&str)->bool {
    if s.is_empty()||s.len()>256 { return false; }
    if s.starts_with('/') { return false; }
    if s.contains("..") { return false; }
    s.ends_with(".js")||s.ends_with(".wasm")
}
#[test]
fn plugin_id_validation() {
    assert!(is_valid_plugin_id("example-translate"));
    assert!(!is_valid_plugin_id("ab"));
    assert!(!is_valid_plugin_id("BadCaps"));
}
#[test]
fn sanitize_entry_ok() {
    assert!(sanitize_entry("index.js"));
    assert!(sanitize_entry("src/main.wasm"));
    assert!(!sanitize_entry("../escape.js"));
    assert!(!sanitize_entry("/abs.js"));
    assert!(!sanitize_entry("evil.txt"));
}
#[test]
fn manifest_contract() {
    let ok=json!({"name":"example-translate","version":"0.1.0","entry":"index.js","capabilities":["notify","clipboard"]});
    assert!(is_valid_plugin_id(ok["name"].as_str().unwrap()));
    assert!(sanitize_entry(ok["entry"].as_str().unwrap()));
    let bad=json!({"name":"bad","version":"0.1.0","entry":"index.js","capabilities":["evil"]});
    let caps = bad["capabilities"].as_array().unwrap();
    assert!(caps.iter().any(|c| c=="evil"));
    // our validation would reject evil cap
    assert!(!caps.iter().all(|c| ["notify","clipboard","storage","ai.summarize","ai.transcribe"].contains(&c.as_str().unwrap())));
}
#[test]
fn capability_check() {
    let caps = vec!["notify".to_string(),"clipboard".to_string()];
    assert!(caps.contains(&"notify".to_string()));
    assert!(!caps.contains(&"storage".to_string()));
}
#[test]
fn plugin_message_contract() {
    let cases = [
        ("plugin.list", json!({})),
        ("plugin.load", json!({"pluginId":"example-translate"})),
        ("plugin.emit", json!({"pluginId":"example-translate","event":"notify.new"})),
    ];
    for (typ,payload) in cases {
        let msg=json!({"v":1,"id":"test","type":typ,"ts":0,"nonce":"a","payload":payload});
        assert!(serde_json::to_string(&msg).unwrap().contains(typ));
    }
}
#[test]
fn hot_reload_debounce_simulation() {
    let mut pending: std::collections::HashMap<String,i64>=std::collections::HashMap::new();
    pending.insert("/tmp/plugins/a/bridge.json".into(), 0);
    pending.insert("/tmp/plugins/b/bridge.json".into(), 100);
    // debounce 500ms: at time 400, only first pending >500? none
    let now=400;
    let to_reload: Vec<_> = pending.iter().filter(|(_,t)| now - **t > 500).collect();
    assert_eq!(to_reload.len(),0);
    let now=600;
    let to_reload2: Vec<_> = pending.iter().filter(|(_,t)| now - **t > 500).collect();
    assert_eq!(to_reload2.len(),1);
}
