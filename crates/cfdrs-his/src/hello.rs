//! Hello world origin test server.
//!
//! Covers HIS-072.
//!
//! Go: `hello/hello.go` serves a test origin with routes:
//! `/`, `/uptime`, `/ws`, `/sse`, `/_health`.
//!
//! Deferred to Host and Runtime Foundation.

/// Routes served by the Go hello world server.
pub const HELLO_ROUTES: &[&str] = &["/", "/uptime", "/ws", "/sse", "/_health"];

/// Trait for the hello world test origin.
pub trait HelloServer: Send + Sync {
    /// Start serving. Blocks until shutdown.
    fn serve(&self, addr: std::net::SocketAddr) -> cfdrs_shared::Result<()>;

    /// Signal shutdown.
    fn shutdown(&self);
}

/// Stub hello server.
pub struct StubHelloServer;

impl HelloServer for StubHelloServer {
    fn serve(&self, _addr: std::net::SocketAddr) -> cfdrs_shared::Result<()> {
        Err(cfdrs_shared::ConfigError::deferred("hello world server"))
    }

    fn shutdown(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_routes_match_go() {
        assert!(HELLO_ROUTES.contains(&"/"));
        assert!(HELLO_ROUTES.contains(&"/uptime"));
        assert!(HELLO_ROUTES.contains(&"/ws"));
        assert!(HELLO_ROUTES.contains(&"/sse"));
        assert!(HELLO_ROUTES.contains(&"/_health"));
    }
}
