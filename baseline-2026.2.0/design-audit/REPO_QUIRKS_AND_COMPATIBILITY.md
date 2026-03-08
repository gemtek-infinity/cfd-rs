# Cloudflared Quirks And Compatibility Notes

This document isolates quirks, compatibility paths, deprecations, and behavioral edges that are easy to miss in a rewrite or audit.

## 1. Why This Exists

The repo contains many behaviors that are intentional but non-obvious:

- hidden flags preserved for scripts
- deprecated commands intentionally retained as explicit failures
- runtime defaults chosen for diagnostics discoverability
- protocol behavior constrained by feature support rather than pure preference

## 2. CLI Quirks

### 2.1 Empty Invocation Is Not Help

- empty `cloudflared` can create config and log directories and enter service mode

### 2.2 `cloudflared tunnel` Is Not Purely Namespace-Only

- direct invocation may create, route, or run depending on flags

### 2.3 Compatibility Placeholder Commands

- removed commands such as `db-connect` and `proxy-dns` remain as explicit removed-command placeholders
- this reduces silent script breakage but means command discovery must distinguish “registered” from “supported”

### 2.4 Hidden Flags Still Matter

- many hidden flags influence runtime behavior, internal tooling, or compatibility paths
- they are not safe to ignore during rewrite or breakage review

## 3. Config Quirks

### 3.1 Auto-Created Config File

- missing default config path may be created automatically by service mode

### 3.2 Unknown Config Keys Are Warning-Oriented

- not all unknown keys are hard failures because loader performs a second strict decode to collect warnings

### 3.3 CLI Single-Origin Flags Are Legacy-Scoped

- flags like `http-host-header` and `no-tls-verify` only fully apply in CLI single-origin mode when ingress rules are not used

### 3.4 `warp-routing.enabled` History

- historically present, now not supported for local config paths
- rewrite must not accidentally resurrect old semantics

## 4. Ingress Quirks

### 4.1 No-Rule Default Is 503, Not Localhost

- this changed historically and is easy to regress if one assumes old tunnel defaults

### 4.2 Internal Rules Use Negative Indices

- not just an implementation detail; diagnostics/log reasoning depends on distinguishing internal from user rules

### 4.3 Catch-All Rule Is Mandatory

- last rule semantics are an explicit invariant and not just a best practice

## 5. Protocol Quirks

### 5.1 PQ Implies QUIC

- PQ is not a general security toggle; it materially restricts transport choice

### 5.2 QUIC Success Memory And Fallback

- protocol choice is not recomputed naively on every failure; remembered-success behavior influences fallback dynamics

### 5.3 QUIC v2 Versus v3 Datagram Paths

- both exist and are not interchangeable
- v3 intentionally rejects some session RPC operations

### 5.4 macOS UDP Behavior

- QUIC code contains OS-specific UDP network selection for correct DF bit behavior

## 6. Management And Diagnostics Quirks

### 6.1 Diagnostics Default Changed Over Time

- management diagnostics were opt-in, then enabled by default in later versions

### 6.2 One Active Log Stream

- only one actor session is allowed at a time, except same actor may preempt its own prior session

### 6.3 Multiple Metrics Listeners Affect Diagnostics

- diag tooling expects known metrics-address search behavior and can surface multiple-instance ambiguity

## 7. Service And Packaging Quirks

### 7.1 Linux Update Service Uses Exit Code 11

- update timer restarts cloudflared only when updater exits with code 11

### 7.2 Container Runtime Changes Metrics Defaults

- `CONTAINER_BUILD=1` changes metrics runtime to `virtual`, which changes bind defaults and discoverability assumptions

### 7.3 Root Versus User Launchd Behavior

- macOS service install semantics differ materially by privilege level

## 8. Historical Compatibility Milestones

High-value entries from release history:

- 2026.2.0: `proxy-dns` removed
- 2024.12.1: metrics known-port probing behavior
- 2024.10.0: QUIC grace-period fix
- 2024.2.1: diagnostics default-on
- 2023.3.2: quick tunnels single connection
- 2023.3.1: no-ingress default 503
- 2023.2.2: legacy tunnels and h2mux unsupported
- 2022.8.1: remembered successful protocol behavior
- 2022.3.0: `unix+tls:` origin support

## 9. Reconnect And Protocol Quirks

### 9.1 Reconnect Signal Bypasses Backoff

