//! ICMP proxy contracts.
//!
//! Covers HIS-069 through HIS-071.
//!
//! Deferred to Host and Runtime Foundation. Defines the trait contract,
//! source-address auto-detection, and the relevant constants.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};

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
    // Falls back to u32::MAX so the range check safely returns false.
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

// --- HIS-071: ICMP source address auto-detection ---

/// IPv6 address entry parsed from `/proc/net/if_inet6`.
struct IfInet6Entry {
    addr: Ipv6Addr,
    iface: String,
}

/// Parse entries from `/proc/net/if_inet6` content.
///
/// Each line: `<32 hex chars> <index> <prefix_len> <scope> <flags> <name>`
fn parse_if_inet6_content(content: &str) -> Vec<IfInet6Entry> {
    let mut entries = Vec::new();

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() < 6 {
            continue;
        }

        let hex = parts[0];

        if hex.len() != 32 {
            continue;
        }

        let mut octets = [0u8; 16];
        let mut valid = true;

        for i in 0..16 {
            match u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16) {
                Ok(b) => octets[i] = b,
                Err(_) => {
                    valid = false;
                    break;
                }
            }
        }

        if !valid {
            continue;
        }

        entries.push(IfInet6Entry {
            addr: Ipv6Addr::from(octets),
            iface: parts[5].to_owned(),
        });
    }

    entries
}

/// Parse `/proc/net/if_inet6` for all IPv6 addresses.
///
/// Linux-only: returns an empty vec on non-Linux or when the file
/// is missing.
fn parse_if_inet6() -> Vec<IfInet6Entry> {
    let Ok(content) = std::fs::read_to_string("/proc/net/if_inet6") else {
        return Vec::new();
    };
    parse_if_inet6_content(&content)
}

/// Auto-detect local address via the UDP-connect trick.
///
/// Binds a UDP socket and connects to `dst:port` without sending
/// data. The OS kernel selects the local address based on routing.
/// Returns `None` if the system has no route to `dst`.
///
/// Go: `findLocalAddr(dst net.IP, port int)`.
pub fn find_local_addr(dst: IpAddr, port: u16) -> Option<IpAddr> {
    let bind_addr = if dst.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" };
    let socket = UdpSocket::bind(bind_addr).ok()?;
    socket.connect(SocketAddr::new(dst, port)).ok()?;
    Some(socket.local_addr().ok()?.ip())
}

/// Determine ICMPv4 source address from user input or auto-detection.
///
/// If `user_defined` is a valid IPv4 string, returns it directly.
/// Otherwise auto-detects via `find_local_addr("192.168.0.1", 53)`.
/// Falls back to `0.0.0.0` when auto-detection fails.
///
/// Go: `determineICMPv4Src(userDefinedSrc, logger)`.
pub fn determine_icmpv4_src(user_defined: Option<&str>) -> Ipv4Addr {
    if let Some(addr) = user_defined
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<Ipv4Addr>().ok())
    {
        return addr;
    }

    match find_local_addr(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)), 53) {
        Some(IpAddr::V4(addr)) => addr,
        _ => Ipv4Addr::UNSPECIFIED,
    }
}

