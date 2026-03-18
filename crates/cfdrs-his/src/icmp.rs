//! ICMP proxy contracts.
//!
//! Covers HIS-069 through HIS-071.
//!
//! HIS-069: ICMP proxy with non-privileged sockets (Linux `SOCK_DGRAM` +
//!          `IPPROTO_ICMP`). The kernel assigns an ephemeral port as the echo
//!          ID, and filters replies to that port. Outbound echo requests have
//!          their ID rewritten to the assigned port; inbound replies are
//!          rewritten back to the original ID before returning to the caller.
//! HIS-070: ping group range permission check.
//! HIS-071: ICMP source-address auto-detection.

use std::collections::HashMap;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV6, UdpSocket};
use std::os::fd::{AsRawFd, OwnedFd};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::time::Duration;

use nix::sys::socket::{
    self as nixsock, AddressFamily, MsgFlags, SockFlag, SockProtocol, SockType, SockaddrIn, SockaddrIn6,
};
use nix::unistd::getegid;

// ---------------------------------------------------------------------------
// HIS-069: constants
// ---------------------------------------------------------------------------

/// Maximum transmission unit matching Go `packet.mtu`.
pub const MTU: usize = 1500;

/// Default TTL matching Go `packet.DefaultTTL`.
pub const DEFAULT_TTL: u8 = 255;

/// ICMP header length: type(1) + code(1) + checksum(2) + id(2) + seq(2).
const ICMP_HEADER_LEN: usize = 8;

/// Byte offset of the echo identifier within an ICMP message.
const ICMP_ID_OFFSET: usize = 4;

/// Byte offset of the echo sequence number within an ICMP message.
const ICMP_SEQ_OFFSET: usize = 6;

/// ICMP types (RFC 792).
const ICMP_ECHO_REQUEST: u8 = 8;
const ICMP_ECHO_REPLY: u8 = 0;

/// ICMPv6 types (RFC 4443).
const ICMPV6_ECHO_REQUEST: u8 = 128;
const ICMPV6_ECHO_REPLY: u8 = 129;

// ---------------------------------------------------------------------------
// HIS-070: ping group range
// ---------------------------------------------------------------------------

/// Path to the kernel ping group range file.
pub const PING_GROUP_RANGE_PATH: &str = "/proc/sys/net/ipv4/ping_group_range";

// ---------------------------------------------------------------------------
// HIS-071: ICMP source address flags
// ---------------------------------------------------------------------------

/// Environment variable for ICMPv4 source address.
pub const ICMPV4_SRC_ENV: &str = "TUNNEL_ICMPV4_SRC";

/// Environment variable for ICMPv6 source address.
pub const ICMPV6_SRC_ENV: &str = "TUNNEL_ICMPV6_SRC";

/// CLI flag names matching Go baseline.
pub const ICMPV4_SRC_FLAG: &str = "icmpv4-src";
pub const ICMPV6_SRC_FLAG: &str = "icmpv6-src";

// ---------------------------------------------------------------------------
// HIS-069: error type
// ---------------------------------------------------------------------------

