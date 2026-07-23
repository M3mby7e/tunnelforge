use std::path::PathBuf;

use tauri::State;
use uuid::Uuid;

use crate::error::Error;
use crate::model::{AuthConfig, TunnelConfig};
use crate::store::{keychain_store, known_hosts, paths, ConfigStore};
use crate::tunnel::{ConnectSecrets, TunnelManager};

fn known_hosts_path() -> Result<PathBuf, Error> {
    Ok(paths::app_config_dir()?.join("known_hosts"))
}

/// Resolve one auth method's secret into `into`.
fn resolve_auth(auth: &AuthConfig, into: &mut ConnectSecrets) -> Result<(), Error> {
    match auth {
        AuthConfig::Password { secret_ref } => {
            into.password = keychain_store::get_secret(secret_ref)?;
        }
        AuthConfig::PrivateKey {
            passphrase_ref: Some(reference),
            ..
        } => {
            into.passphrase = keychain_store::get_secret(reference)?;
        }
        AuthConfig::KeyboardInteractive {
            secret_ref: Some(reference),
        } => {
            into.password = keychain_store::get_secret(reference)?;
        }
        _ => {}
    }
    Ok(())
}

/// Resolve every secret a tunnel needs from the OS keychain (main auth, proxy
/// credentials, and each jump host's auth).
fn resolve_secrets(tunnel: &TunnelConfig) -> Result<ConnectSecrets, Error> {
    let mut secrets = ConnectSecrets::default();
    resolve_auth(&tunnel.auth, &mut secrets)?;

    // Proxy credentials are stored as "user\npassword".
    if let Some(reference) = tunnel.proxy.as_ref().and_then(|p| p.auth_ref.as_ref()) {
        if let Some(raw) = keychain_store::get_secret(reference)? {
            let (user, pass) = raw.split_once('\n').unwrap_or((raw.as_str(), ""));
            secrets.proxy_auth = Some((user.to_string(), pass.to_string()));
        }
    }

    for jump in &tunnel.jump_hosts {
        let mut jump_secrets = ConnectSecrets::default();
        resolve_auth(&jump.auth, &mut jump_secrets)?;
        secrets.jumps.push(jump_secrets);
    }
    Ok(secrets)
}

fn find_tunnel(store: &ConfigStore, id: Uuid) -> Result<TunnelConfig, Error> {
    store
        .load()?
        .tunnels
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| Error::NotFound(format!("tunnel {id}")))
}

/// Start a single tunnel by id.
#[tauri::command]
pub async fn start_tunnel(
    id: Uuid,
    store: State<'_, ConfigStore>,
    manager: State<'_, TunnelManager>,
) -> Result<(), Error> {
    let tunnel = find_tunnel(&store, id)?;
    let secrets = resolve_secrets(&tunnel)?;
    manager.start(tunnel, secrets, known_hosts_path()?).await;
    Ok(())
}

/// Stop a single tunnel by id.
#[tauri::command]
pub async fn stop_tunnel(id: Uuid, manager: State<'_, TunnelManager>) -> Result<(), Error> {
    manager.stop(&id).await;
    Ok(())
}

/// Start every enabled tunnel. Tunnels whose secrets can't be resolved are skipped.
#[tauri::command]
pub async fn start_all_tunnels(
    store: State<'_, ConfigStore>,
    manager: State<'_, TunnelManager>,
) -> Result<(), Error> {
    let config = store.load()?;
    let known_hosts = known_hosts_path()?;
    for tunnel in config.tunnels.into_iter().filter(|t| t.enabled) {
        let Ok(secrets) = resolve_secrets(&tunnel) else {
            continue;
        };
        manager.start(tunnel, secrets, known_hosts.clone()).await;
    }
    Ok(())
}

/// Stop every running tunnel.
#[tauri::command]
pub async fn stop_all_tunnels(manager: State<'_, TunnelManager>) -> Result<(), Error> {
    manager.stop_all().await;
    Ok(())
}

/// Ids of tunnels the engine currently has running.
#[tauri::command]
pub fn running_tunnels(manager: State<'_, TunnelManager>) -> Vec<Uuid> {
    manager.running_ids()
}

/// Forget the pinned host key for a server so the next connection re-trusts it.
/// Use after a legitimate server key change.
#[tauri::command]
pub fn forget_host_key(host: String, port: u16) -> Result<(), Error> {
    known_hosts::forget_host(&known_hosts_path()?, &host, port)
}
