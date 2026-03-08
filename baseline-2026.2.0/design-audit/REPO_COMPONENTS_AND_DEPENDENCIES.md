# Cloudflared Components, Technologies, And Dependency Map

This appendix expands the architectural and external-component coverage into a repository inventory suitable for onboarding, risk analysis, and rewrite planning.

Path scope note: component and file paths in this appendix refer to the Go
reference implementation rooted at `old-impl/`.

## 1. First-Party Component Inventory

### 1.1 Runtime-Critical Components

| Component | Responsibility | Primary Risks |
| --- | --- | --- |
| `cmd/cloudflared/tunnel` | daemon startup, runtime assembly, signal/shutdown, CLI bridging | precedence errors, startup regression, shutdown bugs |
| `supervisor` | HA edge connections, reconnect, fallback, edge lifecycle | reconnect loops, protocol fallback regressions, race conditions |
| `connection` | transport implementations and edge stream/datagram handling | QUIC/HTTP2 divergence, flow control, stream correctness |
| `orchestration` | hot config application and proxy ownership | stale config, lock/atomic misuse, update semantics |
| `ingress` | matching, validation, origin choice | misrouting, invalid defaulting, wildcard/path bugs |
| `proxy` | request/stream forwarding to origins | truncation, timeout, header propagation bugs |
| `datagramsession` | UDP/ICMP flow multiplexing | session leak, idle cleanup, packet ordering bugs |
| `flow` | active-flow limiting | incorrect exhaustion or under-enforcement |
| `metrics` | health and metrics surface | false readiness, accidental exposure, bind conflicts |
| `management` | remote log and diagnostics management plane | auth gating, concurrent session limits, remote debug exposure |
| `tunnelrpc` | RPC schema and adapters | protocol compatibility breakage |
| `tunnelstate` | active-connection truth source | stale readiness or diagnostics data |
| `config` | config search/load/schema | precedence and migration bugs |
| `credentials` | cert and tunnel credential discovery | auth failures, lookup surprises |

### 1.2 Support And Tooling Components

| Component | Responsibility |
| --- | --- |
| `cmd/cloudflared/access` | Access user workflows |
| `cmd/cloudflared/tail` | remote log streaming client |
| `cmd/cloudflared/management` | management JWT helper |
| `logger` | structured logging setup |
| `tracing` | telemetry and trace propagation |
| `diagnostic` | troubleshooting report and collectors |
| `watcher` | config file watch support |
| `retry` | backoff primitives |
| `tlsconfig` | TLS config construction |
| `carrier` | Access TCP-over-WebSocket forwarding |
| `hello` | built-in hello-world origin |
| `socks` | SOCKS proxy support |
| `sshgen` | Access SSH helper cert generation |

### 1.3 Integration Components

| Component | Responsibility |
| --- | --- |
| `cfapi` | Cloudflare REST API client surface |
| `edgediscovery` | edge address discovery and protocol percentage selection |
| `features` | feature-flag selection and deprecated-feature removal |
| `packet` | packet encoding/decoding for private routing |
| `ipaccess` | IP allow/deny policy enforcement |
| `component-tests` | end-to-end externally visible behavior tests |

## 2. External Libraries And Why They Matter

### 2.1 Direct Dependencies With Architectural Importance

| Dependency | Role In Repo | Notes |
| --- | --- | --- |
| `github.com/quic-go/quic-go` | QUIC transport | replaced with Cloudflare-oriented fork |
| `github.com/urfave/cli/v2` | CLI framework | replaced with fork |
| `zombiezen.com/go/capnproto2` | RPC schema/runtime | canonical tunnel control-plane serialization |
| `github.com/prometheus/client_golang` | metrics | local and remote metrics exposure |
| `go.opentelemetry.io/otel` | tracing | distributed tracing and exporters |
| `github.com/rs/zerolog` | logging | structured logging standard across repo |
| `nhooyr.io/websocket` | management WebSocket | remote log streaming and management handling |
| `github.com/gorilla/websocket` | websocket support in other paths | additional WS handling |
| `github.com/coreos/go-systemd/v22` | Linux systemd support | notifications and service integration |
| `github.com/facebookgo/grace` | graceful listener support | metrics listener integration |
| `gopkg.in/yaml.v3` | YAML config parse | central config schema layer |
| `go.uber.org/automaxprocs` | CPU quota awareness | important in container runtime behavior |
| `github.com/google/uuid` | connector and tunnel IDs | appears in multiple contracts |
| `github.com/google/gopacket` | packet parsing | private routing and datagrams |