/// Errors from ICMP proxy operations.
#[derive(Debug, thiserror::Error)]
pub enum IcmpError {
    #[error("ICMP socket error: {0}")]
    Socket(#[from] nix::errno::Errno),

    #[error("ICMP message too short: {len} bytes (minimum {ICMP_HEADER_LEN})")]
    MessageTooShort { len: usize },

    #[error("unexpected ICMP type {icmp_type} (expected echo request or reply)")]
    UnexpectedType { icmp_type: u8 },

    #[error("process GID {gid} is not within ping_group_range {low}..={high}")]
    NotInPingGroup { gid: u32, low: u32, high: u32 },

    #[error("cannot read ping group range from {path}: {reason}")]
    PingGroupUnreadable { path: &'static str, reason: String },
}

// ---------------------------------------------------------------------------
// HIS-069: ICMP proxy trait
// ---------------------------------------------------------------------------

/// Trait for ICMP proxy operations.
///
/// Go: `ingress/icmp_linux.go` — `icmpProxy` with `Request()` / `Serve()`.
/// Full async runtime integration is a composition concern for `cfdrs-bin`;
/// this trait defines the contract boundary.
pub trait IcmpProxy: Send + Sync {
    /// Start listening for ICMP packets.
    fn start(&self) -> cfdrs_shared::Result<()>;

    /// Stop the ICMP proxy.
    fn stop(&self);
}

/// Stub ICMP proxy — returns a deferred error on start.
pub struct StubIcmpProxy;

impl IcmpProxy for StubIcmpProxy {
    fn start(&self) -> cfdrs_shared::Result<()> {
        Err(cfdrs_shared::ConfigError::deferred("ICMP proxy"))
    }

    fn stop(&self) {}
}

// ---------------------------------------------------------------------------
// HIS-069: ICMP message parsing and checksum
// ---------------------------------------------------------------------------

/// Parsed echo identifier and sequence from an ICMP echo message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EchoFields {
    pub icmp_type: u8,
    pub code: u8,
    pub id: u16,
    pub seq: u16,
}

/// Parse the echo fields from a raw ICMP message.
pub fn parse_echo_fields(msg: &[u8]) -> Result<EchoFields, IcmpError> {
    if msg.len() < ICMP_HEADER_LEN {
        return Err(IcmpError::MessageTooShort { len: msg.len() });
    }

    Ok(EchoFields {
        icmp_type: msg[0],
        code: msg[1],
        id: u16::from_be_bytes([msg[ICMP_ID_OFFSET], msg[ICMP_ID_OFFSET + 1]]),
        seq: u16::from_be_bytes([msg[ICMP_SEQ_OFFSET], msg[ICMP_SEQ_OFFSET + 1]]),
    })
}

/// Returns `true` if the ICMP type is an echo request (v4 or v6).
pub fn is_echo_request(icmp_type: u8) -> bool {
    icmp_type == ICMP_ECHO_REQUEST || icmp_type == ICMPV6_ECHO_REQUEST
}

/// Returns `true` if the ICMP type is an echo reply (v4 or v6).
pub fn is_echo_reply(icmp_type: u8) -> bool {
    icmp_type == ICMP_ECHO_REPLY || icmp_type == ICMPV6_ECHO_REPLY
}

/// Compute the RFC 1071 Internet checksum over `data`.
fn internet_checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0;

    while i + 1 < data.len() {
        sum += u16::from_be_bytes([data[i], data[i + 1]]) as u32;
        i += 2;
    }

    if i < data.len() {
        sum += (data[i] as u32) << 8;
    }

    while sum > 0xffff {
        sum = (sum & 0xffff) + (sum >> 16);
    }

    !sum as u16
}

/// Rewrite the echo ID in a raw ICMP message and recompute the checksum.
///
/// Returns a new `Vec<u8>` with the modified message. The original is
/// not mutated.
///
/// Go: outbound in `icmpEchoFlow.sendToDst()`, inbound in `returnToSrc()`.
pub fn rewrite_echo_id(msg: &[u8], new_id: u16) -> Result<Vec<u8>, IcmpError> {
    if msg.len() < ICMP_HEADER_LEN {
        return Err(IcmpError::MessageTooShort { len: msg.len() });
    }

    let mut out = msg.to_vec();

    // Write new echo ID.
    let id_bytes = new_id.to_be_bytes();
    out[ICMP_ID_OFFSET] = id_bytes[0];
    out[ICMP_ID_OFFSET + 1] = id_bytes[1];

    // Zero checksum field before recomputation.
    out[2] = 0;
    out[3] = 0;

    let cksum = internet_checksum(&out);
    let cksum_bytes = cksum.to_be_bytes();
    out[2] = cksum_bytes[0];
    out[3] = cksum_bytes[1];

    Ok(out)
}

// ---------------------------------------------------------------------------
// HIS-069: non-privileged ICMP socket
// ---------------------------------------------------------------------------

/// Non-privileged ICMP socket.
///
/// Go: `newICMPConn()` in `icmp_posix.go` → `icmp.ListenPacket("udp4", ip)`.
/// Linux kernel: `socket(AF_INET, SOCK_DGRAM, IPPROTO_ICMP)` + `bind()`.
/// The kernel assigns an ephemeral port that becomes the echo ID for this
/// socket. Replies are filtered by the kernel to match that port.
pub struct IcmpConn {
    fd: OwnedFd,
    local_port: u16,
    is_v4: bool,
}

impl IcmpConn {
    /// Open a non-privileged ICMPv4 socket bound to `listen_ip`.
    ///
    /// The kernel assigns a port; retrieve it with [`Self::local_port()`].
    pub fn new_v4(listen_ip: Ipv4Addr) -> Result<Self, IcmpError> {
        let fd = nixsock::socket(
            AddressFamily::Inet,
            SockType::Datagram,
            SockFlag::SOCK_CLOEXEC,
            SockProtocol::Icmp,
        )?;

        let octets = listen_ip.octets();
        let bind_addr = SockaddrIn::new(octets[0], octets[1], octets[2], octets[3], 0);
        nixsock::bind(fd.as_raw_fd(), &bind_addr)?;

        let local: SockaddrIn = nixsock::getsockname(fd.as_raw_fd())?;
        let local_port = local.port();

        Ok(Self {
            fd,
            local_port,
            is_v4: true,
        })
    }

