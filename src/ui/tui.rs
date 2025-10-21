use crossterm::event::{Event, KeyEvent};
use crossterm::{cursor, event, event::KeyCode, execute, terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType}};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::collections::HashMap;
use std::io::stdout;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};

use crate::model::{MonitorEvent, MonitorKind, MonitorPayload};
use crate::monitor::Monitorable;

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

/// spawn_tui 返回一个 JoinHandle，包含主循环 + 渲染任务
pub fn spawn_tui(
    mut rx: mpsc::Receiver<MonitorEvent>,
    interval_ms: u64,
    shutdown_tx: broadcast::Sender<()>,
    servers: Vec<String>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut shutdown_rx = shutdown_tx.subscribe();
        // 共享状态
        let state = Arc::new(RwLock::new(AppState::new(servers)));
        let kinds = [
            MonitorKind::Mem,
            MonitorKind::Cpu,
            MonitorKind::Disk,
            MonitorKind::Net,
        ];

        // 初始化终端
        let terminal = Arc::new(Mutex::new(
            Terminal::new(CrosstermBackend::new(stdout())).unwrap(),
        ));
        {
            let mut term = terminal.lock().await;
            let _ = execute!(term.backend_mut(), cursor::Hide, Clear(ClearType::All), cursor::MoveTo(0, 0));
        }
        enable_raw_mode().ok();

        // 🔹 渲染任务
        let render_state = state.clone();
        let render_term = terminal.clone();
        let value = state.clone();
        let render_handle = tokio::spawn(async move {
            loop {
                // 处理键盘事件（非阻塞）
                if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                    if let Event::Key(KeyEvent { code, kind, .. }) = event::read().unwrap() {
                        if kind == event::KeyEventKind::Press {
                            let mut state = value.write().await;
                            if state.handle_key(code) {
                                let _ = shutdown_tx.send(());
                                break;
                            }
                        }
                    }
                }
                {
                    let state = render_state.read().await;
                    let mut term = render_term.lock().await;
                    render(&mut term, &state, &kinds);
                }
            }
        });
        // 🔹 主循环：处理事件和用户输入
        loop {
            // 检查 shutdown
            if shutdown_rx.try_recv().is_ok() {
                break;
            }

            // 处理数据事件
            while let Ok(ev) = rx.try_recv() {
                let mut state = state.write().await;
                state.update_event(ev);
            }

            // 控制循环频率
            tokio::time::sleep(Duration::from_millis(interval_ms)).await;
        }

        // 停止渲染任务
        render_handle.abort();

        // 恢复终端状态
        disable_raw_mode().ok();
        {
            let mut term = terminal.lock().await;
            execute!(term.backend_mut(), cursor::Show, Clear(ClearType::All), cursor::MoveTo(0, 0)).ok();
        }
    })
}

/// 渲染函数，只读取状态，不修改
fn render(
    term: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    state: &AppState,
    kinds: &[MonitorKind; 4],
) {
    let text = if state.servers.is_empty() {
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
    };

    let help = "[N/L] NEXT/LAST SERVER  [1-4] MEM/CPU/DISK/NET  [Q] QUIT";

    let _ = term.draw(|f| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Min(3), Constraint::Length(2)])
            .split(f.area());
        f.render_widget(
            Paragraph::new(text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Stalking Monitor"),
                )
                .style(Style::default().fg(Color::White)),
            chunks[0],
        );
        f.render_widget(
            Paragraph::new(help).style(Style::default().fg(Color::Yellow)),
            chunks[1],
        );
    });
}
