use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use bridge_core::{BridgeMessage, MessageType};
use tracing::{info, warn};
use std::sync::Arc;
use tokio::sync::broadcast;
use crate::pairing::PairingManager;
use crate::services;

pub async fn run_server(port: u16, pairing: PairingManager) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    info!("WS listening on {addr}");
    let (tx, _rx) = broadcast::channel::<String>(200);
    let pairing = Arc::new(pairing);

    let tx_hb = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3));
        loop {
            interval.tick().await;
            let status = services::status::collect_status();
            let msg = BridgeMessage::new(MessageType::StatusPush, serde_json::to_value(status).unwrap());
            let _ = tx_hb.send(msg.to_json());
        }
    });

    info!("QR payload: {}", pairing.qr_payload());
    info!("Host: {} FP: {} SAS: {}", pairing.host(), pairing.fingerprint(), pairing.sas_preview());

    loop {
        let (stream, peer) = listener.accept().await?;
        let tx = tx.clone();
        let pairing = pairing.clone();
        tokio::spawn(handle_conn(stream, peer, tx, pairing));
    }
}

async fn handle_conn(stream: TcpStream, peer: std::net::SocketAddr, tx: broadcast::Sender<String>, pairing: Arc<PairingManager>) {
    // Peek first bytes to detect HTTP GET /qr vs WebSocket
    // For simplicity, try HTTP detection via raw read with timeout
    // We delegate to WS handler; HTTP fallback handled via simple branch before WS handshake
    // Check if stream is HTTP by peeking
    use tokio::io::{AsyncReadExt};
    let mut buf = [0u8; 512];
    let n = {
        let mut stream_ref = &stream;
        // peek without consuming: use try_peek via &TcpStream peek
        // TcpStream::peek is async
        match tokio::time::timeout(tokio::time::Duration::from_millis(200), stream.peek(&mut buf)).await {
            Ok(Ok(n)) => n,
            _ => 0,
        }
    };
    if n > 0 && buf[0..n].starts_with(b"GET ") {
        let req = String::from_utf8_lossy(&buf[0..n]);
        // Only treat as HTTP if it's NOT a WebSocket upgrade
        let is_ws = req.to_ascii_lowercase().contains("upgrade: websocket");
        if !is_ws {
            let body = if req.contains("GET /qr") || req.contains("GET / ") {
                serde_json::json!({
                    "qr": pairing.qr_payload(),
                    "host": pairing.host(),
                    "port": 8443,
                    "fp": pairing.fingerprint(),
                    "sas": pairing.sas_preview(),
                    "device_id": pairing.device_id()
                }).to_string()
            } else if req.contains("GET /status") {
                serde_json::to_string(&services::status::collect_status()).unwrap()
            } else {
                serde_json::json!({"ok": true}).to_string()
            };
            let resp = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, OPTIONS\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
            let _ = stream.into_std().map(|s| {
                use std::io::Write;
                let mut s = s;
                let _ = s.write_all(resp.as_bytes());
            });
            return;
        }
    }

    let ws = match accept_async(stream).await {
        Ok(w) => w,
        Err(e) => { warn!("ws handshake fail {peer}: {e}"); return; }
    };
    info!("client {peer} connected");
    let (mut write, mut read) = ws.split();
    let mut rx = tx.subscribe();

    // Send initial pairing info on connect
    let init = BridgeMessage::new(MessageType::PairingTrusted, serde_json::json!({
        "qr": pairing.qr_payload(),
        "host": pairing.host(),
        "fp": pairing.fingerprint(),
        "sas": pairing.sas_preview()
    }));
    let _ = write.send(Message::Text(init.to_json().into())).await;

    loop {
        tokio::select! {
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(txt))) => {
                        if let Ok(bmsg) = BridgeMessage::from_json(&txt) {
                            let resp = services::router::route(bmsg, pairing.clone()).await;
                            if let Some(r) = resp {
                                let json = r.to_json();
                                // Broadcast clipboard/notify/control to all clients, else direct
                                match r.typ {
                                    MessageType::ClipboardSync | MessageType::NotifyNew | MessageType::NotifyAction
                                    | MessageType::InputEvent | MessageType::InputAck
                                    | MessageType::DisplayInfo | MessageType::DisplayFrame
                                    | MessageType::ControlStart | MessageType::ControlStop
                                    | MessageType::StorageSync | MessageType::StorageConflict => {
                                        let _ = tx.send(json);
                                    },
                                    _ => {
                                        if write.send(Message::Text(json.into())).await.is_err() { break; }
                                    }
                                }
                            }
                        } else {
                            warn!("invalid json from {peer}: {}", &txt[..txt.len().min(200)]);
                        }
                    },
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(e)) => { warn!("ws err {e}"); break; },
                    _ => {}
                }
            },
            Ok(broadcast_msg) = rx.recv() => {
                if write.send(Message::Text(broadcast_msg.into())).await.is_err() { break; }
            }
        }
    }
    info!("client {peer} disconnected");
}
