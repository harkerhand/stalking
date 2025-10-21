use crate::model::{MonitorEvent, MonitorKind, MonitorPayload};
use crate::monitor::Monitorable;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use std::collections::HashMap;
use std::io::{stdout, Write};
use tokio::sync::mpsc::Receiver;
use tokio::time::{sleep, Duration};

pub fn spawn_plain(
    mut rx: Receiver<MonitorEvent>,
    interval_ms: u64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut server_states: HashMap<String, HashMap<MonitorKind, MonitorPayload>> = HashMap::new();
        let mut server_list: Vec<String> = Vec::new();
        let mut current_server_idx: usize = 0;
        let kinds = [MonitorKind::Mem, MonitorKind::Cpu, MonitorKind::Disk, MonitorKind::Net];
        let mut current_kind_idx: usize = 0;

        // 启用原始模式，便于捕获按键
        let _raw = crossterm::terminal::enable_raw_mode();

        loop {
            // 1. 收集所有新事件
            while let Ok(event) = rx.try_recv() {
                match event {
                    MonitorEvent::Sample { server, kind, payload, .. } => {
                        if !server_states.contains_key(&server) {
                            server_list.push(server.clone());
                        }
                        server_states.entry(server).or_default().insert(kind, payload);
                    }
                    MonitorEvent::Error { server, kind, error, .. } => {
                        eprintln!("[{}][{:?}]: {}", server, kind, error);
                    }
                }
            }
            // 保证 current_server_idx 有效
            if server_list.is_empty() {
                current_server_idx = 0;
            } else if current_server_idx >= server_list.len() {
                current_server_idx = server_list.len() - 1;
            }

            // 2. 处理键盘事件（非阻塞）
            while event::poll(Duration::from_millis(1)).unwrap_or(false) {
                if let Event::Key(KeyEvent { code, .. }) = event::read().unwrap() {
                    match code {
                        KeyCode::Char('l') => {
                            if !server_list.is_empty() {
                                current_server_idx = (current_server_idx + server_list.len() - 1) % server_list.len();
                            }
                        }
                        KeyCode::Char('n') => {
                            if !server_list.is_empty() {
                                current_server_idx = (current_server_idx + 1) % server_list.len();
                            }
                        }
                        KeyCode::Char(c) if c >= '1' && c <= '4' => {
                            current_kind_idx = (c as u8 - b'1') as usize;
                        }
                        KeyCode::Esc | KeyCode::Char('q') => {
                            crossterm::terminal::disable_raw_mode().ok();
                            return;
                        }
                        _ => {}
                    }
                }
            }

            // 3. 清屏
            print!("\x1B[2J\x1B[1;1H");
            stdout().flush().ok();

            // 4. 显示当前 server 和监控项
            if !server_list.is_empty() {
                let server = &server_list[current_server_idx];
                println!("=== Server: {} ({}/{}) ===", server, current_server_idx + 1, server_list.len());
                let monitor_map = server_states.get(server).unwrap();
                let kind = &kinds[current_kind_idx];
                let kind_name = match kind {
                    MonitorKind::Mem => "MEM",
                    MonitorKind::Cpu => "CPU",
                    MonitorKind::Disk => "DISK",
                    MonitorKind::Net => "NET",
                };
                print!("[{}] ", kind_name);
                if let Some(payload) = monitor_map.get(&kind) {
                    let display = match payload {
                        MonitorPayload::Mem(mem) => mem.common_display(),
                        MonitorPayload::Cpu(cpu) => cpu.common_display(),
                        MonitorPayload::Disk(disk) => disk.common_display(),
                        MonitorPayload::Net(net) => net.common_display(),
                        _ => "Unsupported Payload".to_string(),
                    };
                    println!("{}", display);
                } else {
                    println!("NO DATA");
                }
                println!("\n[N/L] NEXT/LAST SERVER  [1-4] MEM/CPU/DISK/NET  [q] QUIT");
            } else {
                println!("NO SERVERS DATA");
            }

            sleep(Duration::from_millis(interval_ms)).await;
        }
    })
}
