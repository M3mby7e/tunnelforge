//! The tunnel engine: SSH sessions, per-mode forwarding, per-tunnel runtime,
//! and the manager that owns them. Decoupled from Tauri via an event channel.

pub mod event;
pub mod forward;
pub mod handler;
pub mod manager;
pub mod proxy;
pub mod reconnect;
pub mod runtime;
pub mod session;
pub mod stats;

pub use event::{EngineEvent, EventSink};
pub use manager::TunnelManager;
pub use session::ConnectSecrets;