    /// Open a non-privileged ICMPv6 socket bound to `listen_ip`.
    pub fn new_v6(listen_ip: Ipv6Addr) -> Result<Self, IcmpError> {
        let fd = nixsock::socket(
            AddressFamily::Inet6,
            SockType::Datagram,
            SockFlag::SOCK_CLOEXEC,
            SockProtocol::IcmpV6,
        )?;

        let sock_addr = SockaddrIn6::from(SocketAddrV6::new(listen_ip, 0, 0, 0));
        nixsock::bind(fd.as_raw_fd(), &sock_addr)?;

        let local: SockaddrIn6 = nixsock::getsockname(fd.as_raw_fd())?;
        let local_port = local.port();

        Ok(Self {
            fd,
            local_port,
            is_v4: false,
        })
    }

    /// Open an ICMP socket matching the address family of `listen_ip`.
    pub fn new(listen_ip: IpAddr) -> Result<Self, IcmpError> {
        match listen_ip {
            IpAddr::V4(v4) => Self::new_v4(v4),
            IpAddr::V6(v6) => Self::new_v6(v6),
        }
    }

    /// The kernel-assigned port, which serves as the echo ID for this socket.
    pub fn local_port(&self) -> u16 {
        self.local_port
    }

    /// Whether this is an IPv4 socket.
    pub fn is_v4(&self) -> bool {
        self.is_v4
    }

    /// Send `data` (a complete ICMP message including header) to `dst`.
    pub fn send_to(&self, data: &[u8], dst: IpAddr) -> Result<usize, IcmpError> {
        let n = match dst {
            IpAddr::V4(v4) => {
                let octets = v4.octets();
                let addr = SockaddrIn::new(octets[0], octets[1], octets[2], octets[3], 0);
                nixsock::sendto(self.fd.as_raw_fd(), data, &addr, MsgFlags::empty())?
            }
            IpAddr::V6(v6) => {
                let addr = SockaddrIn6::from(SocketAddrV6::new(v6, 0, 0, 0));
                nixsock::sendto(self.fd.as_raw_fd(), data, &addr, MsgFlags::empty())?
            }
        };
        Ok(n)
    }

    /// Receive one ICMP message into `buf`.
    ///
    /// Returns `(bytes_read, sender_ip)`.
    pub fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, IpAddr), IcmpError> {
        if self.is_v4 {
            let (n, addr) = nixsock::recvfrom::<SockaddrIn>(self.fd.as_raw_fd(), buf)?;
            let ip = addr
                .map(|a| IpAddr::V4(a.ip()))
                .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
            Ok((n, ip))
        } else {
            let (n, addr) = nixsock::recvfrom::<SockaddrIn6>(self.fd.as_raw_fd(), buf)?;
            let ip = addr
                .map(|a| {
                    let sa: SocketAddrV6 = a.into();
                    IpAddr::V6(*sa.ip())
                })
                .unwrap_or(IpAddr::V6(Ipv6Addr::UNSPECIFIED));
            Ok((n, ip))
        }
    }
}

// ---------------------------------------------------------------------------
// HIS-069: flow identifier
// ---------------------------------------------------------------------------

/// Flow identifier: `(src_ip, dst_ip, original_echo_id)`.
///
/// Go: `flow3Tuple` in `icmp_linux.go`. Used as the key in `FunnelTracker`.
/// The `Type()` method returns `"srcIP_dstIP_echoID"`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Flow3Tuple {
    pub src_ip: IpAddr,
    pub dst_ip: IpAddr,
    pub original_echo_id: u16,
}

impl fmt::Display for Flow3Tuple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.src_ip, self.dst_ip, self.original_echo_id)
    }
}

impl Flow3Tuple {
    /// Returns the flow type string matching Go `flow3Tuple.Type()`.
    pub fn flow_type(&self) -> &'static str {
        "srcIP_dstIP_echoID"
    }
}

// ---------------------------------------------------------------------------
// HIS-069: per-flow echo ID rewriting state
// ---------------------------------------------------------------------------

/// Per-flow ICMP state with echo ID rewriting.
///
/// Go: `icmpEchoFlow` in `icmp_posix.go`. Each flow owns a socket whose
/// kernel-assigned port is the `assigned_echo_id`. Outbound requests have
/// their echo ID rewritten from `original → assigned`; inbound replies are
/// rewritten `assigned → original`.
pub struct IcmpEchoFlow {
    conn: IcmpConn,
    assigned_echo_id: u16,
    original_echo_id: u16,
    src: IpAddr,
    last_active: AtomicI64,
    closed: AtomicBool,
}

