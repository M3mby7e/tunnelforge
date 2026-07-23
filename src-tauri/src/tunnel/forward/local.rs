use std::sync::Arc;

use tokio::io::copy_bidirectional;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use crate::error::{Error, Result};
use crate::model::{ForwardTarget, ListenSpec};
use crate::tunnel::event::Emitter;
use crate::tunnel::session::SshHandle;
use crate::tunnel::stats::{CountingStream, TunnelStats};

/// Local forwarding (`ssh -L`): accept TCP connections on a local listener and
/// forward each through an SSH `direct-tcpip` channel to the target reachable
/// from the server.
pub async fn run_local_forward(
    handle: Arc<SshHandle>,
    listen: ListenSpec,
    target: ForwardTarget,
    stats: Arc<TunnelStats>,
    emitter: Emitter,
    cancel: CancellationToken,
) -> Result<()> {
    let listener = TcpListener::bind((listen.bind_address.as_str(), listen.port))
        .await
        .map_err(|e| {
            Error::Ssh(format!(
                "failed to bind {}:{}: {e}",
                listen.bind_address, listen.port
            ))
        })?;

    let local = listener
        .local_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| format!("{}:{}", listen.bind_address, listen.port));
    emitter.info(format!(
        "Listening on {local} → {}:{}",
        target.host, target.port
    ));

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                emitter.info("Listener stopped");
                return Ok(());
            }
            accept = listener.accept() => {
                let (socket, peer) = match accept {
                    Ok(pair) => pair,
                    Err(e) => {
                        emitter.warn(format!("Accept error: {e}"));
                        continue;
                    }
                };
                let handle = handle.clone();
                let target = target.clone();
                let emitter = emitter.clone();
                let cancel = cancel.clone();
                let stats = stats.clone();
                tokio::spawn(async move {
                    forward_connection(handle, socket, peer, target, stats, emitter, cancel)
                        .await;
                });
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn forward_connection(
    handle: Arc<SshHandle>,
    socket: tokio::net::TcpStream,
    peer: std::net::SocketAddr,
    target: ForwardTarget,
    stats: Arc<TunnelStats>,
    emitter: Emitter,
    cancel: CancellationToken,
) {
    let channel = match handle
        .channel_open_direct_tcpip(
            target.host.clone(),
            target.port as u32,
            peer.ip().to_string(),
            peer.port() as u32,
        )
        .await
    {
        Ok(channel) => channel,
        Err(e) => {
            emitter.warn(format!("Could not open channel to {}: {e}", target.host));
            return;
        }
    };

    stats.conn_open();
    let mut socket = CountingStream::new(socket, stats.clone());
    let mut stream = channel.into_stream();
    tokio::select! {
        _ = cancel.cancelled() => {}
        result = copy_bidirectional(&mut socket, &mut stream) => {
            if let Err(e) = result {
                emitter.warn(format!("Connection from {peer} ended: {e}"));
            }
        }
    }
    stats.conn_close();
}
