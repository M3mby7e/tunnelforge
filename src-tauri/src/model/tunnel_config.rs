use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::model::auth_config::AuthConfig;
use crate::model::proxy_config::{JumpHost, ProxyConfig};

/// The direction/mode of forwarding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "lowercase")]
pub enum ForwardKind {
    Local,
    Remote,
    Dynamic,
}

/// The SSH server to tunnel through, plus how to log in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SshEndpoint {
    pub host: String,
    pub port: u16,
    pub username: String,
}

/// Where a listener binds (local for local/dynamic, server-side for remote).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ListenSpec {
    pub bind_address: String,
    pub port: u16,
}

/// The ultimate destination for local/remote forwarding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ForwardTarget {
    pub host: String,
    pub port: u16,
}

/// Auto-reconnect backoff policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ReconnectPolicy {
    pub enabled: bool,
    pub initial_delay_ms: u32,
    pub max_delay_ms: u32,
    pub factor: f64,
    pub jitter: bool,
    /// `None` = retry forever.
    #[serde(default)]
    pub max_retries: Option<u32>,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            initial_delay_ms: 1_000,
            max_delay_ms: 60_000,
            factor: 2.0,
            jitter: true,
            max_retries: None,
        }
    }
}

/// A single tunnel definition. This is persisted; secrets are referenced by
/// keychain id inside `auth`/`proxy`, never stored here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct TunnelConfig {
    pub id: Uuid,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub kind: ForwardKind,

    pub enabled: bool,
    pub auto_start: bool,
    pub reconnect: ReconnectPolicy,

    pub ssh: SshEndpoint,
    pub auth: AuthConfig,

    pub listen: ListenSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<ForwardTarget>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<ProxyConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub jump_hosts: Vec<JumpHost>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep_alive_seconds: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connect_timeout_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compression: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