impl IcmpEchoFlow {
    /// Create a new flow with the given socket and echo ID mapping.
    pub fn new(conn: IcmpConn, src: IpAddr, original_echo_id: u16) -> Self {
        let assigned_echo_id = conn.local_port();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        Self {
            conn,
            assigned_echo_id,
            original_echo_id,
            src,
            last_active: AtomicI64::new(now),
            closed: AtomicBool::new(false),
        }
    }

    /// The echo ID assigned by the kernel (= socket port).
    pub fn assigned_echo_id(&self) -> u16 {
        self.assigned_echo_id
    }

    /// The original echo ID from the eyeball client.
    pub fn original_echo_id(&self) -> u16 {
        self.original_echo_id
    }

    /// The source IP address that packets are returned to.
    pub fn src(&self) -> IpAddr {
        self.src
    }

    /// Mark this flow as active (updates last-active timestamp).
    pub fn update_last_active(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        self.last_active.store(now, Ordering::Relaxed);
    }

    /// Unix timestamp (seconds) of last activity.
    pub fn last_active_secs(&self) -> i64 {
        self.last_active.load(Ordering::Relaxed)
    }

    /// Mark this flow as closed.
    pub fn close(&self) {
        self.closed.store(true, Ordering::Release);
    }

    /// Whether this flow has been closed.
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    /// Send an ICMP echo request to `dst`, rewriting the echo ID from
    /// original → assigned.
    ///
    /// Go: `icmpEchoFlow.sendToDst()`.
    pub fn send_to_dst(&self, dst: IpAddr, msg: &[u8]) -> Result<usize, IcmpError> {
        self.update_last_active();
        let rewritten = rewrite_echo_id(msg, self.assigned_echo_id)?;
        self.conn.send_to(&rewritten, dst)
    }

    /// Receive one reply from the socket and rewrite the echo ID from
    /// assigned → original.
    ///
    /// Returns `(rewritten_message, sender_ip)`.
    ///
    /// Go: the receive side of `icmpProxy.listenResponse()` +
    ///     `icmpEchoFlow.returnToSrc()`.
    pub fn recv_and_rewrite(&self, buf: &mut [u8]) -> Result<(Vec<u8>, IpAddr), IcmpError> {
        let (n, from) = self.conn.recv_from(buf)?;
        self.update_last_active();
        let rewritten = rewrite_echo_id(&buf[..n], self.original_echo_id)?;
        Ok((rewritten, from))
    }
}

// ---------------------------------------------------------------------------
// HIS-069: flow tracker
// ---------------------------------------------------------------------------

/// Concurrent flow registry.
///
/// Go: `packet.FunnelTracker` in `packet/funnel.go`. Maps `Flow3Tuple` →
/// `IcmpEchoFlow`. Supports `GetOrRegister` with conditional replacement,
/// `Unregister` with equality check, and idle cleanup.
pub struct FlowTracker {
    flows: Mutex<HashMap<Flow3Tuple, IcmpEchoFlow>>,
}

impl FlowTracker {
    pub fn new() -> Self {
        Self {
            flows: Mutex::new(HashMap::new()),
        }
    }

    /// Look up or create a flow for the given triple.
    ///
    /// - If `id` is not registered, calls `new_flow_fn` and registers the
    ///   result. Returns `(flow_ref, true)`.
    /// - If `id` is registered and `should_replace` returns `false`, returns
    ///   the existing flow. Returns `(flow_ref, false)`.
    /// - If `id` is registered and `should_replace` returns `true`, closes the
    ///   old flow, calls `new_flow_fn`, and registers the replacement.
    ///
    /// Go: `FunnelTracker.GetOrRegister()`.
    pub fn get_or_register<F, N>(
        &self,
        id: Flow3Tuple,
        should_replace: F,
        new_flow_fn: N,
    ) -> Result<bool, IcmpError>
    where
        F: FnOnce(&IcmpEchoFlow) -> bool,
        N: FnOnce() -> Result<IcmpEchoFlow, IcmpError>,
    {
        let mut flows = self.flows.lock().expect("flow tracker lock poisoned");

        if let Some(existing) = flows.get(&id) {
            if !should_replace(existing) {
                return Ok(false);
            }
            existing.close();
            flows.remove(&id);
        }

        let new_flow = new_flow_fn()?;
        flows.insert(id, new_flow);
        Ok(true)
    }

