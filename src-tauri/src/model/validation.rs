use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::model::app_config::AppConfig;
use crate::model::auth_config::AuthConfig;
use crate::model::proxy_config::ProxyConfig;
use crate::model::tunnel_config::{ForwardKind, SshEndpoint, TunnelConfig};

/// A single field-level validation problem, surfaced to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl ValidationError {
    fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}

type Errors = Vec<ValidationError>;

/// Validate the entire configuration (every tunnel).
pub fn validate_app_config(cfg: &AppConfig) -> Result<(), Errors> {
    let mut errs = Errors::new();
    for (i, tunnel) in cfg.tunnels.iter().enumerate() {
        validate_tunnel_into(tunnel, &format!("tunnels[{i}]."), &mut errs);
    }
    finish(errs)
}

/// Validate a single tunnel in isolation.
pub fn validate_tunnel(tunnel: &TunnelConfig) -> Result<(), Errors> {
    let mut errs = Errors::new();
    validate_tunnel_into(tunnel, "", &mut errs);
    finish(errs)
}

fn finish(errs: Errors) -> Result<(), Errors> {
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs)
    }
}

fn validate_tunnel_into(t: &TunnelConfig, prefix: &str, errs: &mut Errors) {
    let field = |name: &str| format!("{prefix}{name}");

    if t.name.trim().is_empty() {
        errs.push(ValidationError::new(field("name"), "Name is required"));
    }

    validate_ssh(&t.ssh, prefix, errs);
    validate_auth(&t.auth, &field("auth"), errs);

    // Listen bind address.
    if !is_valid_host(&t.listen.bind_address) {
        errs.push(ValidationError::new(
            field("listen.bindAddress"),
            "Bind address must be a valid IP or hostname",
        ));
    }

    // Listen port: 0 means "auto-pick" and is only allowed for local/dynamic.
    match t.kind {
        ForwardKind::Remote if t.listen.port == 0 => {
            errs.push(ValidationError::new(
                field("listen.port"),
                "A remote forward requires an explicit port (1-65535)",
            ));
        }
        _ => {}
    }

    // Target is required for local & remote, and ignored for dynamic.
    match t.kind {
        ForwardKind::Local | ForwardKind::Remote => match &t.target {
            None => errs.push(ValidationError::new(
                field("target"),
                "A target host and port are required for this tunnel type",
            )),
            Some(target) => {
                if !is_valid_host(&target.host) {
                    errs.push(ValidationError::new(
                        field("target.host"),
                        "Target host must be a valid IP or hostname",
                    ));
                }
                if target.port == 0 {
                    errs.push(ValidationError::new(
                        field("target.port"),
                        "Target port must be between 1 and 65535",
                    ));
                }
            }
        },
        ForwardKind::Dynamic => {}
    }

    if let Some(proxy) = &t.proxy {
        validate_proxy(proxy, &field("proxy"), errs);
    }

    for (i, jump) in t.jump_hosts.iter().enumerate() {
        let jp = format!("{}jumpHosts[{i}].", prefix);
        validate_ssh(&jump.endpoint, &jp, errs);
        validate_auth(&jump.auth, &format!("{jp}auth"), errs);
    }

    // Reconnect policy sanity.
    let r = &t.reconnect;
    if r.factor < 1.0 {
        errs.push(ValidationError::new(
            field("reconnect.factor"),
            "Backoff factor must be at least 1.0",
        ));
    }
    if r.initial_delay_ms > r.max_delay_ms {
        errs.push(ValidationError::new(
            field("reconnect.initialDelayMs"),
            "Initial delay cannot exceed max delay",
        ));
    }
}

fn validate_ssh(ssh: &SshEndpoint, prefix: &str, errs: &mut Errors) {
    let field = |name: &str| format!("{prefix}ssh.{name}");
    if !is_valid_host(&ssh.host) {
        errs.push(ValidationError::new(
            field("host"),
            "SSH host must be a valid IP or hostname",
        ));
    }
    if ssh.port == 0 {
        errs.push(ValidationError::new(
            field("port"),
            "SSH port must be between 1 and 65535",
        ));
    }
    if ssh.username.trim().is_empty() {
        errs.push(ValidationError::new(
            field("username"),
            "SSH username is required",
        ));
    }
}

fn validate_auth(auth: &AuthConfig, prefix: &str, errs: &mut Errors) {
    match auth {
        AuthConfig::Password { secret_ref } => {
            if secret_ref.trim().is_empty() {
                errs.push(ValidationError::new(
                    format!("{prefix}.secretRef"),
                    "Password reference is missing",
                ));
            }
        }
        AuthConfig::PrivateKey { key_path, .. } => {
            if key_path.trim().is_empty() {
                errs.push(ValidationError::new(
                    format!("{prefix}.keyPath"),
                    "Private key path is required",
                ));
            }
        }
        AuthConfig::PrivateKeyInline { key_ref, .. } => {
            if key_ref.trim().is_empty() {
                errs.push(ValidationError::new(
                    format!("{prefix}.keyRef"),
                    "Imported key reference is missing",
                ));
            }
        }
        AuthConfig::Agent | AuthConfig::KeyboardInteractive { .. } => {}
    }
}

