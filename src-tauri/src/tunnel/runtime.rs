use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::error::{Error, Result};
use crate::model::{ForwardKind, TunnelConfig, TunnelState};
use crate::tunnel::event::{Emitter, EventSink};
use crate::tunnel::forward::dynamic::run_dynamic_forward;
use crate::tunnel::forward::local::run_local_forward;
use crate::tunnel::forward::remote::run_remote_forward;
use crate::tunnel::reconnect::{backoff_delay, should_retry};
use crate::tunnel::session::{connect, ConnectSecrets, SshHandle, SshSession};
use crate::tunnel::stats::TunnelStats;

/// How often a connected tunnel emits a fresh stats snapshot.
const STATS_INTERVAL: Duration = Duration::from_secs(1);
/// How often the session is polled for an unexpected disconnect.
const HEALTH_INTERVAL: Duration = Duration::from_secs(1);

/// A running tunnel: its cancellation token and the supervising task.
pub struct RunningTunnel {
    cancel: CancellationToken,
    task: JoinHandle<()>,
}

impl RunningTunnel {
    /// Signal the tunnel to stop and wait for it to wind down.
    pub async fn stop(self) {
        self.cancel.cancel();
        let _ = self.task.await;
    }
}

/// Spawn a supervised tunnel task and return a handle to stop it.
pub fn spawn(
    cfg: TunnelConfig,
    secrets: ConnectSecrets,
    known_hosts: PathBuf,
    sink: EventSink,
) -> RunningTunnel {
    let cancel = CancellationToken::new();
    let task = tokio::spawn(run(cfg, secrets, known_hosts, sink, cancel.clone()));
    RunningTunnel { cancel, task }
}

async fn run(
    cfg: TunnelConfig,
    secrets: ConnectSecrets,
    known_hosts: PathBuf,
    sink: EventSink,
    cancel: CancellationToken,
) {
    let emitter = Emitter::new(cfg.id, sink);
    let mut attempt: u32 = 0;

    loop {
        emitter.status(if attempt == 0 {
            TunnelState::Connecting
        } else {
            TunnelState::Reconnecting
        });

        let stats = TunnelStats::new();
        let session = match connect(
            &cfg,
            &secrets,
            known_hosts.clone(),
            emitter.clone(),
            stats.clone(),
        )
        .await
        {
            Ok(session) => Arc::new(session),
            Err(e) => {
                emitter.error(format!("Connection failed: {e}"));
                if !schedule_retry(&cfg, &mut attempt, &cancel, &emitter).await {
                    return;
                }
                continue;
            }
        };

        emitter.status(TunnelState::Connected);
        attempt = 0;

        let result = serve(&cfg, session, stats, &emitter, &cancel).await;

        // A user-initiated stop takes precedence over any drop/error.
        if cancel.is_cancelled() {
            emitter.status(TunnelState::Idle);
            return;
        }
        match result {
            Ok(()) => emitter.warn("Tunnel disconnected"),
            Err(e) => emitter.error(format!("Tunnel error: {e}")),
        }
        if !schedule_retry(&cfg, &mut attempt, &cancel, &emitter).await {
            return;
        }
    }
}

/// Run the forwarder with a session-drop watchdog and a stats ticker. Returns
/// when the tunnel is cancelled, the session drops, or the forwarder errors.
async fn serve(
    cfg: &TunnelConfig,
    session: Arc<SshSession>,
    stats: Arc<TunnelStats>,
    emitter: &Emitter,
    cancel: &CancellationToken,
) -> Result<()> {
    // Cancelled by either a user stop (via `cancel`) or a detected disconnect.
    let attempt_cancel = CancellationToken::new();

    let link = {
        let attempt_cancel = attempt_cancel.clone();
        let cancel = cancel.clone();
        tokio::spawn(async move {
            cancel.cancelled().await;
            attempt_cancel.cancel();
        })
    };

    let health = {
        let attempt_cancel = attempt_cancel.clone();
        let session = session.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(HEALTH_INTERVAL);
            loop {
                tokio::select! {
                    _ = attempt_cancel.cancelled() => break,
                    _ = interval.tick() => {
                        if session.is_closed() {
                            attempt_cancel.cancel();
                            break;
                        }
                    }
                }
            }
        })
    };

    let ticker = {
        let emitter = emitter.clone();
        let stats = stats.clone();
        let attempt_cancel = attempt_cancel.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(STATS_INTERVAL);
            loop {
                tokio::select! {
                    _ = attempt_cancel.cancelled() => break,
                    _ = interval.tick() => {
                        emitter.status_with_stats(TunnelState::Connected, stats.snapshot());
                    }
                }
            }
        })
    };

    let result = run_forward(
        cfg,
        session.handle(),
        stats,
        emitter.clone(),
        attempt_cancel,
    )
    .await;

    ticker.abort();
    health.abort();
    link.abort();
    result
}

async fn run_forward(
    cfg: &TunnelConfig,
    handle: Arc<SshHandle>,
    stats: Arc<TunnelStats>,
    emitter: Emitter,
    cancel: CancellationToken,
) -> Result<()> {
    match cfg.kind {
        ForwardKind::Local => {
            let target = cfg
                .target
                .clone()
                .ok_or_else(|| Error::Ssh("local tunnel has no target".into()))?;
            run_local_forward(handle, cfg.listen.clone(), target, stats, emitter, cancel).await
        }
        ForwardKind::Remote => {
            run_remote_forward(handle, cfg.listen.clone(), emitter, cancel).await
        }
        ForwardKind::Dynamic => {
            run_dynamic_forward(handle, cfg.listen.clone(), stats, emitter, cancel).await
        }
    }
}

/// Advance the attempt counter and wait out the backoff. Returns `false` if the
/// tunnel was cancelled or retries are exhausted (the caller should stop).
async fn schedule_retry(
    cfg: &TunnelConfig,
    attempt: &mut u32,
    cancel: &CancellationToken,
    emitter: &Emitter,
) -> bool {
    *attempt += 1;
    if !should_retry(&cfg.reconnect, *attempt) {
        let message = if cfg.reconnect.enabled {
            "Gave up after reconnect attempts"
        } else {
            "Disconnected"
        };
        emitter.status_msg(TunnelState::Error, message);
        return false;
    }

    let delay = backoff_delay(&cfg.reconnect, *attempt);
    emitter.status_msg(
        TunnelState::Reconnecting,
        format!(
            "Reconnecting in {:.0}s (attempt {})",
            delay.as_secs_f64(),
            attempt
        ),
    );

    tokio::select! {
        _ = cancel.cancelled() => {
            emitter.status(TunnelState::Idle);
            false
        }
        _ = tokio::time::sleep(delay) => true,
    }
}
