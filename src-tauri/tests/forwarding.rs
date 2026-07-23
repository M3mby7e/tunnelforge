//! End-to-end tests: bytes must round-trip through real tunnels of every mode
//! (local `-L`, dynamic `-D`/SOCKS5, and remote `-R`), driven against an
//! in-process russh server that proxies direct-tcpip and reverse forwards.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use russh::server::{self, Auth, ChannelOpenHandle, Msg, Server as _, Session};
use russh::Channel;
use tokio::io::{copy_bidirectional, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::unbounded_channel;
use tokio::time::{sleep, timeout};
use uuid::Uuid;

use tunnelium_lib::model::{
    AuthConfig, ForwardKind, ForwardTarget, JumpHost, ListenSpec, ReconnectPolicy, SshEndpoint,
    TunnelConfig,
};
use tunnelium_lib::tunnel::event::EngineEvent;
use tunnelium_lib::tunnel::runtime;
use tunnelium_lib::tunnel::session::ConnectSecrets;

// --- In-process SSH server that accepts any auth and proxies direct-tcpip ---

#[derive(Clone)]
struct TestServer;

impl server::Server for TestServer {
    type Handler = TestHandler;
    fn new_client(&mut self, _peer: Option<SocketAddr>) -> TestHandler {
        TestHandler
    }
}

struct TestHandler;

impl server::Handler for TestHandler {
    type Error = russh::Error;

    async fn auth_password(&mut self, _user: &str, _password: &str) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn auth_publickey(
        &mut self,
        _user: &str,
        _key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn channel_open_direct_tcpip(
        &mut self,
        channel: Channel<Msg>,
        host_to_connect: &str,
        port_to_connect: u32,
        _originator_address: &str,
        _originator_port: u32,
        reply: ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        reply.accept().await;
        let addr = format!("{host_to_connect}:{port_to_connect}");
        tokio::spawn(async move {
            if let Ok(mut target) = TcpStream::connect(addr).await {
                let mut stream = channel.into_stream();
                let _ = copy_bidirectional(&mut stream, &mut target).await;
            }
        });
        Ok(())
    }

    async fn tcpip_forward(
        &mut self,
        address: &str,
        port: &mut u32,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        let handle = session.handle();
        let bind = format!("{address}:{port}");
        let address = address.to_string();
        let bind_port = *port;
        tokio::spawn(async move {
            let Ok(listener) = TcpListener::bind(bind).await else {
                return;
            };
            while let Ok((mut inbound, peer)) = listener.accept().await {
                let handle = handle.clone();
                let address = address.clone();
                tokio::spawn(async move {
                    if let Ok(channel) = handle
                        .channel_open_forwarded_tcpip(
                            address,
                            bind_port,
                            peer.ip().to_string(),
                            peer.port() as u32,
                        )
                        .await
                    {
                        let mut stream = channel.into_stream();
                        let _ = copy_bidirectional(&mut inbound, &mut stream).await;
                    }
                });
            }
        });
        Ok(true)
    }
}

async fn spawn_echo_server() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                loop {
                    match socket.read(&mut buf).await {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            if socket.write_all(&buf[..n]).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            });
        }
    });
    addr
}

async fn spawn_ssh_server() -> SocketAddr {
    let key =
        russh::keys::PrivateKey::random(&mut rand::rng(), russh::keys::Algorithm::Ed25519).unwrap();
    let config = Arc::new(server::Config {
        keys: vec![key],
        ..Default::default()
    });
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let mut server = TestServer;
        let _ = server.run_on_socket(config, &listener).await;
    });
    addr
}

async fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

