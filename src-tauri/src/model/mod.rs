//! Plain data types for the persisted configuration. No behavior, no I/O —
//! just serde-friendly structs plus validation.

pub mod app_config;
pub mod auth_config;
pub mod proxy_config;
pub mod status;
pub mod tunnel_config;
pub mod validation;

pub use app_config::{AppConfig, Defaults, NotificationPrefs, ThemePref, CONFIG_VERSION};
pub use auth_config::AuthConfig;
pub use proxy_config::{JumpHost, ProxyConfig, ProxyKind};
pub use status::{LogLevel, LogLine, StatsSnapshot, TunnelState, TunnelStatus};
pub use tunnel_config::{
    ForwardKind, ForwardTarget, ListenSpec, ReconnectPolicy, SshEndpoint, TunnelConfig,
};
pub use validation::{validate_app_config, validate_tunnel, ValidationError};
