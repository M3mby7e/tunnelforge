use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

/// Live lifecycle state of a tunnel (not persisted).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum TunnelState {
    Idle,
    Connecting,
    Connected,
    Reconnecting,
    Error,
    Stopping,
}

/// Point-in-time traffic/health metrics for a tunnel.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StatsSnapshot {
    // u64 in Rust for accurate counters; Tauri delivers them as JSON numbers,
    // so the TS side sees `number` rather than `bigint`.
    #[ts(type = "number")]
    pub bytes_up: u64,
    #[ts(type = "number")]
    pub bytes_down: u64,
    pub active_connections: u32,
    #[ts(type = "number")]
    pub uptime_seconds: u64,
    pub retry_count: u32,
}

/// A status update emitted to the UI over the `tunnel://status` event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct TunnelStatus {
    pub id: Uuid,
    pub state: TunnelState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub stats: StatsSnapshot,
    pub since: DateTime<Utc>,
}

impl TunnelStatus {
    pub fn new(id: Uuid, state: TunnelState) -> Self {
        Self {
            id,
            state,
            message: None,
            stats: StatsSnapshot::default(),
            since: Utc::now(),
        }
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

/// A single log line emitted to the UI over the `tunnel://log` event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    pub id: Uuid,
    pub level: LogLevel,
    pub ts: DateTime<Utc>,
    pub line: String,
}

impl LogLine {
    pub fn new(id: Uuid, level: LogLevel, line: impl Into<String>) -> Self {
        Self {
            id,
            level,
            ts: Utc::now(),
            line: line.into(),
        }
    }
}