fn validate_proxy(proxy: &ProxyConfig, prefix: &str, errs: &mut Errors) {
    if !is_valid_host(&proxy.host) {
        errs.push(ValidationError::new(
            format!("{prefix}.host"),
            "Proxy host must be a valid IP or hostname",
        ));
    }
    if proxy.port == 0 {
        errs.push(ValidationError::new(
            format!("{prefix}.port"),
            "Proxy port must be between 1 and 65535",
        ));
    }
}

/// Accepts an IP literal, `localhost`, or a syntactically valid hostname.
fn is_valid_host(host: &str) -> bool {
    if host.is_empty() || host.len() > 253 {
        return false;
    }
    if host.parse::<IpAddr>().is_ok() {
        return true;
    }
    host.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            && !label.starts_with('-')
            && !label.ends_with('-')
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::tunnel_config::{ForwardTarget, ListenSpec, ReconnectPolicy};
    use chrono::Utc;
    use uuid::Uuid;

    fn base_tunnel(kind: ForwardKind) -> TunnelConfig {
        let now = Utc::now();
        TunnelConfig {
            id: Uuid::new_v4(),
            name: "test".into(),
            description: None,
            kind,
            enabled: true,
            auto_start: false,
            reconnect: ReconnectPolicy::default(),
            ssh: SshEndpoint {
                host: "example.com".into(),
                port: 22,
                username: "sam".into(),
            },
            auth: AuthConfig::Agent,
            listen: ListenSpec {
                bind_address: "127.0.0.1".into(),
                port: 5432,
            },
            target: Some(ForwardTarget {
                host: "db.internal".into(),
                port: 5432,
            }),
            proxy: None,
            jump_hosts: Vec::new(),
            keep_alive_seconds: None,
            connect_timeout_ms: None,
            compression: None,
            group: None,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn accepts_a_valid_local_tunnel() {
        assert!(validate_tunnel(&base_tunnel(ForwardKind::Local)).is_ok());
    }

    #[test]
    fn dynamic_tunnel_does_not_require_a_target() {
        let mut t = base_tunnel(ForwardKind::Dynamic);
        t.target = None;
        assert!(validate_tunnel(&t).is_ok());
    }

    #[test]
    fn local_tunnel_requires_a_target() {
        let mut t = base_tunnel(ForwardKind::Local);
        t.target = None;
        let errs = validate_tunnel(&t).unwrap_err();
        assert!(errs.iter().any(|e| e.field == "target"));
    }

    #[test]
    fn rejects_empty_name() {
        let mut t = base_tunnel(ForwardKind::Local);
        t.name = "   ".into();
        let errs = validate_tunnel(&t).unwrap_err();
        assert!(errs.iter().any(|e| e.field == "name"));
    }

    #[test]
    fn rejects_zero_ssh_port() {
        let mut t = base_tunnel(ForwardKind::Local);
        t.ssh.port = 0;
        let errs = validate_tunnel(&t).unwrap_err();
        assert!(errs.iter().any(|e| e.field == "ssh.port"));
    }

    #[test]
    fn remote_tunnel_rejects_auto_listen_port() {
        let mut t = base_tunnel(ForwardKind::Remote);
        t.listen.port = 0;
        let errs = validate_tunnel(&t).unwrap_err();
        assert!(errs.iter().any(|e| e.field == "listen.port"));
    }

    #[test]
    fn rejects_bad_bind_address() {
        let mut t = base_tunnel(ForwardKind::Local);
        t.listen.bind_address = "not a host!".into();
        let errs = validate_tunnel(&t).unwrap_err();
        assert!(errs.iter().any(|e| e.field == "listen.bindAddress"));
    }

    #[test]
    fn rejects_backoff_factor_below_one() {
        let mut t = base_tunnel(ForwardKind::Local);
        t.reconnect.factor = 0.5;
        let errs = validate_tunnel(&t).unwrap_err();
        assert!(errs.iter().any(|e| e.field == "reconnect.factor"));
    }

    #[test]
    fn host_helper_accepts_ip_and_hostname() {
        assert!(is_valid_host("127.0.0.1"));
        assert!(is_valid_host("::1"));
        assert!(is_valid_host("localhost"));
        assert!(is_valid_host("db.internal.example.com"));
        assert!(!is_valid_host(""));
        assert!(!is_valid_host("-bad.example.com"));
        assert!(!is_valid_host("has space"));
    }
}
