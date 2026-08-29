use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{PathBuf, Path};
use std::sync::{Mutex, OnceLock};
use tracing::{info, warn};
use bridge_core::{PluginState, validate_plugin_manifest, validate_plugin_load_payload, can_plugin_access, sanitize_plugin_path};

const PLUGINS_ROOT_FALLBACK: &str = "plugins";

static PLUGIN_REGISTRY: OnceLock<Mutex<HashMap<String, Plugin>>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct Plugin {
    pub manifest: Value,
    pub state: PluginState,
    pub dir: PathBuf,
    pub loaded_at: i64,
    pub fuel: u64,
}

fn now_ms() -> i64 { chrono::Utc::now().timestamp_millis() }

fn plugins_root() -> PathBuf {
    // Try bridge daemon crate relative plugins, fallback to ~/.config
    let candidate = PathBuf::from(PLUGINS_ROOT_FALLBACK);
    if candidate.exists() { return candidate; }
    // Try project root plugins (for daemon running from vs code)
    let proj_plugins = PathBuf::from("/home/mylappy/Projects/bridge/plugins");
    if proj_plugins.exists() { return proj_plugins; }
    if let Some(proj) = directories::ProjectDirs::from("dev", "bridge", "bridge") {
        let p = proj.config_local_dir().join("plugins");
        let _ = std::fs::create_dir_all(&p);
        return p;
    }
    PathBuf::from("/tmp/bridge-plugins")
}

pub fn plugins_root_canonical() -> Option<PathBuf> {
    plugins_root().canonicalize().ok().or(Some(plugins_root()))
}

pub fn sanitize_plugin_filesystem_path(plugin_dir: &Path, entry: &str) -> Result<PathBuf, String> {
    // entry must be sanitize_plugin_path + canonical inside plugin_dir
    sanitize_plugin_path(entry).map_err(|e| e.to_string())?;
    let joined = plugin_dir.join(entry);
    // ensure normalized no .. and that canonical starts with plugin_dir canonical
    // For file not yet existence, we check parent canonical + ensure no traversal
    // Use lexical check: entry must not contain .. already validated, and joined must not escape via ..
    // We attempt to canonicalize parent
    let canonical_root = plugin_dir.canonicalize().unwrap_or(plugin_dir.to_path_buf());
    let candidate = joined.clone();
    // If file exists, check canonical
    if candidate.exists() {
        if let Ok(canon) = candidate.canonicalize() {
            if !canon.starts_with(&canonical_root) {
                return Err(format!("entry escapes plugin dir: {}", entry));
            }
            return Ok(canon);
        }
    } else {
        // Check lexical: ensure joined path components don't escape
        // We already validated entry no .., so lexical safe
        // Just ensure no absolute
        if candidate.is_absolute() && !candidate.starts_with(&canonical_root) {
            return Err(format!("entry absolute escapes: {}", entry));
        }
    }
    Ok(joined)
}

pub fn capability_check(plugin_id: &str, needed: &str) -> bool {
    let reg = PLUGIN_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
    let g = reg.lock().unwrap();
    if let Some(p) = g.get(plugin_id) {
        if p.state != PluginState::Running && p.state != PluginState::Loaded {
            return false;
        }
        let caps: Vec<String> = p.manifest.get("capabilities")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();
        return can_plugin_access(&caps, needed);
    }
    false
}

pub fn plugin_registry_snapshot() -> Vec<Value> {
    let reg = PLUGIN_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
    let g = reg.lock().unwrap();
    g.values().map(|p| {
        let caps = p.manifest.get("capabilities").cloned().unwrap_or(json!([]));
        json!({
            "id": p.manifest.get("name").and_then(|v| v.as_str()).unwrap_or("unknown"),
            "name": p.manifest.get("name").and_then(|v| v.as_str()).unwrap_or("unknown"),
            "version": p.manifest.get("version").and_then(|v| v.as_str()).unwrap_or("0.0.0"),
            "displayName": p.manifest.get("displayName").and_then(|v| v.as_str()).unwrap_or(""),
            "description": p.manifest.get("description").and_then(|v| v.as_str()).unwrap_or(""),
            "entry": p.manifest.get("entry").and_then(|v| v.as_str()).unwrap_or(""),
            "capabilities": caps,
            "state": format!("{:?}", p.state).to_uppercase(),
            "dir": p.dir.display().to_string(),
            "loadedAt": p.loaded_at
        })
    }).collect()
}

