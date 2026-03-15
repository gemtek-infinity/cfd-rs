// CDC-026: Log streaming WebSocket protocol types.
//
// Frozen baseline: management/events.go, management/session.go
//
// The management service exposes a `/logs` WebSocket endpoint. Clients send
// `start_streaming` / `stop_streaming` text frames; the server replies with
// `logs` text frames containing an array of log entries. All messages are JSON.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// ---------------------------------------------------------------------------
// Log event types — maps to Go `LogEventType` (int8, JSON-serialized as string)
// ---------------------------------------------------------------------------

/// Log event type discriminator.
///
/// Go uses `int8` iota with custom `MarshalJSON`/`UnmarshalJSON` that
/// serialize as lowercase string names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i8)]
pub enum LogEventType {
    Cloudflared = 0,
    Http = 1,
    Tcp = 2,
    Udp = 3,
}

impl LogEventType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cloudflared => "cloudflared",
            Self::Http => "http",
            Self::Tcp => "tcp",
            Self::Udp => "udp",
        }
    }

    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s {
            "cloudflared" => Some(Self::Cloudflared),
            "http" => Some(Self::Http),
            "tcp" => Some(Self::Tcp),
            "udp" => Some(Self::Udp),
            _ => None,
        }
    }
}

impl fmt::Display for LogEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for LogEventType {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for LogEventType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::from_str_opt(&s).ok_or_else(|| serde::de::Error::custom(format!("unknown log event type: {s}")))
    }
}

// ---------------------------------------------------------------------------
// Log level — maps to Go `LogLevel` (int8, JSON-serialized as string)
// ---------------------------------------------------------------------------

/// Log level for streaming filter and log entries.
///
/// Go uses `int8` iota with custom `MarshalJSON`/`UnmarshalJSON`.
/// Only debug/info/warn/error are valid; panic/fatal/trace are excluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i8)]
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
}

impl LogLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }

    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s {
            "debug" => Some(Self::Debug),
            "info" => Some(Self::Info),
            "warn" => Some(Self::Warn),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for LogLevel {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for LogLevel {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::from_str_opt(&s).ok_or_else(|| serde::de::Error::custom(format!("unknown log level: {s}")))
    }
}

// ---------------------------------------------------------------------------
// Log JSON field key constants — maps to Go zerolog field key constants
// ---------------------------------------------------------------------------

/// Key for the timestamp field in a log entry.
pub const TIME_KEY: &str = "time";

/// Key for the level field in a log entry.
pub const LEVEL_KEY: &str = "level";

/// Key for the message field in a log entry.
pub const MESSAGE_KEY: &str = "message";

/// Key for the event type field in a log entry.
pub const EVENT_TYPE_KEY: &str = "event";

/// Key for the catch-all fields in a log entry.
pub const FIELDS_KEY: &str = "fields";

// ---------------------------------------------------------------------------
// Log entry — maps to Go `Log` struct
// ---------------------------------------------------------------------------

/// A single log entry sent by the server.
///
/// Go JSON: `{time, level, message, event, fields}` — all `omitempty`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogEntry {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub time: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<LogLevel>,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub message: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<LogEventType>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<HashMap<String, serde_json::Value>>,
}

// ---------------------------------------------------------------------------
// Streaming filters — maps to Go `StreamingFilters` struct
// ---------------------------------------------------------------------------

/// Filters applied to a log streaming session.
///
/// Go JSON: `{events, level, sampling}` — all `omitempty`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StreamingFilters {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<LogEventType>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<LogLevel>,

    #[serde(default, skip_serializing_if = "is_zero_f64")]
    pub sampling: f64,
}

fn is_zero_f64(v: &f64) -> bool {
    *v == 0.0
}

impl StreamingFilters {
    /// Clamp sampling to `[0.0, 1.0]` as Go does.
    pub fn clamped_sampling(&self) -> f64 {
        self.sampling.clamp(0.0, 1.0)
    }

    /// Convert clamped sampling to an integer percentage for the sampler.
    /// Returns `None` if sampling is effectively disabled (0 or 1).
    pub fn sampling_percentage(&self) -> Option<u32> {
        let s = self.clamped_sampling();
        if s <= 0.0 || s >= 1.0 {
            return None;
        }
        Some((s * 100.0) as u32)
    }
}

