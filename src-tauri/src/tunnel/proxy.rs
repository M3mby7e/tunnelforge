use base64ish::encode as base64_encode;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::error::{Error, Result};
use crate::model::{ProxyConfig, ProxyKind};

/// Optional `(username, password)` for proxy authentication.
pub type ProxyAuth = Option<(String, String)>;

/// Open a TCP stream to `host:port` tunnelled through the given proxy.
pub async fn dial_via_proxy(
    proxy: &ProxyConfig,
    host: &str,
    port: u16,
    auth: ProxyAuth,
) -> Result<TcpStream> {
    let mut stream = TcpStream::connect((proxy.host.as_str(), proxy.port))
        .await
        .map_err(|e| {
            Error::Ssh(format!(
                "could not reach proxy {}:{}: {e}",
                proxy.host, proxy.port
            ))
        })?;

    match proxy.kind {
        ProxyKind::Http => http_connect(&mut stream, host, port, &auth).await?,
        ProxyKind::Socks5 => socks5_connect(&mut stream, host, port, &auth).await?,
    }
    Ok(stream)
}

async fn http_connect(
    stream: &mut TcpStream,
    host: &str,
    port: u16,
    auth: &ProxyAuth,
) -> Result<()> {
    let mut request = format!("CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\n");
    if let Some((user, pass)) = auth {
        let token = base64_encode(format!("{user}:{pass}").as_bytes());
        request.push_str(&format!("Proxy-Authorization: Basic {token}\r\n"));
    }
    request.push_str("\r\n");
    stream.write_all(request.as_bytes()).await?;

    // Read the status line + headers up to the blank line.
    let mut buf = Vec::with_capacity(256);
    let mut byte = [0u8; 1];
    loop {
        let n = stream.read(&mut byte).await?;
        if n == 0 {
            return Err(Error::Ssh("proxy closed the connection".into()));
        }
        buf.push(byte[0]);
        if buf.ends_with(b"\r\n\r\n") {
            break;
        }
        if buf.len() > 8192 {
            return Err(Error::Ssh("proxy response too large".into()));
        }
    }

    let head = String::from_utf8_lossy(&buf);
    let status_ok = head
        .lines()
        .next()
        .map(|line| line.contains(" 200"))
        .unwrap_or(false);
    if !status_ok {
        let first = head.lines().next().unwrap_or("").trim();
        return Err(Error::Ssh(format!("HTTP proxy CONNECT failed: {first}")));
    }
    Ok(())
}

async fn socks5_connect(
    stream: &mut TcpStream,
    host: &str,
    port: u16,
    auth: &ProxyAuth,
) -> Result<()> {
    // Greeting: offer no-auth and (if configured) username/password.
    if auth.is_some() {
        stream.write_all(&[0x05, 0x02, 0x00, 0x02]).await?;
    } else {
        stream.write_all(&[0x05, 0x01, 0x00]).await?;
    }
    let mut choice = [0u8; 2];
    stream.read_exact(&mut choice).await?;
    if choice[0] != 0x05 {
        return Err(Error::Ssh("not a SOCKS5 proxy".into()));
    }
    match choice[1] {
        0x00 => {}
        0x02 => socks5_userpass(stream, auth).await?,
        0xff => return Err(Error::Ssh("SOCKS5 proxy rejected our auth methods".into())),
        other => {
            return Err(Error::Ssh(format!(
                "unsupported SOCKS5 auth method {other}"
            )))
        }
    }

    // CONNECT request with a domain-name target (proxy resolves it).
    let host_bytes = host.as_bytes();
    if host_bytes.len() > 255 {
        return Err(Error::Ssh("SOCKS5 hostname too long".into()));
    }
    let mut request = vec![0x05, 0x01, 0x00, 0x03, host_bytes.len() as u8];
    request.extend_from_slice(host_bytes);
    request.extend_from_slice(&port.to_be_bytes());
    stream.write_all(&request).await?;

    // Reply: version, rep, rsv, atyp, bound addr, bound port.
    let mut head = [0u8; 4];
    stream.read_exact(&mut head).await?;
    if head[1] != 0x00 {
        return Err(Error::Ssh(format!(
            "SOCKS5 proxy CONNECT failed (code {})",
            head[1]
        )));
    }
    let addr_len = match head[3] {
        0x01 => 4,
        0x04 => 16,
        0x03 => {
            let mut len = [0u8; 1];
            stream.read_exact(&mut len).await?;
            len[0] as usize
        }
        other => return Err(Error::Ssh(format!("bad SOCKS5 reply atyp {other}"))),
    };
    let mut rest = vec![0u8; addr_len + 2];
    stream.read_exact(&mut rest).await?;
    Ok(())
}

async fn socks5_userpass(stream: &mut TcpStream, auth: &ProxyAuth) -> Result<()> {
    let (user, pass) = auth
        .as_ref()
        .ok_or_else(|| Error::Ssh("proxy requires credentials".into()))?;
    let mut msg = vec![0x01, user.len() as u8];
    msg.extend_from_slice(user.as_bytes());
    msg.push(pass.len() as u8);
    msg.extend_from_slice(pass.as_bytes());
    stream.write_all(&msg).await?;

    let mut reply = [0u8; 2];
    stream.read_exact(&mut reply).await?;
    if reply[1] != 0x00 {
        return Err(Error::Ssh("SOCKS5 proxy authentication failed".into()));
    }
    Ok(())
}

/// Minimal, dependency-free base64 (standard alphabet, padded).
mod base64ish {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    pub fn encode(input: &[u8]) -> String {
        let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
        for chunk in input.chunks(3) {
            let b0 = chunk[0] as u32;
            let b1 = *chunk.get(1).unwrap_or(&0) as u32;
            let b2 = *chunk.get(2).unwrap_or(&0) as u32;
            let n = (b0 << 16) | (b1 << 8) | b2;
            out.push(ALPHABET[(n >> 18 & 63) as usize] as char);
            out.push(ALPHABET[(n >> 12 & 63) as usize] as char);
            out.push(if chunk.len() > 1 {
                ALPHABET[(n >> 6 & 63) as usize] as char
            } else {
                '='
            });
            out.push(if chunk.len() > 2 {
                ALPHABET[(n & 63) as usize] as char
            } else {
                '='
            });
        }
        out
    }
}
