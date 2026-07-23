use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::error::{Error, Result};
use crate::model::ListenSpec;
use crate::tunnel::event::Emitter;
use crate::tunnel::session::SshHandle;

/// Remote forwarding (`ssh -R`): ask the server to listen on `listen` and send
/// each inbound connection back to us as a `forwarded-tcpip` channel. Those
/// channels are handled by the client handler (see `handler.rs`), which pipes
/// them to the machine-local target. This task just requests the forward and
/// keeps it alive until cancelled.
pub async fn run_remote_forward(
    handle: Arc<SshHandle>,
    listen: ListenSpec,
    emitter: Emitter,
    cancel: CancellationToken,
) -> Result<()> {
    let bound = handle
        .tcpip_forward(listen.bind_address.clone(), listen.port as u32)
        .await
        .map_err(|e| Error::Ssh(format!("remote forward request failed: {e}")))?;
    let port = if listen.port == 0 {
        bound as u16
    } else {
        listen.port
    };
    emitter.info(format!(
        "Server listening on {}:{} → forwarding back to this machine",
        listen.bind_address, port
    ));

    cancel.cancelled().await;

    let _ = handle
        .cancel_tcpip_forward(listen.bind_address.clone(), port as u32)
        .await;
    emitter.info("Remote forward stopped");
    Ok(())
}
