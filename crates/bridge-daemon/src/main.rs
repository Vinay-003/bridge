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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    let args = Args::parse();
    info!("Bridge daemon starting — ws:{} quic:{}", args.port, args.quic_port);

    // init pairing (generate keypair + fingerprint)
    let pairing = pairing::PairingManager::new(args.port);
    info!("Device ID: {} fp: {} sas: {}", pairing.device_id(), pairing.fingerprint(), pairing.sas_preview());

    // mdns advertise
    let mdns = discovery::Mdns::new(&pairing.device_id(), args.port)?;
    mdns.register();

    // transport WS server
    transport::run_server(args.port, pairing).await?;
    Ok(())
}