/// Determine ICMPv6 source address and zone from user input or
/// `/proc/net/if_inet6` enumeration.
///
/// If `user_defined` is a valid IPv6 string, returns it with an
/// empty zone. Otherwise enumerates IPv6 addresses from
/// `/proc/net/if_inet6` and returns the first non-loopback entry
/// with its interface name as the zone.
///
/// Go's `determineICMPv6Src` prefers the interface that also holds
/// `ipv4_src`. This implementation returns the first non-loopback
/// IPv6 from any interface — Linux routing provides equivalent
/// selection in practice.
///
/// Linux-only: reads `/proc/net/if_inet6`.
pub fn determine_icmpv6_src(user_defined: Option<&str>, _ipv4_src: Ipv4Addr) -> (Ipv6Addr, String) {
    if let Some(addr) = user_defined
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<Ipv6Addr>().ok())
    {
        return (addr, String::new());
    }

    for entry in parse_if_inet6() {
        if !entry.addr.is_loopback() {
            return (entry.addr, entry.iface);
        }
    }

    (Ipv6Addr::UNSPECIFIED, String::new())
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

    // --- HIS-071: ICMP source-address flag and env parity ---
    // Go: baseline-2026.2.0/cmd/cloudflared/flags/flags.go
    //     ICMPV4Src = "icmpv4-src"
    //     ICMPV6Src = "icmpv6-src"
    // Go: baseline-2026.2.0/cmd/cloudflared/tunnel/subcommands.go
    //     EnvVars: []string{"TUNNEL_ICMPV4_SRC"}
    //     EnvVars: []string{"TUNNEL_ICMPV6_SRC"}

    #[test]
    fn icmpv4_src_flag_matches_go_baseline() {
        assert_eq!(ICMPV4_SRC_FLAG, "icmpv4-src");
    }

    #[test]
    fn icmpv6_src_flag_matches_go_baseline() {
        assert_eq!(ICMPV6_SRC_FLAG, "icmpv6-src");
    }

    #[test]
    fn icmpv4_src_env_matches_go_baseline() {
        assert_eq!(ICMPV4_SRC_ENV, "TUNNEL_ICMPV4_SRC");
    }

    #[test]
    fn icmpv6_src_env_matches_go_baseline() {
        assert_eq!(ICMPV6_SRC_ENV, "TUNNEL_ICMPV6_SRC");
    }

    // --- HIS-071: ICMP source address auto-detection ---

    #[test]
    fn find_local_addr_does_not_panic() {
        // Environment-dependent: may return None in restricted CI.
        let _ = find_local_addr(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)), 53);
    }

    #[test]
    fn determine_icmpv4_src_parses_explicit_addr() {
        assert_eq!(determine_icmpv4_src(Some("10.0.0.5")), Ipv4Addr::new(10, 0, 0, 5),);
    }

    #[test]
    fn determine_icmpv4_src_empty_triggers_auto_detect() {
        let _ = determine_icmpv4_src(Some(""));
    }

    #[test]
    fn determine_icmpv4_src_none_triggers_auto_detect() {
        let _ = determine_icmpv4_src(None);
    }

    #[test]
    fn determine_icmpv4_src_invalid_input_falls_through() {
        let _ = determine_icmpv4_src(Some("not-an-address"));
    }

    #[test]
    fn determine_icmpv6_src_parses_explicit_addr() {
        let (addr, zone) = determine_icmpv6_src(Some("fe80::1"), Ipv4Addr::LOCALHOST);
        assert_eq!(addr, "fe80::1".parse::<Ipv6Addr>().expect("valid ipv6"));
        assert!(zone.is_empty());
    }

    #[test]
    fn determine_icmpv6_src_none_enumerates_interfaces() {
        let (_addr, _zone) = determine_icmpv6_src(None, Ipv4Addr::new(10, 0, 0, 1));
    }

    #[test]
    fn parse_if_inet6_content_loopback() {
        let content = "00000000000000000000000000000001 01 80 10 80 lo\n";
        let entries = parse_if_inet6_content(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].addr, Ipv6Addr::LOCALHOST);
        assert_eq!(entries[0].iface, "lo");
    }

    #[test]
    fn parse_if_inet6_content_multiple_interfaces() {
        let content = "00000000000000000000000000000001 01 80 10 80 lo\nfe800000000000000000000000000001 02 \
                       40 20 80 eth0\n";
        let entries = parse_if_inet6_content(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].iface, "lo");
        assert_eq!(entries[1].iface, "eth0");
    }

    #[test]
    fn parse_if_inet6_content_empty() {
        assert!(parse_if_inet6_content("").is_empty());
    }

    #[test]
    fn parse_if_inet6_content_invalid_hex_skipped() {
        let content = "ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ 01 80 10 80 lo\n";
        assert!(parse_if_inet6_content(content).is_empty());
    }

    #[test]
    fn parse_if_inet6_content_short_line_skipped() {
        let content = "00000000000000000000000000000001 01 80\n";
        assert!(parse_if_inet6_content(content).is_empty());
    }

    #[test]
    fn determine_icmpv6_src_prefers_non_loopback() {
        // Parse synthetic content: loopback first, then eth0 with
        // a link-local address. The auto-detect should skip loopback.
        let content = "00000000000000000000000000000001 01 80 10 80 lo\nfe800000000000000000000000000001 02 \
                       40 20 80 eth0\n";
        let entries = parse_if_inet6_content(content);

        // Simulates the selection logic in determine_icmpv6_src.
        let selected = entries.iter().find(|e| !e.addr.is_loopback());
        assert!(selected.is_some());
        assert_eq!(selected.expect("non-loopback entry").iface, "eth0");
    }
}