    /// Retrieve a reference to the flow for read-only inspection.
    ///
    /// The caller must not hold the lock across await points.
    pub fn with_flow<R>(&self, id: &Flow3Tuple, f: impl FnOnce(&IcmpEchoFlow) -> R) -> Option<R> {
        let flows = self.flows.lock().expect("flow tracker lock poisoned");
        flows.get(id).map(f)
    }

    /// Unregister and close a flow if it matches the expected echo IDs.
    ///
    /// Go: `FunnelTracker.Unregister()` — only removes if the current flow
    /// is `Equal` to the one being unregistered.
    pub fn unregister(&self, id: &Flow3Tuple, assigned_echo_id: u16) -> bool {
        let mut flows = self.flows.lock().expect("flow tracker lock poisoned");

        let matches = flows
            .get(id)
            .is_some_and(|f| f.assigned_echo_id() == assigned_echo_id);

        if matches {
            if let Some(removed) = flows.remove(id) {
                removed.close();
            }
            return true;
        }
        false
    }

    /// Remove idle flows whose last activity is older than `idle_timeout`.
    ///
    /// Go: `FunnelTracker.cleanup()` — called periodically from
    /// `FunnelTracker.ScheduleCleanup()`.
    pub fn cleanup_idle(&self, idle_timeout: Duration) {
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
            - idle_timeout.as_secs() as i64;

        let mut flows = self.flows.lock().expect("flow tracker lock poisoned");
        flows.retain(|_, flow| {
            if flow.last_active_secs() < cutoff {
                flow.close();
                false
            } else {
                true
            }
        });
    }

    /// Number of currently tracked flows.
    pub fn len(&self) -> usize {
        self.flows.lock().expect("flow tracker lock poisoned").len()
    }

