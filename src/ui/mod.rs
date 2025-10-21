use serde::Deserialize;

pub mod plain;
pub mod tui;

pub use plain::spawn_plain;
pub use tui::spawn_tui;

#[derive(Debug, Deserialize, Clone, Default)]
pub enum DisplayKind {
    #[default]
    Plain,
    Tui,
}