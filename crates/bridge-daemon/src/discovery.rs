use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::collections::HashMap;

pub struct Mdns {
    daemon: ServiceDaemon,
    service_id: String,
    port: u16,
}

impl Mdns {
    pub fn new(device_id: &str, port: u16) -> anyhow::Result<Self> {
        let daemon = ServiceDaemon::new()?;
        Ok(Self { daemon, service_id: device_id.to_string(), port })
    }
    pub fn register(&self) {
        let host = gethostname::gethostname().to_string_lossy().to_string();
        let mut props = HashMap::new();
        props.insert("id".to_string(), self.service_id.clone());
        props.insert("ver".to_string(), "1".to_string());
        let name = format!("bridge-{}", &self.service_id[..8.min(self.service_id.len())]);
        let svc = ServiceInfo::new(
            "_bridge._tcp.local.",
            &name,
            &format!("{}.local.", host),
            "",
            self.port,
            props,
        ).unwrap().enable_addr_auto();
        if let Err(e) = self.daemon.register(svc) {
            tracing::warn!("mdns register failed: {e}");
        } else {
            tracing::info!("mDNS registered _bridge._tcp on port {}", self.port);
        }
    }
}
