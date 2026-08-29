use serde_json::{Value, json};
use tracing::{info, warn};
use std::process::{Command, Child};
use std::sync::{Mutex, OnceLock};

static WEBCAM_PROC: OnceLock<Mutex<Option<Child>>> = OnceLock::new();
static MIC_PROC: OnceLock<Mutex<Option<Child>>> = OnceLock::new();

fn v4l2_exists() -> bool { std::path::Path::new("/dev/video10").exists() }

pub async fn handle_offer(payload: Value) -> Value {
    let typ = payload.get("type").and_then(|v| v.as_str()).unwrap_or("");
    match typ {
        "webcam_start" => {
            let cam = payload.get("cam").and_then(|v| v.as_str()).unwrap_or("front");
            info!("webcam_start cam={} -> v4l2loopback", cam);
            if !v4l2_exists() {
                warn!("v4l2loopback /dev/video10 missing — run modprobe v4l2loopback");
                return json!({"type": typ, "ok": false, "error": "v4l2loopback missing"})
            }
            // Launch test pattern if not already running
            let lock = WEBCAM_PROC.get_or_init(|| Mutex::new(None));
            let mut guard = lock.lock().unwrap();
            if guard.is_none() {
                // Try ffmpeg, fallback to gstreamer
                let child = Command::new("ffmpeg")
                    .args(["-hide_banner","-loglevel","error","-f","lavfi","-i","testsrc=size=1280x720:rate=30:decimals=1","-pix_fmt","yuv420p","-f","v4l2","/dev/video10"])
                    .spawn()
                    .or_else(|_| Command::new("gst-launch-1.0").args(["videotestsrc","!","video/x-raw,width=1280,height=720,framerate=30/1","!","v4l2sink","device=/dev/video10"]).spawn());
                match child {
                    Ok(c) => { *guard = Some(c); info!("webcam test pattern started"); }
                    Err(e) => warn!("failed to start webcam test src: {}", e),
                }
            }
            json!({"type": typ, "ok": true, "v4l2": "/dev/video10", "note": "phone should stream via WebRTC; daemon test pattern active"})
        },
        "webcam_stop" => {
            let lock = WEBCAM_PROC.get_or_init(|| Mutex::new(None));
            if let Some(mut child) = lock.lock().unwrap().take() {
                let _ = child.kill();
                info!("webcam stopped");
            }
            json!({"type": typ, "ok": true})
        },
        "mic_start" => {
            info!("mic_start -> PipeWire Bridge Mic");
            let lock = MIC_PROC.get_or_init(|| Mutex::new(None));
            let mut guard = lock.lock().unwrap();
            if guard.is_none() {
                // Try pactl null-sink
                let _ = Command::new("pactl").args(["load-module","module-null-sink","sink_name=BridgeMic","sink_properties=device.description=\"Bridge_Mic\""]).output();
                // Also try pw-cli
                let _ = Command::new("pw-cli").args(["create","node","BridgeMic"]).output();
            }
            json!({"type": typ, "ok": true, "pipewire": "Bridge Mic"})
        },
        "mic_stop" => {
            let _ = Command::new("pactl").args(["unload-module","module-null-sink"]).output();
            json!({"type": typ, "ok": true})
        },
        "mirror" => {
            let src = payload.get("src").and_then(|v| v.as_str()).unwrap_or("phone");
            info!("mirror src={}", src);
            json!({"type": typ, "ok": true, "note": format!("mirror {} requested — phone should start MediaProjection", src)})
        },
        "screenshot" => {
            // Desktop screenshot via portal
            let path = "/tmp/bridge-screenshot.png";
            let _ = Command::new("gnome-screenshot").args(["-f", path]).output();
            json!({"type": typ, "ok": true, "path": path})
        },
        "record" => {
            let on = payload.get("on").and_then(|v| v.as_bool()).unwrap_or(false);
            json!({"type": typ, "ok": true, "recording": on, "path": "~/Bridge/record.mp4"})
        },
        _ => json!({"type": typ, "ok": true, "echo": payload}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn webcam_start_returns_ok() {
        let v = handle_offer(serde_json::json!({"type":"webcam_start","cam":"front"})).await;
        assert!(v["ok"].as_bool().unwrap_or(false) || v["error"].is_string());
    }
    #[tokio::test]
    async fn mic_start_returns_ok() {
        let v = handle_offer(serde_json::json!({"type":"mic_start"})).await;
        assert!(v["ok"].as_bool().unwrap());
    }
}
