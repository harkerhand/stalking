use crate::Monitorable;
use anyhow::Result;

/// Parsed info from `df -P` (POSIX format).
#[derive(Debug, Clone, Default)]
pub struct DiskInfo {
    pub filesystems: Vec<MountEntry>,
}

/// A single mount entry (filesystem)
#[derive(Debug, Clone)]
pub struct MountEntry {
    pub filesystem: String,
    pub size_kb: u64,
    pub used_kb: u64,
    pub avail_kb: u64,
    pub use_percent: f64,
    pub mount_point: String,
}

impl DiskInfo {
    /// Compute total used/total percent
    pub fn total_used_percent(&self) -> f64 {
        let total: u64 = self.filesystems.iter().map(|e| e.size_kb).sum();
        if total == 0 {
            return 0.0;
        }
        let used: u64 = self.filesystems.iter().map(|e| e.used_kb).sum();
        (used as f64 / total as f64) * 100.0
    }

    /// Total disk size in bytes
    pub fn total_bytes(&self) -> u64 {
        self.filesystems.iter().map(|e| e.size_kb * 1024).sum()
    }

    /// Total used disk size in bytes
    pub fn used_bytes(&self) -> u64 {
        self.filesystems.iter().map(|e| e.used_kb * 1024).sum()
    }

    /// Total available disk size in bytes
    pub fn avail_bytes(&self) -> u64 {
        self.filesystems.iter().map(|e| e.avail_kb * 1024).sum()
    }
}

impl Monitorable for DiskInfo {
    fn exec_cmd(&self) -> &'static str {
        // POSIX format, easier to parse
        "df -P -x tmpfs -x devtmpfs"
    }

    fn parse_from_str(&mut self, s: &str) -> Result<()> {
        let mut entries = Vec::new();

        for (i, line) in s.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || i == 0 {
                // skip header
                continue;
            }
            let cols: Vec<&str> = line.split_whitespace().collect();
            // POSIX format: Filesystem Size Used Avail Use% Mounted_on
            if cols.len() < 6 {
                continue;
            }

            let fs = cols[0].to_string();
            let size_kb = cols[1].parse::<u64>().unwrap_or(0);
            let used_kb = cols[2].parse::<u64>().unwrap_or(0);
            let avail_kb = cols[3].parse::<u64>().unwrap_or(0);

            // remove trailing %
            let use_percent_str = cols[4].trim_end_matches('%');
            let use_percent = use_percent_str.parse::<f64>().unwrap_or(0.0);

            let mount_point = cols[5].to_string();

            entries.push(MountEntry {
                filesystem: fs,
                size_kb,
                used_kb,
                avail_kb,
                use_percent,
                mount_point,
            });
        }
        self.filesystems = entries;
        Ok(())
    }

    fn common_display(&self) -> String {
        if self.filesystems.is_empty() {
            return "No storage info found".to_string();
        }

        let total_gb = self.total_bytes() as f64 / 1_073_741_824.0;
        let used_gb = self.used_bytes() as f64 / 1_073_741_824.0;
        let avail_gb = self.avail_bytes() as f64 / 1_073_741_824.0;
        let used_pct = self.total_used_percent();

        let mut summary = format!(
            "Total Storage: {:.2} GB, Used: {:.2} GB ({:.2}%), Available: {:.2} GB\n",
            total_gb, used_gb, used_pct, avail_gb
        );

        summary.push_str("Mount Points:\n");
        for e in &self.filesystems {
            summary.push_str(&format!(
                "  {:<15} {:>6.1}G used ({:>5.1}%), mount: {}\n",
                e.filesystem,
                e.used_kb as f64 / 1_048_576.0,
                e.use_percent,
                e.mount_point
            ));
        }

        summary
    }
}