- `ReconnectSignal` on the reconnect channel causes immediate tunnel restart with optional delay.
- This intentionally bypasses the normal exponential backoff to allow external control signals to trigger fast reconnection.

### 9.2 HasConnectedWith Prevents Fallback

- If ANY connection has successfully used the current protocol (tracked by `ConnTracker`), the supervisor will NOT fall back to a different protocol even after max retries.
- This prevents transient QUIC failures from permanently demoting to HTTP/2 when QUIC is known to work.
- This is invisible to operators and could cause confusion when troubleshooting seemingly-stuck connections.

### 9.3 Reconnect Token Usage

- `--use-reconnect-token` flag controls whether reconnect tokens cache connection state.
- Reconnect tokens allow faster reconnection by reusing registration state.

### 9.4 DNS Resolver Address Override

- `--dns-resolver-addrs` overrides virtual DNS resolution targets for WARP routing.
- Only affects private routing flows, not tunnel DNS resolution itself.

## 10. SOCKS And Bastion Quirks

### 10.1 SOCKS BIND And ASSOCIATE Not Implemented

- SOCKS5 BIND (command 2) and ASSOCIATE (command 3) return `commandNotSupported`.
- Source code contains TODO comments for future implementation.
- Scripts depending on these SOCKS features will fail silently at the SOCKS level.

### 10.2 FQDN Resolution Happens Before IP Access Check

- In SOCKS CONNECT, if the destination is an FQDN, it is resolved to an IP before the IP access policy is evaluated.
- Resolution failure returns `ruleFailure` SOCKS code, which looks like a policy denial even though it was a DNS issue.

### 10.3 Bastion Destination Header Is Authentication-Free

- `Cf-Access-Jump-Destination` header controls where bastion mode connects.
- The header value is trusted as-is; access control is assumed to be handled at the edge level.
- A missing header returns an error rather than a default destination.

## 11. Access Token Quirks

### 11.1 Token Lock Can Be Force-Deleted

- After 7 retries of exponential backoff waiting for a lock file, the lock is force-deleted.
- This handles stale locks from crashed processes but could theoretically race with another process.

### 11.2 Expired Tokens Are Silently Deleted

- When checking for an existing token, if `jwtPayload.Exp` is in the past, the token file is deleted from disk.
- No notification is given; the next access attempt triggers a full auth flow.

### 11.3 Management Token Lifecycle

- Management tokens are obtained via REST API (`GetManagementToken`) with specific resource scopes.
- Tokens have limited lifetime and are not cached across invocations.
- `tail` command auto-acquires management token if `--token` is not provided.

## 12. Edge Discovery Quirks

### 12.1 Preferred Address Stickiness

- `GetAddr(connIndex)` prefers the previously-used address for a given connection index.
- This means reconnecting tunnels tend to hit the same edge server, which may or may not be desirable.

### 12.2 DoT Fallback Is Not Logged Prominently

- DNS-over-TLS fallback activates silently when standard DNS fails.
- Operators may not realize their standard DNS is failing.

## 13. Feature Flag Quirks

### 13.1 Deprecated Features Auto-Filtered

- `dedupAndRemoveFeatures()` silently removes deprecated feature flags.
- If an operator or script passes a deprecated feature via `--features`, it is removed without error.

### 13.2 Datagram Version Rollout Is Percentage-Based

- Remote feature evaluation uses percentage thresholds to gradually roll out datagram v3.
- The specific percentage is fetched remotely, making behavior non-deterministic across installations.

## 14. Wire Format And Transport Quirks

### 14.1 V2 Suffix Encoding Is Performance-Motivated

- V2 datagrams suffix sessionID and type (instead of prefixing) to avoid copying the payload into a new buffer.
- This means parsing a V2 datagram requires reading from the end, not the beginning — a non-obvious design that would be missed in a naive rewrite.

### 14.2 V3 RequestID Is Not A UUID

- V3 uses a custom `uint128` type for RequestID, NOT a UUID. The serialization format (big-endian hi/lo) differs from UUID byte ordering.
- Code that assumes UUID compatibility will subtly corrupt session identifiers.

### 14.3 HTTP/2 Rewrites 101 To 200

- HTTP/2 spec forbids 101 Switching Protocols. cloudflared silently rewrites 101 → 200 on HTTP/2 connections.
- A rewrite that doesn't do this will break WebSocket upgrade semantics over HTTP/2.

