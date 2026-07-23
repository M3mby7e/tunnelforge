use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::model::StatsSnapshot;

/// Live, thread-safe counters for one tunnel.
///
/// Convention: **up** = bytes the local side sends into the tunnel, **down** =
/// bytes received from the tunnel. This is measured by wrapping the local
/// (non-SSH) socket in a [`CountingStream`], so a read from it counts as up and
/// a write to it counts as down.
#[derive(Debug)]
pub struct TunnelStats {
    bytes_up: AtomicU64,
    bytes_down: AtomicU64,
    active: AtomicU32,
    started: Instant,
}

impl TunnelStats {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            bytes_up: AtomicU64::new(0),
            bytes_down: AtomicU64::new(0),
            active: AtomicU32::new(0),
            started: Instant::now(),
        })
    }

    fn add_up(&self, n: u64) {
        self.bytes_up.fetch_add(n, Ordering::Relaxed);
    }

    fn add_down(&self, n: u64) {
        self.bytes_down.fetch_add(n, Ordering::Relaxed);
    }

    pub fn conn_open(&self) {
        self.active.fetch_add(1, Ordering::Relaxed);
    }

    pub fn conn_close(&self) {
        self.active.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> StatsSnapshot {
        StatsSnapshot {
            bytes_up: self.bytes_up.load(Ordering::Relaxed),
            bytes_down: self.bytes_down.load(Ordering::Relaxed),
            active_connections: self.active.load(Ordering::Relaxed),
            uptime_seconds: self.started.elapsed().as_secs(),
            retry_count: 0,
        }
    }
}

/// Wraps a local socket so bytes read/written update a [`TunnelStats`].
pub struct CountingStream<S> {
    inner: S,
    stats: Arc<TunnelStats>,
}

impl<S> CountingStream<S> {
    pub fn new(inner: S, stats: Arc<TunnelStats>) -> Self {
        Self { inner, stats }
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for CountingStream<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let before = buf.filled().len();
        let result = Pin::new(&mut self.inner).poll_read(cx, buf);
        if let Poll::Ready(Ok(())) = &result {
            let read = buf.filled().len() - before;
            self.stats.add_up(read as u64);
        }
        result
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for CountingStream<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        data: &[u8],
    ) -> Poll<io::Result<usize>> {
        let result = Pin::new(&mut self.inner).poll_write(cx, data);
        if let Poll::Ready(Ok(n)) = &result {
            self.stats.add_down(*n as u64);
        }
        result
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_reflects_counters() {
        let stats = TunnelStats::new();
        stats.add_up(100);
        stats.add_down(250);
        stats.conn_open();
        stats.conn_open();
        stats.conn_close();

        let snap = stats.snapshot();
        assert_eq!(snap.bytes_up, 100);
        assert_eq!(snap.bytes_down, 250);
        assert_eq!(snap.active_connections, 1);
    }
}