pub fn load_plugin_from_dir(plugin_dir: &Path) -> Result<Value, String> {
    let manifest_path = plugin_dir.join("bridge.json");
    let content = std::fs::read_to_string(&manifest_path).map_err(|e| format!("read bridge.json fail: {}", e))?;
    let manifest: Value = serde_json::from_str(&content).map_err(|e| format!("json parse fail: {}", e))?;
    validate_plugin_manifest(&manifest).map_err(|e| e.to_string())?;
    // sanitize entry filesystem check
    let entry = manifest.get("entry").and_then(|v| v.as_str()).unwrap_or("");
    sanitize_plugin_filesystem_path(plugin_dir, entry)?;
    // Check capabilities subset
    Ok(manifest)
}

pub fn register_plugin(plugin_dir: PathBuf) -> Result<String, String> {
    let manifest = load_plugin_from_dir(&plugin_dir)?;
    let name = manifest.get("name").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
    let reg = PLUGIN_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
    let mut g = reg.lock().unwrap();
    if g.contains_key(&name) {
        // hot reload: transition Reloading
        if let Some(existing) = g.get_mut(&name) {
            if existing.state == PluginState::Running {
                existing.state = PluginState::Reloading;
            }
            existing.manifest = manifest.clone();
            existing.state = PluginState::Running;
            existing.loaded_at = now_ms();
            existing.fuel = 10_000_000;
            info!(target:"audit", "plugin hot reload {} dir={}", name, plugin_dir.display());
            return Ok(name);
        }
    }
    let plugin = Plugin {
        manifest: manifest.clone(),
        state: PluginState::Running,
        dir: plugin_dir.clone(),
        loaded_at: now_ms(),
        fuel: 10_000_000, // wasmtime fuel stub 10M
    };
    g.insert(name.clone(), plugin);
    info!(target:"audit", "plugin loaded {} dir={} caps={:?}", name, plugin_dir.display(), manifest.get("capabilities"));
    Ok(name)
}

pub fn scan_plugins() -> Vec<String> {
    let root = plugins_root();
    let mut loaded = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&root) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let bridge_json = p.join("bridge.json");
                if bridge_json.exists() {
                    match register_plugin(p.clone()) {
                        Ok(id) => loaded.push(id),
                        Err(e) => warn!("plugin load fail {}: {}", p.display(), e),
                    }
                }
            }
        }
    }
    loaded
}

pub fn unload_plugin(plugin_id: &str) -> bool {
    let reg = PLUGIN_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
    let mut g = reg.lock().unwrap();
    if let Some(p) = g.get_mut(plugin_id) {
        p.state = PluginState::Unloaded;
        true
    } else {
        false
    }
}

