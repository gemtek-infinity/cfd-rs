//! ICMP proxy contracts.
//!
//! Covers HIS-069 through HIS-071.
//!
//! Deferred to Host and Runtime Foundation. Defines the trait contract
//! and the relevant constants.

// --- HIS-070: ping group range ---

/// Path to the kernel ping group range file.
pub const PING_GROUP_RANGE_PATH: &str = "/proc/sys/net/ipv4/ping_group_range";

// --- HIS-071: ICMP source address flags ---

/// Environment variable for ICMPv4 source address.
pub const ICMPV4_SRC_ENV: &str = "TUNNEL_ICMPV4_SRC";

/// Environment variable for ICMPv6 source address.
pub const ICMPV6_SRC_ENV: &str = "TUNNEL_ICMPV6_SRC";

/// CLI flag names matching Go baseline.
pub const ICMPV4_SRC_FLAG: &str = "icmpv4-src";
pub const ICMPV6_SRC_FLAG: &str = "icmpv6-src";

// --- HIS-069: ICMP proxy trait ---

/// Trait for ICMP proxy operations.
///
/// Go: `ingress/icmp_linux.go` creates raw ICMP sockets via
/// `net.ListenPacket()`.
pub trait IcmpProxy: Send + Sync {
    /// Start listening for ICMP packets.
    fn start(&self) -> cfdrs_shared::Result<()>;

    /// Stop the ICMP proxy.
    fn stop(&self);
}

/// Check if the current process can create ICMP sockets.
///
/// Go checks `/proc/sys/net/ipv4/ping_group_range` to see if the
/// current GID is in the allowed range.
pub fn can_create_icmp_socket() -> bool {
    let Ok(content) = std::fs::read_to_string(PING_GROUP_RANGE_PATH) else {
        return false;
    };

    let parts: Vec<&str> = content.split_whitespace().collect();
    if parts.len() != 2 {
        return false;
    }

    let Ok(low) = parts[0].parse::<u32>() else {
        return false;
    };
    let Ok(high) = parts[1].parse::<u32>() else {
        return false;
    };

    // Read GID from /proc to avoid unsafe libc call.
    let gid = std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("Gid:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse::<u32>().ok())
        })
        .unwrap_or(u32::MAX);

    gid >= low && gid <= high
}

/// Stub ICMP proxy.
pub struct StubIcmpProxy;

impl IcmpProxy for StubIcmpProxy {
    fn start(&self) -> cfdrs_shared::Result<()> {
        Err(cfdrs_shared::ConfigError::deferred("ICMP proxy"))
    }

    fn stop(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_create_icmp_socket_does_not_panic() {
        // Just verify it doesn't panic in test environment.
        let _ = can_create_icmp_socket();
    }

    #[test]
    fn stub_icmp_returns_deferred() {
        let proxy = StubIcmpProxy;
        assert!(proxy.start().is_err());
    }
}
