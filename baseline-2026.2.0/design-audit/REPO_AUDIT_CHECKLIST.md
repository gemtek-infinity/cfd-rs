# Cloudflared Audit Checklist

This checklist is for three jobs:

1. Contract breakage analysis.
2. Bug and quirk hunting.
3. Rewrite preservation review.

Use it together with `REPO_REFERENCE.md` and `REPO_SOURCE_INDEX.md`.

Path scope note:

- All code and test paths referenced in this checklist are paths inside `old-impl/`.
- The checklist itself lives in `setpoint-docs-2026.2.0/`.

## 1. Breaking Change Checklist

Treat a change as potentially breaking if it alters any of the following.

### 1.1 CLI Surface

- Top-level command names.
- Subcommand names.
- Flag names or aliases.
- Environment-variable bindings.
- Default values.
- Hidden compatibility flags that scripts may still rely on.
- Empty invocation behavior.
- Service install behavior or generated service templates.

### 1.2 Config Surface

- Config search directories or filename order.
- Auto-create behavior for missing config in service mode.
- YAML key names.
- Meaning of `originRequest` subkeys.
- `warp-routing` semantics.
- ingress matching order or validation rules.
- default no-ingress behavior.

### 1.3 Endpoint Surface

- metrics endpoint list.
- readiness response shape and status semantics.
- management endpoint list.
- token requirements for management access.
- WebSocket close codes and failure reasons.

### 1.4 Transport And Wire Surface

- protocol selector values.
- fallback rules.
- QUIC/HTTP2 incompatibility rules.
- RPC schema fields or method signatures.
- QUIC stream magic/version framing.

### 1.5 Runtime Behavior

- quick-tunnel connection count.
- grace-period semantics.
- readiness definition.
- internal-rule precedence.
- management diagnostics default exposure.
- metrics bind selection order.

## 2. Invariants To Preserve

These are the highest-value repository invariants.

- Last ingress rule must be catch-all.
- Internal ingress rules are checked before user rules.
- Readiness is true only when at least one active edge connection exists.
- No local ingress rules defaults to HTTP 503 behavior, not localhost proxying.
- Quick tunnels are explicit and not implicit.
- Quick tunnels run a single edge connection.
- Post-quantum implies QUIC and cannot use HTTP/2 transport.
- Management log streaming allows at most one active actor session at a time, except self-preemption.
- Metrics listener tries known addresses first, then random port.
- Empty root invocation is service behavior, not help behavior.

## 3. Bug-Hunt Checklist By Subsystem

### 3.1 CLI And Config

- Are CLI/config precedence rules still consistent?
- Do hidden flags still parse without affecting visible behavior unexpectedly?
- Do deprecated flags still behave as documented, including explicit no-op behavior?
- Can empty invocation create files or directories in unexpected contexts?
- Are credentials-token precedence rules unchanged?

### 3.2 Ingress And Proxying

- Can malformed ingress rules bypass catch-all validation?
- Are wildcard hostnames validated and matched consistently?
- Is `--url` rejected when multi-origin ingress is used?
- Does no-rule behavior still return 503 rather than accidentally forwarding to a default local service?
- Do internal management rules remain isolated from user-defined rule order?

### 3.3 Supervisor And Shutdown

- Does graceful shutdown stop new work and wait correctly for in-flight work?
- Are repeated signals handled deterministically?
- Are reconnect and fallback loops bounded correctly?
- Can QUIC and HTTP/2 diverge in shutdown semantics again?

### 3.4 QUIC, HTTP/2, And Datagrams

- Does explicit protocol choice incorrectly fall back?
- Can PQ mode accidentally select HTTP/2?
- Are hidden QUIC tuning flags validated?
- Do datagram v2 and v3 assumptions leak into each other?
- Are unsupported v3 session RPCs rejected cleanly?

### 3.5 Metrics And Management

- Is diagnostics exposure gated correctly?
- Can metrics bind to a more public interface than intended?
- Does `/ready` ever report healthy without active connections?
- Can multiple log-stream sessions start concurrently?
- Are WebSocket close codes and idle/session-limit behavior preserved?

### 3.6 Private Routing

- Are overlapping CIDRs correctly isolated by virtual network?
- Is default virtual-network behavior stable?
- Can force-delete of virtual network orphan or misroute dependent resources?
- Are flow limits enforced and observable?

## 4. Rewrite Review Checklist

Before accepting a rewrite, verify all of the following.

### 4.1 Surface Compatibility

- CLI help and parsing remain compatible.
- Config keys and defaults remain compatible.
- Metrics and management endpoints remain compatible.
- RPC schema compatibility is explicitly maintained or intentionally versioned.

### 4.2 Semantic Compatibility

- Startup order preserves diagnostics, metrics, and supervisor behavior.
- Shutdown order preserves grace-period semantics.
- Ingress matching and defaulting preserve current outcomes.
- Protocol selection, fallback, and remembered-success behavior are preserved.
- quick-tunnel behavior is preserved.

### 4.3 Operational Compatibility

- Linux/macOS/Windows service installation still emits valid service definitions.
- metrics known-address discovery still works for diagnostics.
- log streaming still enforces single-active-session semantics.
- remote diagnostics remain opt-in or default-on exactly as intended by current release policy.

## 5. Regression Test Focus

When reviewing risky changes, run or inspect tests around:

- `config/configuration_test.go`
- `ingress/ingress_test.go`
- `connection/protocol_test.go`
- `connection/http2_test.go`
- `connection/quic_connection_test.go`
- `connection/quic_datagram_v2_test.go`
- `datagramsession/*_test.go`
- `diagnostic/diagnostic_utils_test.go`
- `cfapi/*_test.go`
- `component-tests/test_tunnel.py`
- `component-tests/test_management.py`
- `component-tests/test_quicktunnels.py`
- `component-tests/test_reconnect.py`

## 6. High-Risk Files

If you can only inspect a few files for a risky change, start here:

- `cmd/cloudflared/main.go`
- `cmd/cloudflared/tunnel/cmd.go`
- `cmd/cloudflared/tunnel/subcommands.go`
- `config/configuration.go`
- `ingress/ingress.go`
- `management/service.go`
- `metrics/metrics.go`
- `metrics/readiness.go`
- `connection/protocol.go`
- `connection/http2.go`
- `connection/quic.go`
- `supervisor/tunnel.go`
- `tunnelrpc/proto/tunnelrpc.capnp`