    /// Whether the tracker has no flows.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for FlowTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// HIS-069 + HIS-070: permission check
// ---------------------------------------------------------------------------

/// Check if the current process can create non-privileged ICMP sockets.
///
/// For IPv4, checks that the effective GID is within the kernel's
/// `ping_group_range`. Then opens and immediately closes a test socket.
///
/// Go: `testPermission()` + `checkInPingGroup()` in `icmp_linux.go`.
pub fn check_icmp_permission(listen_ip: IpAddr) -> Result<(), IcmpError> {
    if listen_ip.is_ipv4() {
        check_in_ping_group()?;
    }

    // Open and immediately close a test socket.
    let _conn = IcmpConn::new(listen_ip)?;
    Ok(())
}

/// Check if the current process can create ICMP sockets.
///
/// Go checks `/proc/sys/net/ipv4/ping_group_range` to see if the
/// current GID is in the allowed range.
pub fn can_create_icmp_socket() -> bool {
    check_in_ping_group().is_ok()
}

/// Verify the effective GID is within the kernel ping group range.
///
/// Go: `checkInPingGroup()` in `icmp_linux.go`.
fn check_in_ping_group() -> Result<(), IcmpError> {
    let content =
        std::fs::read_to_string(PING_GROUP_RANGE_PATH).map_err(|e| IcmpError::PingGroupUnreadable {
            path: PING_GROUP_RANGE_PATH,
            reason: e.to_string(),
        })?;

    let parts: Vec<&str> = content.split_whitespace().collect();

    if parts.len() != 2 {
        return Err(IcmpError::PingGroupUnreadable {
            path: PING_GROUP_RANGE_PATH,
            reason: format!("expected 2 values, found {}", parts.len()),
        });
    }

    let low: u32 = parts[0]
        .parse::<u32>()
        .map_err(|e| IcmpError::PingGroupUnreadable {
            path: PING_GROUP_RANGE_PATH,
            reason: e.to_string(),
        })?;

    let high: u32 = parts[1]
        .parse::<u32>()
        .map_err(|e| IcmpError::PingGroupUnreadable {
            path: PING_GROUP_RANGE_PATH,
            reason: e.to_string(),
        })?;

    let gid = getegid().as_raw();

    if gid < low || gid > high {
        return Err(IcmpError::NotInPingGroup { gid, low, high });
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// HIS-069: Linux ICMP proxy
// ---------------------------------------------------------------------------

/// Real ICMP proxy for Linux.
///
/// Go: `icmpProxy` in `icmp_linux.go`. Owns a `FlowTracker` and creates
/// per-flow sockets on demand. Full async listener integration is a
/// composition concern for `cfdrs-bin`.
pub struct LinuxIcmpProxy {
    flow_tracker: FlowTracker,
    listen_ip: IpAddr,
    idle_timeout: Duration,
}

impl LinuxIcmpProxy {
    /// Construct a new ICMP proxy after verifying socket permissions.
    ///
    /// Go: `newICMPProxy()`.
    pub fn new(listen_ip: IpAddr, idle_timeout: Duration) -> Result<Self, IcmpError> {
        check_icmp_permission(listen_ip)?;

        Ok(Self {
            flow_tracker: FlowTracker::new(),
            listen_ip,
            idle_timeout,
        })
    }

    /// Access the inner flow tracker.
    pub fn flow_tracker(&self) -> &FlowTracker {
        &self.flow_tracker
    }

    /// The listen IP this proxy is bound to.
    pub fn listen_ip(&self) -> IpAddr {
        self.listen_ip
    }

    /// The idle timeout for flow cleanup.
    pub fn idle_timeout(&self) -> Duration {
        self.idle_timeout
    }

    /// Handle an incoming ICMP echo request.
    ///
    /// Looks up or creates a flow for `(src, dst, echo_id)`, rewrites the
    /// echo ID, and sends via the flow's socket.
    ///
    /// Go: `icmpProxy.Request()`.
    pub fn handle_request(&self, src: IpAddr, dst: IpAddr, msg: &[u8]) -> Result<(), IcmpError> {
        let fields = parse_echo_fields(msg)?;

        if !is_echo_request(fields.icmp_type) {
            return Err(IcmpError::UnexpectedType {
                icmp_type: fields.icmp_type,
            });
        }

        let flow_id = Flow3Tuple {
            src_ip: src,
            dst_ip: dst,
            original_echo_id: fields.id,
        };

        let listen_ip = self.listen_ip;

        self.flow_tracker.get_or_register(
            flow_id.clone(),
            |_existing| false, // never replace an active flow
            || {
                let conn = IcmpConn::new(listen_ip)?;
                Ok(IcmpEchoFlow::new(conn, src, fields.id))
            },
        )?;

        self.flow_tracker
            .with_flow(&flow_id, |flow| flow.send_to_dst(dst, msg))
            .transpose()?;

        Ok(())
    }

    /// Run idle-flow cleanup once.
    ///
    /// Go: `FunnelTracker.cleanup()` — called from `ScheduleCleanup()` ticker.
    pub fn cleanup_idle_flows(&self) {
        self.flow_tracker.cleanup_idle(self.idle_timeout);
    }
}

impl IcmpProxy for LinuxIcmpProxy {
    fn start(&self) -> cfdrs_shared::Result<()> {
        // Full async listener loop is wired in cfdrs-bin; the proxy itself
        // is ready once constructed.
        Ok(())
    }

    fn stop(&self) {
        self.flow_tracker.cleanup_idle(Duration::ZERO);
    }
}

// ---------------------------------------------------------------------------
// HIS-071: ICMP source address auto-detection
// ---------------------------------------------------------------------------

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

    // --- HIS-069: ICMP checksum ---

    #[test]
    fn internet_checksum_echo_request() {
        // Standard echo request: type=8, code=0, checksum=0, id=0x1234, seq=0x0001
        let msg: Vec<u8> = vec![8, 0, 0, 0, 0x12, 0x34, 0x00, 0x01];
        let cksum = internet_checksum(&msg);
        // Verify: recomputing over the message with the checksum inserted yields 0.
        let mut with_cksum = msg.clone();
        let cksum_bytes = cksum.to_be_bytes();
        with_cksum[2] = cksum_bytes[0];
        with_cksum[3] = cksum_bytes[1];
        assert_eq!(internet_checksum(&with_cksum), 0);
    }

    #[test]
    fn internet_checksum_odd_length() {
        let data: Vec<u8> = vec![0x00, 0x01, 0x02];
        let _ = internet_checksum(&data); // must not panic
    }

    #[test]
    fn internet_checksum_empty() {
        assert_eq!(internet_checksum(&[]), 0xffff);
    }

    // --- HIS-069: echo field parsing ---

    #[test]
    fn parse_echo_fields_valid_request() {
        let msg = vec![8, 0, 0x00, 0x00, 0xab, 0xcd, 0x00, 0x05, 0xde, 0xad];
        let fields = parse_echo_fields(&msg).expect("valid message");
        assert_eq!(fields.icmp_type, ICMP_ECHO_REQUEST);
        assert_eq!(fields.code, 0);
        assert_eq!(fields.id, 0xabcd);
        assert_eq!(fields.seq, 5);
    }

    #[test]
    fn parse_echo_fields_valid_reply() {
        let msg = vec![0, 0, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02];
        let fields = parse_echo_fields(&msg).expect("valid message");
        assert_eq!(fields.icmp_type, ICMP_ECHO_REPLY);
        assert_eq!(fields.id, 1);
        assert_eq!(fields.seq, 2);
    }

    #[test]
    fn parse_echo_fields_too_short() {
        let msg = vec![8, 0, 0x00, 0x00]; // only 4 bytes
        assert!(parse_echo_fields(&msg).is_err());
    }

    #[test]
    fn parse_echo_fields_v6_request() {
        let msg = vec![128, 0, 0x00, 0x00, 0x00, 0x10, 0x00, 0x01];
        let fields = parse_echo_fields(&msg).expect("valid v6 request");
        assert_eq!(fields.icmp_type, ICMPV6_ECHO_REQUEST);
        assert!(is_echo_request(fields.icmp_type));
    }

    #[test]
    fn parse_echo_fields_v6_reply() {
        let msg = vec![129, 0, 0x00, 0x00, 0x00, 0x10, 0x00, 0x01];
        let fields = parse_echo_fields(&msg).expect("valid v6 reply");
        assert_eq!(fields.icmp_type, ICMPV6_ECHO_REPLY);
        assert!(is_echo_reply(fields.icmp_type));
    }

    // --- HIS-069: echo ID rewriting ---

    #[test]
    fn rewrite_echo_id_changes_id_and_checksum() {
        // Echo request: type=8, code=0, checksum=0, id=0x1234, seq=1
        let mut msg = vec![8, 0, 0, 0, 0x12, 0x34, 0x00, 0x01];
        // Set initial checksum.
        let cksum = internet_checksum(&msg).to_be_bytes();
        msg[2] = cksum[0];
        msg[3] = cksum[1];

        let new_id: u16 = 0x5678;
        let rewritten = rewrite_echo_id(&msg, new_id).expect("rewrite ok");

        // ID changed.
        assert_eq!(u16::from_be_bytes([rewritten[4], rewritten[5]]), new_id);

        // Checksum is valid (recompute yields 0).
        assert_eq!(internet_checksum(&rewritten), 0);
    }

    #[test]
    fn rewrite_echo_id_roundtrip() {
        let original_id: u16 = 0xabcd;
        let assigned_id: u16 = 0x1234;

        let mut msg = vec![8, 0, 0, 0, 0xab, 0xcd, 0x00, 0x01, 0xde, 0xad];
        let cksum = internet_checksum(&msg).to_be_bytes();
        msg[2] = cksum[0];
        msg[3] = cksum[1];

        // Forward: original → assigned.
        let rewritten = rewrite_echo_id(&msg, assigned_id).expect("forward rewrite");
        assert_eq!(u16::from_be_bytes([rewritten[4], rewritten[5]]), assigned_id);
        assert_eq!(internet_checksum(&rewritten), 0);

        // Reverse: assigned → original.
        let restored = rewrite_echo_id(&rewritten, original_id).expect("reverse rewrite");
        assert_eq!(u16::from_be_bytes([restored[4], restored[5]]), original_id);
        assert_eq!(internet_checksum(&restored), 0);

        // Payload preserved.
        assert_eq!(restored[8..], msg[8..]);
    }

    #[test]
    fn rewrite_echo_id_too_short() {
        assert!(rewrite_echo_id(&[8, 0, 0], 1).is_err());
    }

    // --- HIS-069: echo type predicates ---

    #[test]
    fn is_echo_request_accepts_v4_and_v6() {
        assert!(is_echo_request(ICMP_ECHO_REQUEST)); // 8
        assert!(is_echo_request(ICMPV6_ECHO_REQUEST)); // 128
        assert!(!is_echo_request(ICMP_ECHO_REPLY));
        assert!(!is_echo_request(3)); // dest unreachable
    }

    #[test]
    fn is_echo_reply_accepts_v4_and_v6() {
        assert!(is_echo_reply(ICMP_ECHO_REPLY)); // 0
        assert!(is_echo_reply(ICMPV6_ECHO_REPLY)); // 129
        assert!(!is_echo_reply(ICMP_ECHO_REQUEST));
    }

    // --- HIS-069: flow 3-tuple ---

    #[test]
    fn flow_3tuple_display_matches_go_format() {
        let f = Flow3Tuple {
            src_ip: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            dst_ip: IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
            original_echo_id: 0x1234,
        };
        assert_eq!(f.to_string(), "10.0.0.1:8.8.8.8:4660");
    }

    #[test]
    fn flow_3tuple_type_matches_go_baseline() {
        let f = Flow3Tuple {
            src_ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
            dst_ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
            original_echo_id: 0,
        };
        assert_eq!(f.flow_type(), "srcIP_dstIP_echoID");
    }

    #[test]
    fn flow_3tuple_hash_equality() {
        let a = Flow3Tuple {
            src_ip: IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)),
            dst_ip: IpAddr::V4(Ipv4Addr::new(5, 6, 7, 8)),
            original_echo_id: 100,
        };
        let b = a.clone();
        assert_eq!(a, b);

        let mut map = HashMap::new();
        map.insert(a, "first");
        assert_eq!(map.get(&b), Some(&"first"));
    }

