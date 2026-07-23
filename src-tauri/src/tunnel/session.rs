use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use russh::client::{self, Config, Handle, KeyboardInteractiveAuthResponse, Msg};
use russh::keys::ssh_key::HashAlg;
use russh::keys::{load_secret_key, PrivateKeyWithHashAlg};
use russh::ChannelStream;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;

use crate::error::{Error, Result};
use crate::model::{AuthConfig, ForwardKind, ProxyConfig, TunnelConfig};
use crate::tunnel::event::Emitter;
use crate::tunnel::handler::HostKeyHandler;
use crate::tunnel::proxy::{self, ProxyAuth};
use crate::tunnel::stats::TunnelStats;

/// Secrets resolved from the OS keychain, passed in so the engine core stays
/// decoupled from keychain access (and therefore testable).
#[derive(Debug, Default, Clone)]
pub struct ConnectSecrets {
    pub password: Option<String>,
    pub passphrase: Option<String>,
    pub proxy_auth: ProxyAuth,
    /// One entry per jump host, in order.
    pub jumps: Vec<ConnectSecrets>,
}

pub type SshHandle = Handle<HostKeyHandler>;

/// A live SSH session to the target, plus any jump-host sessions that must stay
/// alive to keep the chained transport open.
pub struct SshSession {
    handle: Arc<SshHandle>,
    jumps: Vec<SshHandle>,
}

impl SshSession {
    pub fn handle(&self) -> Arc<SshHandle> {
        self.handle.clone()
    }

    /// True if the target session or any jump hop has dropped.
    pub fn is_closed(&self) -> bool {
        self.handle.is_closed() || self.jumps.iter().any(|h| h.is_closed())
    }
}

/// The raw transport under an SSH session: a direct/proxied TCP socket, or a
/// channel stream through the previous jump hop.
enum Transport {
    Tcp(TcpStream),
    Channel(ChannelStream<Msg>),
}