fn make_tunnel(
    kind: ForwardKind,
    ssh_port: u16,
    listen_port: u16,
    target: Option<SocketAddr>,
) -> TunnelConfig {
    let now = Utc::now();
    TunnelConfig {
        id: Uuid::new_v4(),
        name: "e2e".into(),
        description: None,
        kind,
        enabled: true,
        auto_start: false,
        reconnect: ReconnectPolicy::default(),
        ssh: SshEndpoint {
            host: "127.0.0.1".into(),
            port: ssh_port,
            username: "tester".into(),
        },
        auth: AuthConfig::Password {
            secret_ref: "e2e".into(),
        },
        listen: ListenSpec {
            bind_address: "127.0.0.1".into(),
            port: listen_port,
        },
        target: target.map(|addr| ForwardTarget {
            host: addr.ip().to_string(),
            port: addr.port(),
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

fn secrets() -> ConnectSecrets {
    ConnectSecrets {
        password: Some("pw".into()),
        ..ConnectSecrets::default()
    }
}

/// Open a SOCKS5 CONNECT through the local dynamic proxy to `dest`.
async fn socks5_connect(proxy_port: u16, dest: SocketAddr) -> std::io::Result<TcpStream> {
    let mut socket = TcpStream::connect(("127.0.0.1", proxy_port)).await?;
    socket.write_all(&[0x05, 0x01, 0x00]).await?; // greeting: no-auth
    let mut greeting = [0u8; 2];
    socket.read_exact(&mut greeting).await?;
    let IpAddr::V4(ip) = dest.ip() else {
        panic!("test uses an IPv4 target");
    };
    let mut request = vec![0x05, 0x01, 0x00, 0x01];
    request.extend_from_slice(&ip.octets());
    request.extend_from_slice(&dest.port().to_be_bytes());
    socket.write_all(&request).await?;
    let mut reply = [0u8; 10];
    socket.read_exact(&mut reply).await?;
    assert_eq!(reply[1], 0x00, "SOCKS5 CONNECT should succeed");
    Ok(socket)
}

#[tokio::test]
async fn bytes_round_trip_through_local_forward() {
    let echo = spawn_echo_server().await;
    let ssh = spawn_ssh_server().await;
    let listen_port = free_port().await;

    let cfg = make_tunnel(ForwardKind::Local, ssh.port(), listen_port, Some(echo));
    let (tx, _rx) = unbounded_channel::<EngineEvent>();
    let tmp = tempfile::tempdir().unwrap();
    let running = runtime::spawn(cfg, secrets(), tmp.path().join("known_hosts"), tx);

    let payload = b"hello through the tunnel";
    let echoed = timeout(Duration::from_secs(10), async {
        loop {
            if let Ok(mut client) = TcpStream::connect(("127.0.0.1", listen_port)).await {
                if client.write_all(payload).await.is_ok() {
                    let mut buf = vec![0u8; payload.len()];
                    if client.read_exact(&mut buf).await.is_ok() {
                        return buf;
                    }
                }
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("tunnel did not echo within the timeout");

    assert_eq!(&echoed, payload, "payload must round-trip unchanged");
    running.stop().await;
}

#[tokio::test]
async fn bytes_round_trip_through_dynamic_socks_forward() {
    let echo = spawn_echo_server().await;
    let ssh = spawn_ssh_server().await;
    let socks_port = free_port().await;

    let cfg = make_tunnel(ForwardKind::Dynamic, ssh.port(), socks_port, None);
    let (tx, _rx) = unbounded_channel::<EngineEvent>();
    let tmp = tempfile::tempdir().unwrap();
    let running = runtime::spawn(cfg, secrets(), tmp.path().join("known_hosts"), tx);

    let payload = b"hello via socks5";
    let echoed = timeout(Duration::from_secs(10), async {
        loop {
            if let Ok(mut client) = socks5_connect(socks_port, echo).await {
                if client.write_all(payload).await.is_ok() {
                    let mut buf = vec![0u8; payload.len()];
                    if client.read_exact(&mut buf).await.is_ok() {
                        return buf;
                    }
                }
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("SOCKS tunnel did not echo within the timeout");

    assert_eq!(&echoed, payload, "payload must round-trip unchanged");
    running.stop().await;
}

#[tokio::test]
async fn bytes_round_trip_through_remote_forward() {
    let echo = spawn_echo_server().await; // machine-local target
    let ssh = spawn_ssh_server().await;
    let server_port = free_port().await; // where the server listens

    let cfg = make_tunnel(ForwardKind::Remote, ssh.port(), server_port, Some(echo));
    let (tx, _rx) = unbounded_channel::<EngineEvent>();
    let tmp = tempfile::tempdir().unwrap();
    let running = runtime::spawn(cfg, secrets(), tmp.path().join("known_hosts"), tx);

    let payload = b"hello back through -R";
    let echoed = timeout(Duration::from_secs(10), async {
        loop {
            if let Ok(mut client) = TcpStream::connect(("127.0.0.1", server_port)).await {
                if client.write_all(payload).await.is_ok() {
                    let mut buf = vec![0u8; payload.len()];
                    if client.read_exact(&mut buf).await.is_ok() {
                        return buf;
                    }
                }
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("remote-forward tunnel did not echo within the timeout");

    assert_eq!(&echoed, payload, "payload must round-trip unchanged");
    running.stop().await;
}

#[tokio::test]
async fn bytes_round_trip_through_jump_host() {
    let echo = spawn_echo_server().await;
    let target_ssh = spawn_ssh_server().await;
    let jump_ssh = spawn_ssh_server().await;
    let listen_port = free_port().await;

    let mut cfg = make_tunnel(
        ForwardKind::Local,
        target_ssh.port(),
        listen_port,
        Some(echo),
    );
    cfg.jump_hosts = vec![JumpHost {
        endpoint: SshEndpoint {
            host: "127.0.0.1".into(),
            port: jump_ssh.port(),
            username: "tester".into(),
        },
        auth: AuthConfig::Password {
            secret_ref: "jump".into(),
        },
    }];
    let mut connect_secrets = secrets();
    connect_secrets.jumps = vec![secrets()];

    let (tx, _rx) = unbounded_channel::<EngineEvent>();
    let tmp = tempfile::tempdir().unwrap();
    let running = runtime::spawn(cfg, connect_secrets, tmp.path().join("known_hosts"), tx);

    let payload = b"hello through a jump host";
    let echoed = timeout(Duration::from_secs(10), async {
        loop {
            if let Ok(mut client) = TcpStream::connect(("127.0.0.1", listen_port)).await {
                if client.write_all(payload).await.is_ok() {
                    let mut buf = vec![0u8; payload.len()];
                    if client.read_exact(&mut buf).await.is_ok() {
                        return buf;
                    }
                }
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("jump-host tunnel did not echo within the timeout");

    assert_eq!(&echoed, payload, "payload must round-trip unchanged");
    running.stop().await;
}