impl Default for StreamingFilters {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            level: None,
            sampling: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Client event types — maps to Go `ClientEventType` string constants
// ---------------------------------------------------------------------------

/// Client-to-server event type discriminator.
pub const CLIENT_EVENT_START_STREAMING: &str = "start_streaming";
pub const CLIENT_EVENT_STOP_STREAMING: &str = "stop_streaming";

/// Server-to-client event type discriminator.
pub const SERVER_EVENT_LOGS: &str = "logs";

// ---------------------------------------------------------------------------
// Client events — maps to Go `EventStartStreaming`, `EventStopStreaming`
// ---------------------------------------------------------------------------

/// Client event: start log streaming with optional filters.
///
/// Go JSON: `{"type": "start_streaming", "filters": {...}}`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventStartStreaming {
    #[serde(rename = "type")]
    pub event_type: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filters: Option<StreamingFilters>,
}

impl EventStartStreaming {
    pub fn new(filters: Option<StreamingFilters>) -> Self {
        Self {
            event_type: CLIENT_EVENT_START_STREAMING.to_owned(),
            filters,
        }
    }
}

/// Client event: stop log streaming.
///
/// Go JSON: `{"type": "stop_streaming"}`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventStopStreaming {
    #[serde(rename = "type")]
    pub event_type: String,
}

impl EventStopStreaming {
    pub fn new() -> Self {
        Self {
            event_type: CLIENT_EVENT_STOP_STREAMING.to_owned(),
        }
    }
}

impl Default for EventStopStreaming {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Server events — maps to Go `EventLog`
// ---------------------------------------------------------------------------

/// Server event: log entries.
///
/// Go JSON: `{"type": "logs", "logs": [...]}`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventLog {
    #[serde(rename = "type")]
    pub event_type: String,

    pub logs: Vec<LogEntry>,
}

impl EventLog {
    pub fn new(logs: Vec<LogEntry>) -> Self {
        Self {
            event_type: SERVER_EVENT_LOGS.to_owned(),
            logs,
        }
    }
}

// ---------------------------------------------------------------------------
// Session constants — maps to Go `session.go` constants
// ---------------------------------------------------------------------------

