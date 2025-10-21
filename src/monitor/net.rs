use crate::Monitorable;
use anyhow::Result;

#[derive(Debug, Clone, PartialEq)]
pub struct NetInterface {
    pub name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_rate: f64, // bytes/sec
    pub tx_rate: f64, // bytes/sec
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NetInfo {
    pub interfaces: Vec<NetInterface>,
}

impl NetInfo {
    fn parse_netdev(content: &str) -> Vec<(String, u64, u64)> {
        let mut result = Vec::new();
        for line in content.lines().skip(2) {
            if let Some((iface, rest)) = line.split_once(':') {
                let iface = iface.trim().to_string();
                let fields: Vec<&str> = rest.split_whitespace().collect();
                if fields.len() >= 10 {
                    if let (Ok(rx_bytes), Ok(tx_bytes)) =
                        (fields[0].parse::<u64>(), fields[8].parse::<u64>())
                    {
                        result.push((iface, rx_bytes, tx_bytes));
                    }
                }
            }
        }
        result
    }

    fn diff(
        first: &[(String, u64, u64)],
        second: &[(String, u64, u64)],
        dt: f64,
    ) -> Vec<NetInterface> {
        let mut result = Vec::new();
        for (iface, rx1, tx1) in first {
            if let Some((_, rx2, tx2)) = second.iter().find(|(n, _, _)| n == iface) {
                let rx_rate = (*rx2 as f64 - *rx1 as f64) / dt;
                let tx_rate = (*tx2 as f64 - *tx1 as f64) / dt;
                result.push(NetInterface {
                    name: iface.clone(),
                    rx_bytes: *rx2,
                    tx_bytes: *tx2,
                    rx_rate,
                    tx_rate,
                });
            }
        }
        result
    }
}


impl Monitorable for NetInfo {
    fn exec_cmd(&self) -> &'static str {
        // 两次采样 + 分隔符
        "cat /proc/net/dev; sleep 0.2; cat /proc/net/dev"
    }

    fn parse_from_str(&mut self, s: &str) -> Result<()> {
        let parts: Vec<&str> = s.split("Inter-|").collect();
        if parts.len() < 3 {
            anyhow::bail!("unexpected /proc/net/dev format");
        }
        // 恢复成两个 netdev 数据块
        let first = format!("Inter-|{}", parts[1]);
        let second = format!("Inter-|{}", parts[2]);

        let first_list = Self::parse_netdev(&first);
        let second_list = Self::parse_netdev(&second);
        // 假设 sleep 0.2s
        self.interfaces = Self::diff(&first_list, &second_list, 0.2);
        Ok(())
    }

    fn common_display(&self) -> String {
        let mut s = String::from("Network Interfaces:\n");
        for i in &self.interfaces {
            s.push_str(&format!(
                "  {:<10} RX: {:.1} KB/s | TX: {:.1} KB/s \n",
                i.name,
                i.rx_rate / 1024.0,
                i.tx_rate / 1024.0,
            ));
        }
        s
    }
}