### 14.4 Header Serialization Uses Raw Base64 (No Padding)

- `base64.RawStdEncoding` (no `=` padding) is used for header serialization.
- Standard base64 encoding will fail deserialization at the edge.

### 14.5 Management JWT Not Verified Locally

- Management tokens use `UnsafeClaimsWithoutVerification` — the JWT signature is NOT checked locally.
- Verification happens at the edge. A rewrite adding local verification would break the auth flow.

### 14.6 macOS QUIC Uses Explicit UDP4/UDP6

- On macOS (darwin), QUIC connections use `"udp4"` or `"udp6"` explicitly instead of the generic `"udp"` network.
- This works around a quic-go DF (Don't Fragment) bit bug specific to macOS.
- Removing this platform-specific behavior will break macOS QUIC connections.

### 14.7 Pre-Generated ResponseMeta Constants

- ResponseMeta JSON values are pre-generated at init time, not dynamically serialized per-request.
- Performance-optimized but means the JSON shape is fixed — any field addition requires updating the constants.

### 14.8 Management Idle Timeout Code 4003

- WebSocket close code `4003` (`StatusIdleLimitExceeded`) was added for idle timeout beyond the original `4001`/`4002`.
- The idle timeout is 5 minutes. Heartbeat pings every 15 seconds keep the connection alive.

## 15. Proxy And Routing Quirks

### 15.1 LB Probe Detection By User-Agent Prefix

- Load balancer probes are detected by exact User-Agent prefix match: `"Mozilla/5.0 (compatible; Cloudflare-Traffic-Manager/1.0;"`.
- Any change to this prefix string silently breaks LB probe detection.

### 15.2 Missing URL Parts Filled With Defaults

- For TypeHTTP on HTTP/2, missing `r.URL.Scheme` defaults to `"http"` and missing `r.URL.Host` defaults to `"localhost:8080"`.
- These defaults are invisible to operators but critical for correct proxying.

### 15.3 Tag Header Prefix Is Not Configurable

- The `Cf-Warp-Tag-` prefix is hardcoded. Tags cannot use arbitrary header names.

### 15.4 IPv4-Mapped-IPv6 Normalization In V2 Datagrams

- When V2 datagram sessions receive destination IPs from the Cap'n Proto RPC layer, IPv4 addresses arrive as IPv4-mapped-IPv6 (e.g., `::ffff:1.2.3.4`).
- The code explicitly calls `To4()` to normalize them back to pure IPv4 before creating `netip.AddrPort`.
- V3 avoids this entirely by using a flag bit to distinguish IPv4 from IPv6, with fixed-size address fields (4 or 16 bytes).
- A rewrite using V2 RPC-based sessions must include this normalization or addresses will be 16 bytes when 4 are expected.

### 15.5 Punycode Dual-Matching In Ingress Rules

- Ingress rule matching checks both the original Unicode hostname AND its punycode (ASCII) equivalent.
- This is a subtle compatibility detail: a rewrite that only matches one form will silently fail to route IDN hostnames.

### 15.6 vulncheck Is Not govulncheck

- The `make vulncheck` target delegates to `.ci/scripts/vuln-check.sh`, not a direct `govulncheck` invocation.
- The script may have additional filtering or CI-specific behavior.
- `r2-release` appears in release notes but is NOT a Makefile target — it's a CI/CD pipeline concept.

## 16. Compatibility Review Prompts

- Is this path hidden, deprecated, or compatibility-only rather than dead?
- Is this behavior tied to diagnostics/support tooling rather than mainline traffic?
- Did a release note intentionally change this behavior in the past?
- Would a rewrite remove this because it looks odd even though operators or scripts rely on it?
- Does this behavior depend on HasConnectedWith() state that accumulates over the process lifetime?
- Does this behavior depend on file-system lock mechanics that differ across OSes?
- Does this SOCKS behavior rely on IP resolution before policy check?
- Does this wire format use suffix vs prefix encoding?
- Does this datagram handler assume UUID vs uint128 for session identifiers?
- Does this HTTP/2 handler rewrite status codes that the spec forbids?
- Does this base64 encoding use padding or not?
- Does this JWT handling verify signatures locally or trust the edge?
- Is this platform-specific network behavior needed for macOS compatibility?
- Does this ingress rule match both Unicode and punycode hostnames?
- Does this V2 code normalize IPv4-mapped-IPv6 addresses from capnp?
