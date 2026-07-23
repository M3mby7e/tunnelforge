use std::path::PathBuf;
use std::sync::Arc;

use russh::client::{self, ChannelOpenHandle, Msg};
use russh::keys::known_hosts::{check_known_hosts_path, learn_known_hosts_path};
use russh::keys::ssh_key::{HashAlg, PublicKey};
use russh::keys::Error as KeysError;
use russh::Channel;
use tokio::io::copy_bidirectional;
use tokio::net::TcpStream;

use crate::model::ForwardTarget;
use crate::tunnel::event::Emitter;
use crate::tunnel::stats::{CountingStream, TunnelStats};

/// russh client handler: host-key verification (trust-on-first-use) plus, for
/// remote (`-R`) tunnels, handling the `forwarded-tcpip` channels the server
/// opens back to us by connecting them to the machine-local target.
pub struct HostKeyHandler {
    pub host: String,
    pub port: u16,
    pub known_hosts: PathBuf,
    pub emitter: Emitter,
    /// Local destination for remote-forwarded connections (`-R` only).
    pub remote_target: Option<ForwardTarget>,
    pub stats: Arc<TunnelStats>,
}

impl HostKeyHandler {
    fn ensure_known_hosts_file(&self) {
        if let Some(parent) = self.known_hosts.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if !self.known_hosts.exists() {
            let _ = std::fs::write(&self.known_hosts, b"");
        }
    }
}

impl client::Handler for HostKeyHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        let fingerprint = server_public_key.fingerprint(HashAlg::Sha256).to_string();
        self.ensure_known_hosts_file();

        match check_known_hosts_path(&self.host, self.port, server_public_key, &self.known_hosts) {
            Ok(true) => {
                self.emitter
                    .info(format!("Host key verified ({fingerprint})"));
                Ok(true)
            }
            Ok(false) => {
                // Trust on first use: record the key and continue.
                if let Err(e) = learn_known_hosts_path(
                    &self.host,
                    self.port,
                    server_public_key,
                    &self.known_hosts,
                ) {
                    self.emitter.warn(format!("Could not record host key: {e}"));
                }
                self.emitter
                    .warn(format!("New host key trusted on first use ({fingerprint})"));
                Ok(true)
            }
            Err(KeysError::KeyChanged { line }) => {
                self.emitter.error(format!(
                    "HOST KEY CHANGED (known_hosts line {line}) — refusing to connect ({fingerprint})"
                ));
                Ok(false)
            }
            Err(e) => {
                self.emitter
                    .error(format!("Host key check failed: {e} — refusing"));
                Ok(false)
            }
        }
    }

    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: Channel<Msg>,
        _connected_address: &str,
        _connected_port: u32,
        originator_address: &str,
        originator_port: u32,
        reply: ChannelOpenHandle,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        let Some(target) = self.remote_target.clone() else {
            // No local target configured; reject the channel.
            reply
                .reject(russh::ChannelOpenFailure::AdministrativelyProhibited)
                .await;
            return Ok(());
        };
        reply.accept().await;

        let emitter = self.emitter.clone();
        let stats = self.stats.clone();
        let origin = format!("{originator_address}:{originator_port}");
        tokio::spawn(async move {
            match TcpStream::connect((target.host.as_str(), target.port)).await {
                Ok(local) => {
                    stats.conn_open();
                    let mut local = CountingStream::new(local, stats.clone());
                    let mut stream = channel.into_stream();
                    if let Err(e) = copy_bidirectional(&mut stream, &mut local).await {
                        emitter.warn(format!("Remote-forwarded conn from {origin} ended: {e}"));
                    }
                    stats.conn_close();
                }
                Err(e) => emitter.warn(format!(
                    "Could not reach local target {}:{}: {e}",
                    target.host, target.port
                )),
            }
        });
        Ok(())
    }
}