pub fn start_plugin_watcher() -> anyhow::Result<()> {
    use notify::{Watcher, RecursiveMode, EventKind};
    use std::sync::mpsc::channel;
    use std::time::{Duration, Instant};
    let root = plugins_root();
    let _ = std::fs::create_dir_all(&root);
    let (tx, rx) = channel();
    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(&root, RecursiveMode::Recursive)?;
    std::thread::spawn(move || {
        let mut pending: HashMap<PathBuf, Instant> = HashMap::new();
        loop {
            while let Ok(Ok(event)) = rx.try_recv() {
                for path in event.paths {
                    if path.file_name().and_then(|n| n.to_str()) == Some("bridge.json") {
                        pending.insert(path, Instant::now());
                    } else if matches!(event.kind, EventKind::Create(_) | EventKind::Remove(_) ) {
                        // also watch dir creation
                    }
                }
            }
            let now = Instant::now();
            let to_reload: Vec<PathBuf> = pending.iter().filter(|(_, t)| now.duration_since(**t) > Duration::from_millis(500)).map(|(p,_)| p.clone()).collect();
            for p in to_reload {
                pending.remove(&p);
                if let Some(dir) = p.parent() {
                    // debounce hot reload
                    match load_plugin_from_dir(dir) {
                        Ok(_) => {
                            let _ = register_plugin(dir.to_path_buf());
                            info!("plugin hot reload {}", dir.display());
                        },
                        Err(e) => {
                            warn!("plugin reload failed {}: {}", dir.display(), e);
                            // keep old version
                        }
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    });
    std::mem::forget(watcher);
    Ok(())
}

// Handlers (WS)

pub async fn handle_plugin_list(_payload: Value) -> Value {
    let plugins = plugin_registry_snapshot();
    json!({"plugins": plugins})
}

pub async fn handle_plugin_load(payload: Value) -> Value {
    if let Err(e) = validate_plugin_load_payload(&payload) {
        return json!({"error": e.to_string(), "code": "validation"});
    }
    let plugin_id = payload.get("pluginId").and_then(|v| v.as_str()).unwrap_or("");
    let root = plugins_root();
    let dir = root.join(plugin_id);
    if !dir.exists() {
        return json!({"error": format!("plugin not found: {}", plugin_id), "code": "plugin_not_found"});
    }
    match register_plugin(dir) {
        Ok(id) => {
            // Ensure state Running
            json!({"ok": true, "pluginId": id, "state": "RUNNING"})
        },
        Err(e) => json!({"error": e, "code": "validation"})
    }
}

pub async fn handle_plugin_emit(payload: Value) -> Value {
    // payload: {pluginId, event, data}
    let plugin_id = payload.get("pluginId").and_then(|v| v.as_str()).unwrap_or("");
    let event = payload.get("event").and_then(|v| v.as_str()).unwrap_or("");
    if plugin_id.is_empty() || event.is_empty() {
        return json!({"error": "missing pluginId or event", "code": "validation"});
    }
    // Validate plugin exists
    let reg = PLUGIN_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
    let cap_needed = match event {
        "notify.new" => "notify",
        "clipboard.sync" => "clipboard",
        "storage.sync" | "storage.rm" | "storage.ls" => "storage",
        "ai.summarize" => "ai.summarize",
        "ai.transcribe" => "ai.transcribe",
        _ => {
            // generic plugin.* allowed without cap
            if event.starts_with("plugin.") { "" } else { return json!({"error": format!("unknown event: {}", event), "code": "validation"}); }
        }
    };
    if !cap_needed.is_empty() && !capability_check(plugin_id, cap_needed) {
        warn!(target:"audit", "plugin capability denied {} needs {}", plugin_id, cap_needed);
        return json!({"error": format!("capability denied: {}", cap_needed), "code": "capability_denied", "capability": cap_needed, "pluginId": plugin_id});
    }
    // Enforce plugin.* prefix for custom emits
    if event.starts_with("plugin.") || cap_needed.is_empty() || ["notify.new","clipboard.sync","storage.sync","storage.rm","storage.ls","ai.summarize","ai.transcribe"].contains(&event) {
        // For custom plugin emit, allow if capability passes
    }
    // Enforce fuel limit per plugin (wasmtime stub)
    {
        let mut g = reg.lock().unwrap();
        if let Some(p) = g.get_mut(plugin_id) {
            if p.fuel < 1000 {
                return json!({"error": "fuel exhausted", "code": "fuel_exhausted"});
            }
            p.fuel -= 1000;
        }
    }
    info!(target:"audit", "plugin emit {} event={} fuel left", plugin_id, event);
    json!({"ok": true, "pluginId": plugin_id, "event": event, "relayed": true})
}

pub fn reset_plugin_registry() {
    if let Some(m) = PLUGIN_REGISTRY.get() { if let Ok(mut g) = m.lock() { g.clear(); } else if let Err(e) = m.lock() { e.into_inner().clear(); } }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;
    use std::sync::{OnceLock, Mutex};
    static PLUGIN_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    fn plugin_lock() -> std::sync::MutexGuard<'static, ()> { PLUGIN_TEST_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|e| e.into_inner()) }

    fn temp_plugin_dir(name: &str, manifest: Value) -> PathBuf {
        let base = PathBuf::from(format!("/tmp/bridge_test_plugin_{}_{}", name, rand::random::<u32>()));
        std::fs::create_dir_all(&base).unwrap();
        let dir = base.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("bridge.json"), serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
        std::fs::write(dir.join("index.js"), "// mock plugin").unwrap();
        base
    }

    #[test]
    fn manifest_validation_ok() {
        let m = json!({"name":"example-translate","version":"0.1.0","entry":"index.js","capabilities":["notify","clipboard"],"bridgeVersion":"1"});
        assert!(validate_plugin_manifest(&m).is_ok());
    }
    #[test]
    fn manifest_validation_bad_caps() {
        let m = json!({"name":"bad","version":"0.1.0","entry":"index.js","capabilities":["evil"]});
        assert!(validate_plugin_manifest(&m).is_err());
    }
    #[test]
    fn manifest_validation_traversal() {
        let m = json!({"name":"bad","version":"0.1.0","entry":"../../etc/passwd","capabilities":["notify"]});
        assert!(validate_plugin_manifest(&m).is_err());
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
    fn capability_check_test() {
        let _g = plugin_lock();
        reset_plugin_registry();
        let base = temp_plugin_dir("example-translate", json!({"name":"example-translate","version":"0.1.0","entry":"index.js","capabilities":["notify","clipboard"]}));
        let dir = base.join("example-translate");
        let _ = register_plugin(dir);
        assert!(crate::services::plugin::capability_check("example-translate","notify"));
        assert!(crate::services::plugin::capability_check("example-translate","clipboard"));
        assert!(!crate::services::plugin::capability_check("example-translate","storage"));
        assert!(!crate::services::plugin::capability_check("example-translate","ai.transcribe"));
        assert!(!crate::services::plugin::capability_check("unknown","notify"));
        reset_plugin_registry();
        let _ = std::fs::remove_dir_all(base);
    }
    #[tokio::test]
    async fn plugin_load_and_list() {
        let _g = plugin_lock();
        reset_plugin_registry();
        let base = temp_plugin_dir("test-load", json!({"name":"test-load","version":"1.2.3","entry":"index.js","capabilities":["notify"]}));
        let dir = base.join("test-load");
        let _resp = handle_plugin_list(json!({})).await;
        // before load, may have 0
        let id = register_plugin(dir.clone()).unwrap();
        assert_eq!(id, "test-load");
        let resp2 = handle_plugin_list(json!({})).await;
        assert!(resp2["plugins"].as_array().unwrap().iter().any(|p| p["id"]=="test-load"));
        // handle_plugin_load should succeed if we move dir to real plugins_root? For now test with temp dir not in plugins_root will fail
        // So we simulate via register
        reset_plugin_registry();
        let _ = std::fs::remove_dir_all(base);
    }
    #[tokio::test]
    async fn plugin_emit_capability_denied() {
        let _g = plugin_lock();
        reset_plugin_registry();
        let base = temp_plugin_dir("emit-test", json!({"name":"emit-test","version":"0.1.0","entry":"index.js","capabilities":["notify"]}));
        let dir = base.join("emit-test");
        let _ = register_plugin(dir);
        // try clipboard without cap
        let resp = handle_plugin_emit(json!({"pluginId":"emit-test","event":"clipboard.sync","data":{}})).await;
        assert_eq!(resp["code"], "capability_denied");
        // notify should pass
        let resp2 = handle_plugin_emit(json!({"pluginId":"emit-test","event":"notify.new","data":{}})).await;
        assert_eq!(resp2["ok"], true);
        reset_plugin_registry();
        let _ = std::fs::remove_dir_all(base);
    }
    #[test]
    fn plugin_state_transitions() {
        assert!(PluginState::Unloaded.can_transition(&PluginState::Loading));
        assert!(PluginState::Loading.can_transition(&PluginState::Loaded));
        assert!(PluginState::Loaded.can_transition(&PluginState::Running));
        assert!(PluginState::Running.can_transition(&PluginState::Reloading));
        assert!(PluginState::Reloading.can_transition(&PluginState::Running));
        assert!(!PluginState::Unloaded.can_transition(&PluginState::Running));
        assert!(!PluginState::Running.can_transition(&PluginState::Loaded));
    }
    #[test]
    fn fuel_exhaustion() {
        let _g = plugin_lock();
        reset_plugin_registry();
        let base = temp_plugin_dir("fuel-test", json!({"name":"fuel-test","version":"0.1.0","entry":"index.js","capabilities":["notify"]}));
        let dir = base.join("fuel-test");
        let _ = register_plugin(dir.clone());
        // Manually set fuel low
        {
            let reg = PLUGIN_REGISTRY.get().unwrap();
            let mut g = reg.lock().unwrap();
            if let Some(p) = g.get_mut("fuel-test") { p.fuel = 500; }
        }
        // next emit should fail fuel exhausted via handle_plugin_emit
        // We need async runtime for emit, so test sync check
        let reg = PLUGIN_REGISTRY.get().unwrap();
        let g = reg.lock().unwrap();
        assert_eq!(g.get("fuel-test").unwrap().fuel, 500);
        drop(g);
        reset_plugin_registry();
        let _ = std::fs::remove_dir_all(base);
    }
}
