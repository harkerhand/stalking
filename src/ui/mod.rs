use crossterm::event::KeyCode;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

pub mod plain;
pub mod tui;

use crate::model::{MonitorEvent, MonitorKind, MonitorPayload};
use crate::monitor::Monitorable;
pub use plain::spawn_plain;
pub use tui::spawn_tui;

#[derive(Debug, Deserialize, Clone, Default)]
pub enum DisplayKind {
    #[default]
    Plain,
    Tui,
}


struct AppState {
    data: HashMap<String, HashMap<MonitorKind, MonitorPayload>>,
    servers: Vec<String>,
    current_server: AtomicUsize,
    current_kind: AtomicUsize,
}

impl AppState {
    fn new(servers: Vec<String>) -> Self {
        Self {
            data: HashMap::new(),
            servers,
            current_server: AtomicUsize::new(0),
            current_kind: AtomicUsize::new(0),
        }
    }

    fn next_server(&mut self) {
        if !self.servers.is_empty() {
            self.current_server.store(
                (self.current_server.load(Ordering::Relaxed) + 1) % self.servers.len(),
                Ordering::Relaxed,
            );
        }
    }

    fn prev_server(&mut self) {
        if !self.servers.is_empty() {
            self.current_server.store(
                (self.current_server.load(Ordering::Relaxed) + self.servers.len() - 1)
                    % self.servers.len(),
                Ordering::Relaxed,
            );
        }
    }

    fn set_kind(&mut self, idx: usize) {
        self.current_kind.store(
            idx.min(3), // 0-3 对应 4 种类型
            Ordering::Relaxed,
        )
    }

    fn update_event(&mut self, ev: MonitorEvent) {
        match ev {
            MonitorEvent::Sample {
                server,
                kind,
                payload,
                ..
            } => {
                if self.servers.contains(&server) {
                    self.data.entry(server).or_default().insert(kind, payload);
                }
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

    /// 处理键盘事件，返回 true 表示请求退出
    fn handle_key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Char('n') => self.next_server(),
            KeyCode::Char('l') => self.prev_server(),
            KeyCode::Char(c) if ('1'..='4').contains(&c) => {
                self.set_kind((c as u8 - b'1') as usize)
            }
            KeyCode::Esc | KeyCode::Char('q') => return true,
            _ => {}
        }
        false
    }
}


/// 生成主显示文本
fn main_text(state: &AppState, kinds: &[MonitorKind; 4]) -> String {
    if state.servers.is_empty() {
        "NO SERVERS DATA".to_string()
    } else {
        let server = &state.servers[state.current_server.load(Ordering::Relaxed)];
        let kind = &kinds[state.current_kind.load(Ordering::Relaxed)];
        let kind_name = match kind {
            MonitorKind::Mem => "MEM",
            MonitorKind::Cpu => "CPU",
            MonitorKind::Disk => "DISK",
            MonitorKind::Net => "NET",
        };
        let mut t = format!(
            "=== Server: {} ({}/{}) ===\n[{}] ",
            server,
            state.current_server.load(Ordering::Relaxed) + 1,
            state.servers.len(),
            kind_name
        );
        if let Some(map) = state.data.get(server) {
            if let Some(payload) = map.get(kind) {
                t.push_str(&payload.common_display());
            } else {
                t.push_str("NO DATA");
            }
        } else {
            t.push_str("NO DATA");
        }
        t
    }
}