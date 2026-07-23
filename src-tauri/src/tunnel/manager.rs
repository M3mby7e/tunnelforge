use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use uuid::Uuid;

use crate::model::TunnelConfig;
use crate::tunnel::event::EventSink;
use crate::tunnel::runtime::{spawn, RunningTunnel};
use crate::tunnel::session::ConnectSecrets;

/// Owns every running tunnel and drives start/stop/start-all/stop-all.
pub struct TunnelManager {
    running: Mutex<HashMap<Uuid, RunningTunnel>>,
    sink: EventSink,
}

impl TunnelManager {
    pub fn new(sink: EventSink) -> Self {
        Self {
            running: Mutex::new(HashMap::new()),
            sink,
        }
    }

    pub fn is_running(&self, id: &Uuid) -> bool {
        self.running.lock().expect("lock").contains_key(id)
    }

    pub fn running_ids(&self) -> Vec<Uuid> {
        self.running.lock().expect("lock").keys().copied().collect()
    }

    /// Start a tunnel. If it is already running, it is stopped first.
    pub async fn start(&self, cfg: TunnelConfig, secrets: ConnectSecrets, known_hosts: PathBuf) {
        self.stop(&cfg.id).await;
        let id = cfg.id;
        let running = spawn(cfg, secrets, known_hosts, self.sink.clone());
        self.running.lock().expect("lock").insert(id, running);
    }

    /// Stop a tunnel and wait for it to wind down. No-op if not running.
    pub async fn stop(&self, id: &Uuid) {
        let running = self.running.lock().expect("lock").remove(id);
        if let Some(running) = running {
            running.stop().await;
        }
    }

    /// Stop every running tunnel.
    pub async fn stop_all(&self) {
        let all: Vec<RunningTunnel> = {
            let mut map = self.running.lock().expect("lock");
            map.drain().map(|(_, running)| running).collect()
        };
        for running in all {
            running.stop().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AuthConfig, ForwardKind, ForwardTarget, ListenSpec, ReconnectPolicy, SshEndpoint,
        TunnelState,
    };
    use crate::tunnel::event::EngineEvent;
    use chrono::Utc;
    use tokio::net::TcpListener;
    use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
    use tokio::time::{timeout, Duration};

    fn local_cfg(host: &str, port: u16) -> TunnelConfig {
        let now = Utc::now();
        TunnelConfig {
            id: Uuid::new_v4(),
            name: "test".into(),
            description: None,
            kind: ForwardKind::Local,
            enabled: true,
            auto_start: false,
            // Disabled so an unreachable server yields a terminal Error rather
            // than retrying forever.
            reconnect: ReconnectPolicy {
                enabled: false,
                ..ReconnectPolicy::default()
            },
            ssh: SshEndpoint {
                host: host.into(),
                port,
                username: "tester".into(),
            },
            auth: AuthConfig::Agent,
            listen: ListenSpec {
                bind_address: "127.0.0.1".into(),
                port: 0,
            },
            target: Some(ForwardTarget {
                host: "127.0.0.1".into(),
                port: 9,
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

    /// Bind then drop a listener to obtain a port nothing is listening on.
    async fn free_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        port
    }

    async fn wait_for_error(rx: &mut UnboundedReceiver<EngineEvent>, id: Uuid) -> bool {
        let deadline = Duration::from_secs(5);
        timeout(deadline, async {
            while let Some(event) = rx.recv().await {
                if let EngineEvent::Status(status) = event {
                    if status.id == id && status.state == TunnelState::Error {
                        return true;
                    }
                }
            }
            false
        })
        .await
        .unwrap_or(false)
    }

    #[tokio::test]
    async fn start_emits_error_status_when_server_unreachable() {
        let (tx, mut rx) = unbounded_channel();
        let manager = TunnelManager::new(tx);
        let cfg = local_cfg("127.0.0.1", free_port().await);
        let id = cfg.id;
        let tmp = tempfile::tempdir().unwrap();

        manager
            .start(
                cfg,
                ConnectSecrets::default(),
                tmp.path().join("known_hosts"),
            )
            .await;

        assert!(wait_for_error(&mut rx, id).await, "expected Error status");
    }

    #[tokio::test]
    async fn stop_removes_the_tunnel_from_the_registry() {
        let (tx, _rx) = unbounded_channel();
        let manager = TunnelManager::new(tx);
        let cfg = local_cfg("127.0.0.1", free_port().await);
        let id = cfg.id;
        let tmp = tempfile::tempdir().unwrap();

        manager
            .start(
                cfg,
                ConnectSecrets::default(),
                tmp.path().join("known_hosts"),
            )
            .await;
        assert!(manager.is_running(&id));

        manager.stop(&id).await;
        assert!(!manager.is_running(&id));
    }
}
