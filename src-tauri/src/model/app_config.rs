use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::model::tunnel_config::{ReconnectPolicy, SshEndpoint, TunnelConfig};

/// The current on-disk config schema version.
pub const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "lowercase")]
pub enum ThemePref {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPrefs {
    pub on_connect: bool,
    pub on_disconnect: bool,
    pub on_error: bool,
}

impl Default for NotificationPrefs {
    fn default() -> Self {
        Self {
            on_connect: false,
            on_disconnect: false,
            on_error: true,
        }
    }
}

/// Global defaults applied to newly created tunnels.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Defaults {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssh: Option<SshEndpoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reconnect: Option<ReconnectPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep_alive_seconds: Option<u32>,
}

/// The whole persisted application configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub version: u32,
    pub theme: ThemePref,
    pub start_on_boot: bool,
    pub minimize_to_tray: bool,
    pub notifications: NotificationPrefs,
    pub defaults: Defaults,
    #[serde(default)]
    pub tunnels: Vec<TunnelConfig>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            theme: ThemePref::System,
            start_on_boot: false,
            minimize_to_tray: true,
            notifications: NotificationPrefs::default(),
            defaults: Defaults::default(),
            tunnels: Vec::new(),
        }
    }
}