impl AsyncRead for Transport {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Transport::Tcp(s) => Pin::new(s).poll_read(cx, buf),
            Transport::Channel(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for Transport {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        data: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            Transport::Tcp(s) => Pin::new(s).poll_write(cx, data),
            Transport::Channel(s) => Pin::new(s).poll_write(cx, data),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Transport::Tcp(s) => Pin::new(s).poll_flush(cx),
            Transport::Channel(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Transport::Tcp(s) => Pin::new(s).poll_shutdown(cx),
            Transport::Channel(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
}

/// Establish an authenticated SSH session to the tunnel's server, hopping
/// through any configured proxy and jump hosts.
pub async fn connect(
    cfg: &TunnelConfig,
    secrets: &ConnectSecrets,
    known_hosts: PathBuf,
    emitter: Emitter,
    stats: Arc<TunnelStats>,
) -> Result<SshSession> {
    let config = client_config(cfg);
    emitter.info(format!("Connecting to {}:{}", cfg.ssh.host, cfg.ssh.port));

    let (transport, jumps) =
        build_chain(cfg, secrets, &known_hosts, &emitter, &stats, &config).await?;

    let handler = HostKeyHandler {
        host: cfg.ssh.host.clone(),
        port: cfg.ssh.port,
        known_hosts,
        emitter: emitter.clone(),
        remote_target: match cfg.kind {
            ForwardKind::Remote => cfg.target.clone(),
            _ => None,
        },
        stats,
    };

    let mut handle = client::connect_stream(config, transport, handler).await?;
    authenticate(&mut handle, &cfg.ssh.username, &cfg.auth, secrets).await?;
    emitter.info("Authenticated");

    Ok(SshSession {
        handle: Arc::new(handle),
        jumps,
    })
}

fn client_config(cfg: &TunnelConfig) -> Arc<Config> {
    let mut config = Config::default();
    if let Some(secs) = cfg.keep_alive_seconds {
        if secs > 0 {
            config.keepalive_interval = Some(Duration::from_secs(secs as u64));
        }
    }
    if let Some(ms) = cfg.connect_timeout_ms {
        config.inactivity_timeout = Some(Duration::from_millis(ms as u64));
    }
    Arc::new(config)
}

/// Build the transport to the target, chaining through jump hosts. Returns the
/// final transport plus the jump-host session handles (which must be kept alive).
async fn build_chain(
    cfg: &TunnelConfig,
    secrets: &ConnectSecrets,
    known_hosts: &Path,
    emitter: &Emitter,
    stats: &Arc<TunnelStats>,
    config: &Arc<Config>,
) -> Result<(Transport, Vec<SshHandle>)> {
    // The first TCP connection goes to the first jump host (or the target if
    // there are no jumps), optionally through the proxy.
    let first = cfg
        .jump_hosts
        .first()
        .map(|j| &j.endpoint)
        .unwrap_or(&cfg.ssh);
    let mut transport = Transport::Tcp(
        open_tcp(
            cfg.proxy.as_ref(),
            &first.host,
            first.port,
            &secrets.proxy_auth,
            emitter,
        )
        .await?,
    );

    let mut handles = Vec::new();
    for (i, jump) in cfg.jump_hosts.iter().enumerate() {
        let jump_secrets = secrets.jumps.get(i).cloned().unwrap_or_default();
        emitter.info(format!(
            "Hopping through jump host {}:{}",
            jump.endpoint.host, jump.endpoint.port
        ));

        let handler = HostKeyHandler {
            host: jump.endpoint.host.clone(),
            port: jump.endpoint.port,
            known_hosts: known_hosts.to_path_buf(),
            emitter: emitter.clone(),
            remote_target: None,
            stats: stats.clone(),
        };
        let mut hop = client::connect_stream(config.clone(), transport, handler).await?;
        authenticate(&mut hop, &jump.endpoint.username, &jump.auth, &jump_secrets).await?;

        // Open a channel from this hop to the next hop (or the target).
        let next = cfg
            .jump_hosts
            .get(i + 1)
            .map(|j| &j.endpoint)
            .unwrap_or(&cfg.ssh);
        let channel = hop
            .channel_open_direct_tcpip(next.host.clone(), next.port as u32, "127.0.0.1", 0)
            .await?;
        transport = Transport::Channel(channel.into_stream());
        handles.push(hop);
    }

    Ok((transport, handles))
}

async fn open_tcp(
    proxy: Option<&ProxyConfig>,
    host: &str,
    port: u16,
    proxy_auth: &ProxyAuth,
    emitter: &Emitter,
) -> Result<TcpStream> {
    match proxy {
        Some(proxy) => {
            emitter.info(format!(
                "Connecting via {:?} proxy {}:{}",
                proxy.kind, proxy.host, proxy.port
            ));
            proxy::dial_via_proxy(proxy, host, port, proxy_auth.clone()).await
        }
        None => TcpStream::connect((host, port))
            .await
            .map_err(|e| Error::Ssh(format!("could not connect to {host}:{port}: {e}"))),
    }
}

async fn authenticate(
    handle: &mut SshHandle,
    user: &str,
    auth: &AuthConfig,
    secrets: &ConnectSecrets,
) -> Result<()> {
    match auth {
        AuthConfig::PrivateKey { key_path, .. } => {
            let key = load_secret_key(Path::new(key_path), secrets.passphrase.as_deref())?;
            let hash = if key.algorithm().is_rsa() {
                Some(HashAlg::Sha512)
            } else {
                None
            };
            let key = PrivateKeyWithHashAlg::new(Arc::new(key), hash);
            succeeded(handle.authenticate_publickey(user.to_string(), key).await?)
        }
        AuthConfig::Password { .. } => {
            let password = secrets.password.clone().ok_or(Error::AuthFailed)?;
            succeeded(
                handle
                    .authenticate_password(user.to_string(), password)
                    .await?,
            )
        }
        AuthConfig::Agent => authenticate_agent(handle, user.to_string()).await,
        AuthConfig::KeyboardInteractive { .. } => {
            authenticate_keyboard_interactive(handle, user.to_string(), secrets).await
        }
        AuthConfig::PrivateKeyInline { .. } => Err(Error::Unsupported(
            "Imported-key authentication is not yet supported".into(),
        )),
    }
}

fn succeeded(result: russh::client::AuthResult) -> Result<()> {
    if result.success() {
        Ok(())
    } else {
        Err(Error::AuthFailed)
    }
}

/// Try each identity offered by the running SSH agent until one is accepted.
/// The agent client uses `SSH_AUTH_SOCK`, which only exists on Unix.
#[cfg(unix)]
async fn authenticate_agent(handle: &mut SshHandle, user: String) -> Result<()> {
    use russh::keys::agent::client::AgentClient;
    use russh::keys::agent::AgentIdentity;

    let mut agent = AgentClient::connect_env()
        .await
        .map_err(|e| Error::Ssh(format!("could not reach the SSH agent: {e}")))?;
    let identities = agent
        .request_identities()
        .await
        .map_err(|e| Error::Ssh(format!("SSH agent error: {e}")))?;
    if identities.is_empty() {
        return Err(Error::Ssh("the SSH agent has no identities loaded".into()));
    }

    for identity in identities {
        if let AgentIdentity::PublicKey { key, .. } = identity {
            let result = handle
                .authenticate_publickey_with(user.clone(), key, None, &mut agent)
                .await
                .map_err(|e| Error::Ssh(format!("agent authentication failed: {e}")))?;
            if result.success() {
                return Ok(());
            }
        }
    }
    Err(Error::AuthFailed)
}

#[cfg(not(unix))]
async fn authenticate_agent(_handle: &mut SshHandle, _user: String) -> Result<()> {
    Err(Error::Unsupported(
        "SSH agent authentication is only available on macOS/Linux; use a key or password".into(),
    ))
}

/// Answer keyboard-interactive prompts with the stored secret (handles the
/// common password-over-keyboard-interactive case; live OTP prompts would need
/// an interactive UI and are not yet supported).
async fn authenticate_keyboard_interactive(
    handle: &mut SshHandle,
    user: String,
    secrets: &ConnectSecrets,
) -> Result<()> {
    let secret = secrets.password.clone().unwrap_or_default();
    let mut response = handle
        .authenticate_keyboard_interactive_start(user, None::<String>)
        .await?;
    loop {
        match response {
            KeyboardInteractiveAuthResponse::Success => return Ok(()),
            KeyboardInteractiveAuthResponse::Failure { .. } => return Err(Error::AuthFailed),
            KeyboardInteractiveAuthResponse::InfoRequest { prompts, .. } => {
                let answers = prompts.iter().map(|_| secret.clone()).collect();
                response = handle
                    .authenticate_keyboard_interactive_respond(answers)
                    .await?;
            }
        }
    }
}