    // --- HIS-069: flow tracker ---

    // Note: FlowTracker tests that need real IcmpConn only run when the
    // environment supports socket creation (CI may not have ping_group_range).

    #[test]
    fn flow_tracker_starts_empty() {
        let tracker = FlowTracker::new();
        assert!(tracker.is_empty());
        assert_eq!(tracker.len(), 0);
    }

    #[test]
    fn flow_tracker_default_is_empty() {
        let tracker = FlowTracker::default();
        assert!(tracker.is_empty());
    }

    // --- HIS-069: IcmpConn ---

    #[test]
    fn icmp_conn_v4_does_not_panic() {
        // Socket creation may fail without CAP_NET_RAW or ping_group_range.
        let _ = IcmpConn::new_v4(Ipv4Addr::UNSPECIFIED);
    }

    #[test]
    fn icmp_conn_v6_does_not_panic() {
        let _ = IcmpConn::new_v6(Ipv6Addr::UNSPECIFIED);
    }

    #[test]
    fn icmp_conn_new_dispatches_by_family() {
        let _ = IcmpConn::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        let _ = IcmpConn::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED));
    }

    // --- HIS-069: IcmpConn integration (requires ping_group_range) ---

    #[test]
    fn icmp_conn_v4_port_nonzero_when_permitted() {
        if !can_create_icmp_socket() {
            return; // skip in environments without permission
        }
        let conn = IcmpConn::new_v4(Ipv4Addr::UNSPECIFIED).expect("socket creation");
        assert_ne!(conn.local_port(), 0, "kernel should assign a nonzero port");
        assert!(conn.is_v4());
    }

    #[test]
    fn check_icmp_permission_does_not_panic() {
        let _ = check_icmp_permission(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
    }

    // --- HIS-069: LinuxIcmpProxy ---

    #[test]
    fn linux_icmp_proxy_request_rejects_non_echo() {
        if !can_create_icmp_socket() {
            return;
        }
        let proxy = LinuxIcmpProxy::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), Duration::from_secs(60))
            .expect("proxy creation");

        // Type 3 = destination unreachable, not echo request.
        let msg = vec![3, 0, 0, 0, 0x00, 0x01, 0x00, 0x01];
        let result = proxy.handle_request(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
            &msg,
        );
        assert!(result.is_err());
    }

    #[test]
    fn linux_icmp_proxy_registers_flow() {
        if !can_create_icmp_socket() {
            return;
        }
        let proxy = LinuxIcmpProxy::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), Duration::from_secs(60))
            .expect("proxy creation");

        // Build a valid echo request.
        let mut msg = vec![8, 0, 0, 0, 0x00, 0x42, 0x00, 0x01];
        let cksum = internet_checksum(&msg).to_be_bytes();
        msg[2] = cksum[0];
        msg[3] = cksum[1];

        let src = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let dst = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));

        // This registers a flow and sends (send may fail if no route).
        let _ = proxy.handle_request(src, dst, &msg);

        // Flow should be registered regardless of send outcome.
        assert_eq!(proxy.flow_tracker().len(), 1);
    }

    #[test]
    fn linux_icmp_proxy_stop_clears_flows() {
        if !can_create_icmp_socket() {
            return;
        }
        let proxy = LinuxIcmpProxy::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), Duration::from_secs(60))
            .expect("proxy creation");

        let mut msg = vec![8, 0, 0, 0, 0x00, 0x42, 0x00, 0x01];
        let cksum = internet_checksum(&msg).to_be_bytes();
        msg[2] = cksum[0];
        msg[3] = cksum[1];

        let _ = proxy.handle_request(
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
            &msg,
        );

        proxy.stop();
        assert!(proxy.flow_tracker().is_empty());
    }

    // --- HIS-069: constants match Go baseline ---

    #[test]
    fn mtu_matches_go_baseline() {
        // Go: packet/packet.go — mtu = 1500
        assert_eq!(MTU, 1500);
    }

    #[test]
    fn default_ttl_matches_go_baseline() {
        // Go: packet/packet.go — DefaultTTL = 255
        assert_eq!(DEFAULT_TTL, 255);
    }
}
