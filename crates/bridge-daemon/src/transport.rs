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
    let (tx, _rx) = broadcast::channel::<String>(100);
    let pairing = Arc::new(pairing);

    // spawn heartbeat broadcaster
    let tx_hb = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            let status = services::status::collect_status();
            let msg = BridgeMessage::new(MessageType::StatusPush, serde_json::to_value(status).unwrap());
            let _ = tx_hb.send(msg.to_json());
        }
    });

    // spawn pairing QR log
    info!("QR payload: {}", pairing.qr_payload());

    loop {
        let (stream, peer) = listener.accept().await?;
        let tx = tx.clone();
        let pairing = pairing.clone();
        tokio::spawn(handle_conn(stream, peer, tx, pairing));
    }
}

async fn handle_conn(stream: TcpStream, peer: std::net::SocketAddr, tx: broadcast::Sender<String>, _pairing: Arc<PairingManager>) {
    let ws = match accept_async(stream).await {
        Ok(w) => w,
        Err(e) => { warn!("ws handshake fail {peer}: {e}"); return; }
    };
    info!("client {peer} connected");
    let (mut write, mut read) = ws.split();
    let mut rx = tx.subscribe();

    loop {
        tokio::select! {
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(txt))) => {
                        if let Ok(bmsg) = BridgeMessage::from_json(&txt) {
                            let resp = services::router::route(bmsg).await;
                            if let Some(r) = resp {
                                if write.send(Message::Text(r.to_json().into())).await.is_err() { break; }
                            }
                        } else {
                            warn!("invalid json from {peer}");
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
