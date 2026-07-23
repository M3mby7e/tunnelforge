use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::model::auth_config::AuthConfig;
use crate::model::tunnel_config::SshEndpoint;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "lowercase")]
pub enum ProxyKind {
    Http,
    Socks5,
}

/// A proxy used to reach the SSH server (not the tunnel payload).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfig {
    pub kind: ProxyKind,
    pub host: String,
    pub port: u16,
    /// Optional keychain ref for proxy credentials.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_ref: Option<String>,
}

/// A bastion/jump host to hop through before reaching the SSH server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct JumpHost {
    pub endpoint: SshEndpoint,
    pub auth: AuthConfig,
}
