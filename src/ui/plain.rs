use crate::model::{MonitorEvent, MonitorKind, MonitorPayload};
use crate::monitor::Monitorable;
use std::collections::HashMap;
use tokio::sync::mpsc::Receiver;
use tokio::time::{Duration, sleep};

pub fn spawn_plain(
    mut rx: Receiver<MonitorEvent>,
    interval_ms: u64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // 保存每台服务器最新的监控样本
        let mut server_states: HashMap<String, HashMap<MonitorKind, MonitorPayload>> =
            HashMap::new();

        loop {
            // 1. 收集所有新事件
            while let Ok(event) = rx.try_recv() {
                match event {
                    MonitorEvent::Sample {
                        server,
                        kind,
                        payload,
                        ..
                    } => {
                        server_states
                            .entry(server)
                            .or_default()
                            .insert(kind, payload);
                    }
                    MonitorEvent::Error {
                        server,
                        kind,
                        error,
                        ..
                    } => {
                        eprintln!("[{}][{:?}]: {}", server, kind, error);
                    }
                }
            }

            // 2. 清屏
            print!("\x1B[2J\x1B[1;1H"); // ANSI clear screen + move cursor to 1,1

            // 3. 输出每台服务器状态
            for (server, monitor_map) in &server_states {
                println!("=== Server: {} ===", server);
                for (kind, payload) in monitor_map {
                    let display = match payload {
                        MonitorPayload::Mem(mem) => mem.common_display(),
                        MonitorPayload::Cpu(cpu) => cpu.common_display(),
                        MonitorPayload::Disk(disk) => disk.common_display(),
                        MonitorPayload::Net(net) => net.common_display(),
                        _ => "Unsupported Payload".to_string(),
                    };
                    println!("[{:?}] {}", kind, display);
                }
                println!();
            }

            // 4. 刷新间隔
            sleep(Duration::from_millis(interval_ms)).await;
        }
    })
}
