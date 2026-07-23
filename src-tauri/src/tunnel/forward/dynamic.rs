use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::Arc;

use tokio::io::{copy_bidirectional, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::sync::CancellationToken;

use crate::error::{Error, Result};
use crate::model::ListenSpec;
use crate::tunnel::event::Emitter;
use crate::tunnel::session::SshHandle;
use crate::tunnel::stats::{CountingStream, TunnelStats};

const SOCKS5: u8 = 0x05;
const CMD_CONNECT: u8 = 0x01;
const ATYP_IPV4: u8 = 0x01;
const ATYP_DOMAIN: u8 = 0x03;
const ATYP_IPV6: u8 = 0x04;
const REP_SUCCESS: u8 = 0x00;
const REP_GENERAL_FAILURE: u8 = 0x01;
const REP_CMD_NOT_SUPPORTED: u8 = 0x07;

/// Dynamic forwarding (`ssh -D`): a local SOCKS5 proxy. Each SOCKS CONNECT is
/// opened as an SSH `direct-tcpip` channel to the client-requested destination.
pub async fn run_dynamic_forward(
    handle: Arc<SshHandle>,
    listen: ListenSpec,
    stats: Arc<TunnelStats>,
    emitter: Emitter,
    cancel: CancellationToken,
) -> Result<()> {
    let listener = TcpListener::bind((listen.bind_address.as_str(), listen.port))
        .await
        .map_err(|e| {
            Error::Ssh(format!(
                "failed to bind SOCKS proxy {}:{}: {e}",
                listen.bind_address, listen.port
            ))
        })?;
    let local = listener
        .local_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| format!("{}:{}", listen.bind_address, listen.port));
    emitter.info(format!("SOCKS5 proxy listening on {local}"));

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                emitter.info("SOCKS proxy stopped");
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
                let emitter = emitter.clone();
                let cancel = cancel.clone();
                let stats = stats.clone();
                tokio::spawn(async move {
                    if let Err(e) =
                        serve_socks(handle, socket, peer, stats, emitter.clone(), cancel).await
                    {
                        emitter.warn(format!("SOCKS connection error: {e}"));
                    }
                });
            }
        }
    }
}

async fn serve_socks(
    handle: Arc<SshHandle>,
    mut socket: TcpStream,
    peer: std::net::SocketAddr,
    stats: Arc<TunnelStats>,
    emitter: Emitter,
    cancel: CancellationToken,
) -> Result<()> {
    let (host, port) = socks_handshake(&mut socket).await?;

    let channel = match handle
        .channel_open_direct_tcpip(
            host.clone(),
            port as u32,
            peer.ip().to_string(),
            peer.port() as u32,
        )
        .await
    {
        Ok(channel) => channel,
        Err(e) => {
            socks_reply(&mut socket, REP_GENERAL_FAILURE).await?;
            emitter.warn(format!("Could not open channel to {host}:{port}: {e}"));
            return Ok(());
        }
    };

    socks_reply(&mut socket, REP_SUCCESS).await?;

    stats.conn_open();
    let mut socket = CountingStream::new(socket, stats.clone());
    let mut stream = channel.into_stream();
    tokio::select! {
        _ = cancel.cancelled() => {}
        result = copy_bidirectional(&mut socket, &mut stream) => {
            result.map_err(Error::Io)?;
        }
    }
    stats.conn_close();
    Ok(())
}

/// Perform the SOCKS5 greeting + CONNECT request; returns the target host/port.
async fn socks_handshake(socket: &mut TcpStream) -> Result<(String, u16)> {
    // Greeting: [version, nmethods, methods...]
    let mut head = [0u8; 2];
    socket.read_exact(&mut head).await?;
    if head[0] != SOCKS5 {
        return Err(Error::Ssh("not a SOCKS5 client".into()));
    }
    let mut methods = vec![0u8; head[1] as usize];
    socket.read_exact(&mut methods).await?;
    // Reply: no authentication required.
    socket.write_all(&[SOCKS5, 0x00]).await?;

    // Request: [version, cmd, reserved, atyp, addr..., port(2)]
    let mut req = [0u8; 4];
    socket.read_exact(&mut req).await?;
    if req[0] != SOCKS5 {
        return Err(Error::Ssh("bad SOCKS5 request".into()));
    }
    if req[1] != CMD_CONNECT {
        socks_reply(socket, REP_CMD_NOT_SUPPORTED).await?;
        return Err(Error::Ssh("only SOCKS5 CONNECT is supported".into()));
    }

    let host = match req[3] {
        ATYP_IPV4 => {
            let mut addr = [0u8; 4];
            socket.read_exact(&mut addr).await?;
            Ipv4Addr::from(addr).to_string()
        }
        ATYP_DOMAIN => {
            let mut len = [0u8; 1];
            socket.read_exact(&mut len).await?;
            let mut domain = vec![0u8; len[0] as usize];
            socket.read_exact(&mut domain).await?;
            String::from_utf8(domain).map_err(|_| Error::Ssh("invalid SOCKS5 domain".into()))?
        }
        ATYP_IPV6 => {
            let mut addr = [0u8; 16];
            socket.read_exact(&mut addr).await?;
            Ipv6Addr::from(addr).to_string()
        }
        other => {
            return Err(Error::Ssh(format!(
                "unsupported SOCKS5 address type {other}"
            )))
        }
    };

    let mut port = [0u8; 2];
    socket.read_exact(&mut port).await?;
    Ok((host, u16::from_be_bytes(port)))
}

/// Send a SOCKS5 reply with the given reply code and a null bound address.
async fn socks_reply(socket: &mut TcpStream, rep: u8) -> Result<()> {
    socket
        .write_all(&[SOCKS5, rep, 0x00, ATYP_IPV4, 0, 0, 0, 0, 0, 0])
        .await?;
    Ok(())
}
