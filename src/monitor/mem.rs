use crate::Monitorable;
use std::collections::HashMap;

/// Parsed subset of /proc/meminfo. Fields are in kB.
#[derive(Debug, Clone, Default)]
pub struct MemInfo {
    pub mem_total_kb: u64,
    pub mem_free_kb: u64,
    pub mem_available_kb: Option<u64>,
    pub buffers_kb: Option<u64>,
    pub cached_kb: Option<u64>,
    pub swap_total_kb: Option<u64>,
    pub swap_free_kb: Option<u64>,
    /// any other numeric fields captured from the file
    pub other: HashMap<String, u64>,
}

impl MemInfo {
    /// Total memory in bytes
    pub fn total_bytes(&self) -> u64 {
        self.mem_total_kb * 1024
    }

    /// Free memory in bytes (from MemFree)
    pub fn free_bytes(&self) -> u64 {
        self.mem_free_kb * 1024
    }

    /// Available memory in bytes (if present)
    pub fn available_bytes(&self) -> Option<u64> {
        self.mem_available_kb.map(|v| v * 1024)
    }

    /// Used memory in bytes, computed as total - available (preferred) or total - free
    pub fn used_bytes(&self) -> u64 {
        if let Some(avail) = self.available_bytes() {
            self.total_bytes().saturating_sub(avail)
        } else {
            self.total_bytes().saturating_sub(self.free_bytes())
        }
    }

    /// Memory used percent (0..100). Uses available if present.
    pub fn used_percent(&self) -> f64 {
        let total = self.total_bytes() as f64;
        if total == 0.0 {
            return 0.0;
        }
        let used = self.used_bytes() as f64;
        (used / total) * 100.0
    }

    /// Swap used percent if swap fields present
    pub fn swap_used_percent(&self) -> Option<f64> {
        match (self.swap_total_kb, self.swap_free_kb) {
            (Some(total), Some(free)) if total > 0 => {
                let used = total.saturating_sub(free) as f64;
                Some((used / (total as f64)) * 100.0)
            }
            _ => None,
        }
    }
}


impl Monitorable for MemInfo {
    fn exec_cmd(&self) -> &'static str {
        "cat /proc/meminfo"
    }

    /// Parse the full text of /proc/meminfo into a MemInfo.
    fn parse_from_str(&mut self, s: &str) -> anyhow::Result<()> {
        let mut map: HashMap<String, u64> = HashMap::new();

        for line in s.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            // Expect lines like: "MemTotal:       8006048 kB"
            if let Some(pos) = line.find(':') {
                let key = line[..pos].trim().to_string();
                let rest = line[pos + 1..].trim();
                // Extract leading number
                let mut num_str = String::new();
                for ch in rest.chars() {
                    if ch.is_ascii_digit() {
                        num_str.push(ch);
                    } else if !num_str.is_empty() {
                        break;
                    }
                }
                if num_str.is_empty() {
                    continue;
                }
                if let Ok(val) = num_str.parse::<u64>() {
                    map.insert(key, val);
                }
            }
        }

        let mem_total_kb = *map
            .get("MemTotal")
            .ok_or_else(|| anyhow::anyhow!("MemTotal missing"))?;
        let mem_free_kb = *map
            .get("MemFree")
            .ok_or_else(|| anyhow::anyhow!("MemFree missing"))?;

        *self = MemInfo {
            mem_total_kb,
            mem_free_kb,
            mem_available_kb: map.get("MemAvailable").copied(),
            buffers_kb: map.get("Buffers").copied(),
            cached_kb: map.get("Cached").copied(),
            swap_total_kb: map.get("SwapTotal").copied(),
            swap_free_kb: map.get("SwapFree").copied(),
            other: map.into_iter().collect(),
        };
        Ok(())
    }

    fn common_display(&self) -> String {
        let total_gb = self.total_bytes() as f64 / 1024.0 / 1024.0 / 1024.0;
        let used_gb = self.used_bytes() as f64 / 1024.0 / 1024.0 / 1024.0;
        let used_pct = self.used_percent();

        let mut out = format!("Total Memory: {:.2} GB, Used: {:.2} GB ({:.2} %)", total_gb, used_gb, used_pct);
        if let Some(avail) = self.available_bytes() {
            let avail_gb = avail as f64 / 1024.0 / 1024.0 / 1024.0;
            out.push_str(&format!(", Available: {:.2} GB", avail_gb));
        }
        if let Some(swap_pct) = self.swap_used_percent() {
            out.push_str(&format!(", Swap Used: {:.2} %", swap_pct));
        }
        out.push_str("\n");
        out
    }
}