/// Buffered channel capacity for queued log entries before dropping.
/// Go: `logWindow = 30`.
pub const LOG_WINDOW: usize = 30;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_event_type_json_roundtrip() {
        for (variant, expected) in [
            (LogEventType::Cloudflared, "\"cloudflared\""),
            (LogEventType::Http, "\"http\""),
            (LogEventType::Tcp, "\"tcp\""),
            (LogEventType::Udp, "\"udp\""),
        ] {
            let json = serde_json::to_string(&variant).expect("serialize");
            assert_eq!(json, expected, "variant={variant:?}");
            let back: LogEventType = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, variant);
        }
    }

    #[test]
    fn log_level_json_roundtrip() {
        for (variant, expected) in [
            (LogLevel::Debug, "\"debug\""),
            (LogLevel::Info, "\"info\""),
            (LogLevel::Warn, "\"warn\""),
            (LogLevel::Error, "\"error\""),
        ] {
            let json = serde_json::to_string(&variant).expect("serialize");
            assert_eq!(json, expected, "variant={variant:?}");
            let back: LogLevel = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, variant);
        }
    }

    #[test]
    fn log_level_ordering_matches_go() {
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
    }

    #[test]
    fn log_entry_full_json_shape() {
        let entry = LogEntry {
            time: "2026-03-15T10:00:00Z".to_owned(),
            level: Some(LogLevel::Info),
            message: "connection established".to_owned(),
            event: Some(LogEventType::Cloudflared),
            fields: Some(HashMap::from([
                ("connIndex".to_owned(), serde_json::json!(0)),
                ("ip".to_owned(), serde_json::json!("198.41.200.1")),
            ])),
        };
        let json = serde_json::to_value(&entry).expect("serialize");
        assert_eq!(json["time"], "2026-03-15T10:00:00Z");
        assert_eq!(json["level"], "info");
        assert_eq!(json["message"], "connection established");
        assert_eq!(json["event"], "cloudflared");
        assert_eq!(json["fields"]["connIndex"], 0);
        assert_eq!(json["fields"]["ip"], "198.41.200.1");
    }

    #[test]
    fn log_entry_omitempty_matches_go() {
        let entry = LogEntry {
            time: String::new(),
            level: None,
            message: String::new(),
            event: None,
            fields: None,
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        assert_eq!(json, "{}");
    }

    #[test]
    fn streaming_filters_json_shape() {
        let filters = StreamingFilters {
            events: vec![LogEventType::Http, LogEventType::Tcp],
            level: Some(LogLevel::Warn),
            sampling: 0.5,
        };
        let json = serde_json::to_value(&filters).expect("serialize");
        assert_eq!(json["events"], serde_json::json!(["http", "tcp"]));
        assert_eq!(json["level"], "warn");
        assert_eq!(json["sampling"], 0.5);
    }

    #[test]
    fn streaming_filters_empty_omits_all() {
        let filters = StreamingFilters::default();
        let json = serde_json::to_string(&filters).expect("serialize");
        assert_eq!(json, "{}");
    }

    #[test]
    fn sampling_clamping() {
        let f = StreamingFilters {
            sampling: 1.5,
            ..Default::default()
        };
        assert_eq!(f.clamped_sampling(), 1.0);
        let f = StreamingFilters {
            sampling: -0.5,
            ..Default::default()
        };
        assert_eq!(f.clamped_sampling(), 0.0);
        let f = StreamingFilters {
            sampling: 0.5,
            ..Default::default()
        };
        assert_eq!(f.clamped_sampling(), 0.5);
    }

    #[test]
    fn sampling_percentage_edge_cases() {
        let f = StreamingFilters {
            sampling: 0.0,
            ..Default::default()
        };
        assert_eq!(f.sampling_percentage(), None);
        let f = StreamingFilters {
            sampling: 1.0,
            ..Default::default()
        };
        assert_eq!(f.sampling_percentage(), None);
        let f = StreamingFilters {
            sampling: 0.5,
            ..Default::default()
        };
        assert_eq!(f.sampling_percentage(), Some(50));
        let f = StreamingFilters {
            sampling: 0.01,
            ..Default::default()
        };
        assert_eq!(f.sampling_percentage(), Some(1));
    }

    #[test]
    fn event_start_streaming_json_shape() {
        let event = EventStartStreaming::new(Some(StreamingFilters {
            events: vec![LogEventType::Http],
            level: Some(LogLevel::Warn),
            sampling: 0.5,
        }));
        let json = serde_json::to_value(&event).expect("serialize");
        assert_eq!(json["type"], "start_streaming");
        assert_eq!(json["filters"]["events"], serde_json::json!(["http"]));
        assert_eq!(json["filters"]["level"], "warn");
        assert_eq!(json["filters"]["sampling"], 0.5);
    }

    #[test]
    fn event_stop_streaming_json_shape() {
        let event = EventStopStreaming::new();
        let json = serde_json::to_string(&event).expect("serialize");
        assert_eq!(json, r#"{"type":"stop_streaming"}"#);
    }

    #[test]
    fn event_log_json_shape() {
        let event = EventLog::new(vec![LogEntry {
            time: "2026-03-15T10:00:00Z".to_owned(),
            level: Some(LogLevel::Info),
            message: "test".to_owned(),
            event: Some(LogEventType::Cloudflared),
            fields: None,
        }]);
        let json = serde_json::to_value(&event).expect("serialize");
        assert_eq!(json["type"], "logs");
        assert_eq!(json["logs"].as_array().expect("array").len(), 1);
        assert_eq!(json["logs"][0]["message"], "test");
    }

    #[test]
    fn start_streaming_roundtrip() {
        let event = EventStartStreaming::new(Some(StreamingFilters {
            events: vec![LogEventType::Http, LogEventType::Tcp],
            level: Some(LogLevel::Debug),
            sampling: 0.75,
        }));
        let json = serde_json::to_string(&event).expect("serialize");
        let back: EventStartStreaming = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, event);
    }

    #[test]
    fn event_log_roundtrip() {
        let event = EventLog::new(vec![LogEntry {
            time: "2026-01-01T00:00:00Z".to_owned(),
            level: Some(LogLevel::Error),
            message: "err".to_owned(),
            event: Some(LogEventType::Udp),
            fields: Some(HashMap::from([("key".to_owned(), serde_json::json!("value"))])),
        }]);
        let json = serde_json::to_string(&event).expect("serialize");
        let back: EventLog = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, event);
    }

    #[test]
    fn log_window_matches_go() {
        assert_eq!(LOG_WINDOW, 30);
    }

    #[test]
    fn log_level_filter_semantics() {
        // Go: reject if *filters.Level > log.Level (minimum level threshold)
        let min_level = LogLevel::Warn;
        assert!(LogLevel::Debug < min_level, "debug should be filtered out");
        assert!(LogLevel::Info < min_level, "info should be filtered out");
        assert!(LogLevel::Warn >= min_level, "warn should pass");
        assert!(LogLevel::Error >= min_level, "error should pass");
    }
}