### 2.2 Replaced And Forked Dependencies

These are especially important for rewrites and vendor updates.

| Replace | Replacement | Why It Matters |
| --- | --- | --- |
| `github.com/urfave/cli/v2` | `github.com/ipostelnik/cli/v2` | CLI behavior is not necessarily stock upstream |
| `github.com/quic-go/quic-go` | `github.com/chungthuang/quic-go` | QUIC behavior may differ from upstream `quic-go` |
| `github.com/prometheus/golang_client` | patched version | explicit CVE avoidance |

### 2.3 Security And Compliance Dependencies

| Dependency/Mechanism | Role |
| --- | --- |
| boringcrypto symbol checks in `check-fips.sh` | validates FIPS build properties |
| `golang.org/x/crypto` | cryptographic primitives |
| `github.com/go-jose/go-jose/v4` | JWT parsing and validation for Access tokens (used by `token` and `validation` packages) |
| FIPS build tags and static linking | FIPS artifact behavior |

## 3. External Services And Protocols

### 3.1 Cloudflare-Managed Services Used By The Repo

| Service | Usage |
| --- | --- |
| Cloudflare edge tunnel endpoints | primary tunnel data path |
| Cloudflare API v4 | tunnel, route, vnet, token, management operations |
| `management.argotunnel.com` | log streaming and remote diagnostics |
| fedramp management hostname | management plane variant for FedRAMP endpoint users |
| `api.trycloudflare.com` | quick-tunnel service |
| update service endpoints | binary update workflow |

### 3.2 Protocols And Formats

| Protocol/Format | Role |
| --- | --- |
| QUIC | primary edge transport |
| HTTP/2 | fallback edge transport |
| WebSocket | management logs, some proxy flows |
| Cap'n Proto RPC | control-plane registration and configuration |
| YAML | user config format |
| JSON | management responses, tokens, diagnostics, config rendering |
| Prometheus exposition format | metrics |
| PEM-wrapped JSON | origin cert/token storage |

## 4. Build And Toolchain Inventory

### 4.1 Build Toolchain

| Tool | Role |
| --- | --- |
| Go 1.24 | primary language/toolchain |
| GNU Make | canonical build entrypoint |
| `capnp` and `capnpc-go` | Cap'n Proto code generation |
| `golangci-lint` | lint gate |
| `goimports` | formatting |
| `go generate` + `go.uber.org/mock` | mocks |

### 4.2 Key Make Targets

| Target | Meaning |
| --- | --- |
| `cloudflared` | build binary |
| `test` | run race-enabled unit tests |
| `lint` | run golangci-lint |
| `vet` | run `go vet` |
| `cover` | build coverage report |
| `fuzz` | run fuzz targets |
| `capnp` | regenerate Cap'n Proto outputs |
| `fmt` | format code |
| `fmt-check` | CI formatting check |
| `mocks` | regenerate mocks |
| `ci-build`, `ci-test`, `ci-fips-build`, `ci-fips-test` | CI-oriented wrappers |

### 4.3 Container Build Behavior

Container build details from `Dockerfile`:

- builder image: `golang:1.24.13`
- `CONTAINER_BUILD=1` is set during build
- this causes `metrics.Runtime=virtual`
- final image: distroless Debian nonroot
- entrypoint: `cloudflared --no-autoupdate`

Operational implication:

- containerized runtime changes metrics bind defaults and exposure assumptions

### 4.4 FIPS Build Behavior

FIPS-related behavior:

- `FIPS=true` changes build tags and link mode
- binary name may become `cloudflared-fips`
- `check-fips.sh` asserts boringcrypto and FIPS-only symbols in built binary

## 5. Test Inventory

### 5.1 Unit And Fuzz Tests

The repo uses:

- package-level Go unit tests
- race detector under `make test`
- fuzz tests for `packet`, `quic/v3`, `tracing`, `validation`

### 5.2 Component Tests

Component tests are Python-based and depend on real Cloudflare resources and tunnel config.

Python dependencies from `component-tests/requirements.txt`:

- `cloudflare==2.14.3`
- `flaky==3.7.0`
- `pytest==7.3.1`
- `pytest-asyncio==0.21.0`
- `pyyaml==6.0.1`
- `requests==2.28.2`
- `retrying==1.3.4`
- `websockets==11.0.1`

Behavioral value of component tests:

- they represent the closest thing to an external contract test suite in this repo
- they are particularly relevant for quick tunnels, management, reconnect, service behavior, and token flows

## 6. Packaging And Release Components

| File | Role |
| --- | --- |
| `postinst.sh` | package post-install hooks |
| `postrm.sh` | package removal hooks |
| `cloudflared.wxs` | MSI packaging |
| `wix.json` | Windows installer config |
| `github_release.py` | release publishing workflow |
| `github_message.py` | release message workflow |
| `release_pkgs.py` | package publishing |
| `Dockerfile*` | container builds |

## 7. Architectural Risk Notes By Dependency Area

### 7.1 QUIC Area

High risk because it combines:

- forked dependency
- OS-specific UDP behavior
- flow-control tuning knobs
- post-quantum requirements
- datagram v2/v3 split

### 7.2 CLI Area

High risk because it combines:

- forked CLI framework
- hidden compatibility flags
- empty invocation side effects
- service-mode behavior at root level

### 7.3 Management And Diagnostics Area

High risk because it combines:

- remotely reachable control/debug surfaces
- token-gated access control
- single-session streaming semantics
- local and remote metrics exposure paths

### 7.4 Packaging And Service Area

High risk because it combines:

- OS-specific service managers
- autoupdate restart logic
- root vs non-root behavior
- container/runtime default differences

## 8. Wire-Critical Constants For Rewrite

These constants are embedded in wire protocols and must be preserved exactly:

| Constant | Value | Location | Purpose |
| --- | --- | --- | --- |
| Data stream signature | `0x0A 0x36 0xCD 0x12 0xA1 0x3E` | `tunnelrpc/quic/protocol.go` | QUIC stream identification |
| RPC stream signature | `0x52 0xBB 0x82 0x5C 0xDB 0x65` | `tunnelrpc/quic/protocol.go` | QUIC stream identification |
| Protocol version | `"01"` (2 bytes) | `tunnelrpc/quic/protocol.go` | Stream version |
| QUIC ALPN | `"argotunnel"` | `connection/protocol.go` | TLS negotiation |
| HTTP/2 server name | `"h2.cftunnel.com"` | `connection/protocol.go` | TLS SNI |
| QUIC server name | `"quic.cftunnel.com"` | `connection/protocol.go` | TLS SNI |
| MaxConcurrentStreams | `4294967295` | `connection/connection.go` | HTTP/2 setting |
| MaxGracePeriod | 3 minutes | `connection/connection.go` | Shutdown grace |
| LB probe user-agent | `"Mozilla/5.0 (compatible; Cloudflare-Traffic-Manager/1.0;"` | `connection/connection.go` | Probe detection |
| Datagram max payload | 1280 (unix), 1200 (windows) | `quic/param_*.go` | Datagram framing |
| V3 idle timeout | 210 seconds | `quic/v3/session.go` | Session cleanup |
| Management idle | 5 minutes | `management/session.go` | WebSocket timeout |
| Management heartbeat | 15 seconds | `management/session.go` | Ping interval |
| Tag header prefix | `"Cf-Warp-Tag-"` | `proxy/proxy.go` | Request tagging |

## 9. Rewrite And Upgrade Considerations

When upgrading dependencies or replacing subsystems, prioritize review of:

- CLI fork assumptions
- QUIC fork assumptions
- Cap'n Proto schema compatibility
- management WebSocket semantics
- Prometheus and metrics bind behavior
- FIPS build checks and binary naming
- container runtime behavior caused by `CONTAINER_BUILD`
- header serialization format (base64.RawStdEncoding, no padding)
- QUIC stream protocol signatures (exact 6 bytes + 2-byte version)
- datagram V2 suffix vs V3 prefix encoding byte order
- V3 RequestID uint128 type (not UUID)
- management JWT UnsafeClaimsWithoutVerification pattern
- ResponseMeta pre-generated JSON constants
- macOS-specific UDP network type ("udp4"/"udp6" vs "udp")
