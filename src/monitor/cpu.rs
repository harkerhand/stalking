use crate::Monitorable;
use anyhow::Result;

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f64,
    pub mem_percent: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CpuInfo {
    pub usage_percent: f64,
    pub user: u64,
    pub system: u64,
    pub idle: u64,
    pub top_processes: Vec<ProcessInfo>,
}

impl CpuInfo {
    fn parse_stat_line(line: &str) -> Option<(u64, u64, u64, u64, u64, u64, u64)> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 8 || parts[0] != "cpu" {
            return None;
        }
        Some((
            parts[1].parse().ok()?, // user
            parts[2].parse().ok()?, // nice
            parts[3].parse().ok()?, // system
            parts[4].parse().ok()?, // idle
            parts[5].parse().ok()?, // iowait
            parts[6].parse().ok()?, // irq
            parts[7].parse().ok()?, // softirq
        ))
    }

    fn read_cpu_total_idle(s: &str) -> Option<(u64, u64)> {
        for line in s.lines() {
            if line.starts_with("cpu ") {
                let (user, nice, system, idle, iowait, irq, softirq) = Self::parse_stat_line(line)?;
                let total = user + nice + system + idle + iowait + irq + softirq;
                let idle_all = idle + iowait;
                return Some((total, idle_all));
            }
        }
        None
    }

    fn calc_usage(first: &str, second: &str) -> Option<f64> {
        let (t1, i1) = Self::read_cpu_total_idle(first)?;
        let (t2, i2) = Self::read_cpu_total_idle(second)?;
        let total_diff = t2.saturating_sub(t1);
        let idle_diff = i2.saturating_sub(i1);
        if total_diff == 0 {
            return Some(0.0);
        }
        Some(((total_diff - idle_diff) as f64 / total_diff as f64) * 100.0)
    }

    /// 解析 ps 输出的进程信息
    fn parse_top_processes(ps_output: &str) -> Vec<ProcessInfo> {
        let mut result = Vec::new();
        for line in ps_output.lines().skip(1) {
            // 跳过标题
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 4 {
                continue;
            }
            if let (Ok(pid), Ok(cpu), Ok(mem)) = (
                cols[0].parse::<u32>(),
                cols[2].parse::<f64>(),
                cols[3].parse::<f64>(),
            ) {
                let name = cols[1].to_string();
                result.push(ProcessInfo {
                    pid,
                    name,
                    cpu_percent: cpu,
                    mem_percent: mem,
                });
            }
        }
        result
    }
}

impl Monitorable for CpuInfo {
    fn exec_cmd(&self) -> &'static str {
        // 一次执行：两次采样 + ps 输出
        "cat /proc/stat; sleep 0.2; cat /proc/stat; echo '---'; ps -eo pid,comm,%cpu,%mem --sort=-%cpu | head -n 11"
    }

    fn parse_from_str(&mut self, s: &str) -> Result<()> {
        // 分割 /proc/stat 和 ps 输出
        let mut parts = s.split("---");
        let stat_part = parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("missing stat"))?;
        let ps_part = parts.next().unwrap_or("");

        let stat_sections: Vec<&str> = stat_part.split("cpu ").collect();
        if stat_sections.len() < 3 {
            anyhow::bail!("unexpected /proc/stat output");
        }

        let first = format!("cpu {}", stat_sections[1]);
        let second = format!("cpu {}", stat_sections[2]);

        let usage = CpuInfo::calc_usage(&first, &second)
            .ok_or_else(|| anyhow::anyhow!("failed to compute CPU usage"))?;

        let (user, _nice, system, idle, _, _, _) =
            CpuInfo::parse_stat_line(second.lines().next().unwrap())
                .ok_or_else(|| anyhow::anyhow!("failed to parse cpu line"))?;

        let top = CpuInfo::parse_top_processes(ps_part);

        *self = CpuInfo {
            usage_percent: usage,
            user,
            system,
            idle,
            top_processes: top,
        };

        Ok(())
    }

    fn common_display(&self) -> String {
        let mut s = format!("CPU Usage: {:.2}%\nTop 10 processes:\n", self.usage_percent);
        for p in &self.top_processes {
            s.push_str(&format!(
                "  {:<10} {:<20} {:>5.1}% CPU {:>5.1}% MEM\n",
                p.pid, p.name, p.cpu_percent, p.mem_percent
            ));
        }
        s
    }
}
