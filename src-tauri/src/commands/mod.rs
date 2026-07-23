//! Tauri command handlers — the thin, validated IPC boundary between the UI and
//! the core. Commands delegate to the store/engine; they hold no logic of their
//! own beyond input handling.

pub mod config_cmd;
pub mod secret_cmd;
pub mod system_cmd;
pub mod tunnel_cmd;
