use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Battery { pub pct: u8, pub charging: bool, pub tempC: f32 }
#[derive(Serialize, Deserialize, Clone)]
pub struct Ram { pub availMb: u64, pub totalMb: u64 }
#[derive(Serialize, Deserialize, Clone)]
pub struct Storage { pub freeGb: f32, pub totalGb: f32 }
#[derive(Serialize, Deserialize, Clone)]
pub struct Signal { pub dbm: i32, pub bars: u8 }
#[derive(Serialize, Deserialize, Clone)]
pub struct DeviceStatus {
    pub battery: Battery,
    pub ram: Ram,
    pub storage: Storage,
    pub signal: Signal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

pub fn collect_status() -> DeviceStatus {
    let (avail, total) = read_mem();
    DeviceStatus {
        battery: Battery { pct: 77, charging: false, tempC: 30.0 },
        ram: Ram { availMb: avail, totalMb: total },
        storage: Storage { freeGb: 120.5, totalGb: 512.0 },
        signal: Signal { dbm: -67, bars: 4 },
        source: Some("daemon".into()),
    }
}

fn read_mem() -> (u64,u64) {
    let s = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut avail=0; let mut total=0;
    for line in s.lines() {
        if line.starts_with("MemAvailable:") { avail = line.split_whitespace().nth(1).and_then(|v| v.parse().ok()).unwrap_or(0)/1024; }
        if line.starts_with("MemTotal:") { total = line.split_whitespace().nth(1).and_then(|v| v.parse().ok()).unwrap_or(0)/1024; }
    }
    (avail, total)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn status_collects() {
        let s = collect_status();
        assert!(s.ram.totalMb > 0);
    }
}
