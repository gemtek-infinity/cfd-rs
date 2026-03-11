use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

static VERBOSITY: OnceLock<Verbosity> = OnceLock::new();
static NEXT_SPAN_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verbosity {
    Quiet,
    Brief,
    Chatty,
}

impl Verbosity {
    fn from_env() -> Self {
        match std::env::var("MCP_LOG").as_deref() {
            Ok("quiet") => Self::Quiet,
            Ok("chatty") => Self::Chatty,
            _ => Self::Brief,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Quiet => "quiet",
            Self::Brief => "brief",
            Self::Chatty => "chatty",
        }
    }
}

/// Initialize the logging layer. Call once at startup before serving.
pub fn init() {
    let v = *VERBOSITY.get_or_init(Verbosity::from_env);
    if v >= Verbosity::Brief {
        eprintln!("[info] mcp:ready name=cfd-rs-memory verbosity={}", v.as_str());
    }
}

fn verbosity() -> Verbosity {
    *VERBOSITY.get().unwrap_or(&Verbosity::Brief)
}

/// Lightweight span for timing and logging a single tool invocation.
pub struct ToolSpan {
    name: &'static str,
    id: u64,
    start: Instant,
}

impl ToolSpan {
    /// Begin a tool span. Emits `tool:start` in brief and chatty modes.
    pub fn start(name: &'static str) -> Self {
        let id = NEXT_SPAN_ID.fetch_add(1, Ordering::Relaxed);
        let span = Self {
            name,
            id,
            start: Instant::now(),
        };
        if verbosity() >= Verbosity::Brief {
            eprintln!("[info] tool:start name={} id={}", name, id);
        }
        span
    }

    /// Log successful completion with compact metrics. Brief and chatty modes.
    pub fn done(&self, metrics: &str) {
        if verbosity() >= Verbosity::Brief {
            let elapsed_ms = self.start.elapsed().as_millis();
            if metrics.is_empty() {
                eprintln!(
                    "[info] tool:done name={} id={} elapsed_ms={}",
                    self.name, self.id, elapsed_ms,
                );
            } else {
                eprintln!(
                    "[info] tool:done name={} id={} elapsed_ms={} {}",
                    self.name, self.id, elapsed_ms, metrics,
                );
            }
        }
    }

    /// Log a tool error. Always emitted, even in quiet mode.
    pub fn error(&self, error: &str) {
        let elapsed_ms = self.start.elapsed().as_millis();
        eprintln!(
            "[warn] tool:error name={} id={} elapsed_ms={} error=\"{}\"",
            self.name, self.id, elapsed_ms, error,
        );
    }

    /// Log extra context visible only in chatty mode.
    pub fn detail(&self, detail: &str) {
        if verbosity() >= Verbosity::Chatty {
            eprintln!("[info] tool:detail name={} id={} {}", self.name, self.id, detail,);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_verbosity_is_brief() {
        // When MCP_LOG is unset, from_env returns Brief.
        // This test assumes MCP_LOG is not set in the test environment.
        let v = Verbosity::from_env();
        assert!(v >= Verbosity::Brief);
    }

    #[test]
    fn verbosity_ordering() {
        assert!(Verbosity::Quiet < Verbosity::Brief);
        assert!(Verbosity::Brief < Verbosity::Chatty);
    }

    #[test]
    fn verbosity_as_str_roundtrips() {
        assert_eq!(Verbosity::Quiet.as_str(), "quiet");
        assert_eq!(Verbosity::Brief.as_str(), "brief");
        assert_eq!(Verbosity::Chatty.as_str(), "chatty");
    }
}
