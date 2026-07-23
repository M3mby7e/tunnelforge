use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

use crate::model::{LogLevel, LogLine, StatsSnapshot, TunnelState, TunnelStatus};

/// Events the engine emits toward the UI (or a test consumer).
#[derive(Debug, Clone)]
pub enum EngineEvent {
    Status(TunnelStatus),
    Log(LogLine),
}

/// Channel the engine pushes events into. The Tauri layer (or a test) owns the
/// receiver; the engine is decoupled from Tauri.
pub type EventSink = UnboundedSender<EngineEvent>;

/// Convenience wrapper that stamps a tunnel id onto every emitted event.
/// Send failures (receiver dropped) are ignored — a stopped UI must not crash
/// the engine.
#[derive(Clone)]
pub struct Emitter {
    id: Uuid,
    sink: EventSink,
}

impl Emitter {
    pub fn new(id: Uuid, sink: EventSink) -> Self {
        Self { id, sink }
    }

    pub fn status(&self, state: TunnelState) {
        let _ = self
            .sink
            .send(EngineEvent::Status(TunnelStatus::new(self.id, state)));
    }

    pub fn status_msg(&self, state: TunnelState, message: impl Into<String>) {
        let status = TunnelStatus::new(self.id, state).with_message(message);
        let _ = self.sink.send(EngineEvent::Status(status));
    }

    /// Emit a status carrying a fresh stats snapshot (used by the periodic ticker).
    pub fn status_with_stats(&self, state: TunnelState, stats: StatsSnapshot) {
        let mut status = TunnelStatus::new(self.id, state);
        status.stats = stats;
        let _ = self.sink.send(EngineEvent::Status(status));
    }

    pub fn log(&self, level: LogLevel, line: impl Into<String>) {
        let _ = self
            .sink
            .send(EngineEvent::Log(LogLine::new(self.id, level, line)));
    }

    pub fn info(&self, line: impl Into<String>) {
        self.log(LogLevel::Info, line);
    }

    pub fn warn(&self, line: impl Into<String>) {
        self.log(LogLevel::Warn, line);
    }

    pub fn error(&self, line: impl Into<String>) {
        self.log(LogLevel::Error, line);
    }
}
