mod discovery;
mod pairing;
mod services;
mod transport;

use tracing::{info, warn};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name="bridge-daemon")]
struct Args {
    #[arg(long, default_value_t = 8443)]
    port: u16,
    #[arg(long, default_value_t = 8444)]
    quic_port: u16,
    /// Enable global relay via https://relay.bridge.dev/v1/announce with STUN hole punching fallback
    #[arg(long, default_value_t = false)]
    relay: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    let args = Args::parse();
    info!("Bridge daemon starting — ws:{} quic:{} relay:{}", args.port, args.quic_port, args.relay);

    // init pairing (generate keypair + fingerprint)
    let pairing = pairing::PairingManager::new(args.port);
    info!("Device ID: {} fp: {} sas: {}", pairing.device_id(), pairing.fingerprint(), pairing.sas_preview());
    if args.relay {
        info!("Relay enabled: {} + STUN {}", services::relay::relay_announce_url(), services::relay::STUN_SERVER);
    }

    // mdns advertise
    let mdns = discovery::Mdns::new(&pairing.device_id(), args.port)?;
    mdns.register();

    // plugin scan + watcher (Phase 7) and mesh manifest init
    let _ = services::plugin::scan_plugins();
    let _ = services::plugin::start_plugin_watcher();
    // storage notify watcher already handled via storage.rs but ensure init
    let _ = services::storage::start_notify_watcher();

    // relay announce loop if --relay enabled
    if args.relay {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(45));
            loop {
                interval.tick().await;
                // Try STUN hole punch first, fallback to QUIC relay
                match services::relay::try_stun_hole_punch(services::relay::STUN_SERVER) {
                    Ok(addr) => {
                        info!("Relay keepalive STUN mapped {}", addr);
                        let _ = services::relay::try_quic_relay_connect(services::relay::relay_announce_url());
                    },
                    Err(e) => {
                        warn!("STUN failed {}, fallback to QUIC relay: {}", e, services::relay::relay_announce_url());
                        let _ = services::relay::try_quic_relay_connect(services::relay::relay_announce_url());
                    }
                }
            }
        });
    }

    // transport WS server
    transport::run_server(args.port, pairing).await?;
    Ok(())
}
