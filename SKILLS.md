# Cloudflared Go → Rust Rewrite Skill

This skill defines the workflow for porting cloudflared subsystems from Go
(2026.2.0) to Rust, targeting `x86_64-unknown-linux-gnu` as the primary
platform.

- **Go reference**: `baseline-2026.2.0/old-impl/` (read-only, for behavioral comparison)
- **Behavioral spec**: `baseline-2026.2.0/design-audit/REPO_*.md`
- **Target triple**: `x86_64-unknown-linux-gnu`
- **Rust edition**: 2024
- **Rust workspace version rule**: `<go-release>-alpha.YYYYmm`

The Rust workspace version is coupled to the Go compatibility baseline. For the
current baseline, use `2026.2.0-alpha.202603`.

Do not edit `baseline-2026.2.0/old-impl/` or `baseline-2026.2.0/design-audit/` as part of normal rewrite
execution. Treat both as frozen inputs and build Rust-side fixes around them.

## Rewrite Workflow Per Subsystem

For each subsystem being ported, follow these steps in order:

### Step 1 — Read The Spec

1. Open `baseline-2026.2.0/design-audit/REPO_REFERENCE.md` and find the subsystem
2. Read the behavioral spec in `baseline-2026.2.0/design-audit/REPO_BEHAVIORAL_SPEC.md`
3. Read the wire/API contracts in `baseline-2026.2.0/design-audit/REPO_API_CONTRACTS.md`
4. Check `baseline-2026.2.0/design-audit/REPO_QUIRKS_AND_COMPATIBILITY.md` for quirks
5. Review diagrams in `baseline-2026.2.0/design-audit/REPO_DIAGRAMS.md`

### Step 2 — Read The Go Source

1. Use `baseline-2026.2.0/design-audit/REPO_SOURCE_INDEX.md` to find Go files and tests
2. Read `baseline-2026.2.0/old-impl/{package}/` focusing on:
   - Public interface / exported types
   - Error handling paths and error types
   - Concurrency model (goroutines, channels, mutexes)
   - Context cancellation behavior
   - Wire format encoding/decoding (if applicable)
   - Linux-specific code paths (primary target)

### Step 3 — Design The Rust Module

1. Start from the existing crate layout; do not invent a new top-level crate
   split unless the repository governance is updated first
2. Identify the owning crate for the slice using `STATUS.md` and
   `docs/dependency-policy.md`
3. For concurrency-sensitive work, follow
   `docs/go-rust-semantic-mapping.md` and
   `docs/adr/0001-hybrid-concurrency-model.md`
4. For allocator and runtime concerns, follow
   `docs/allocator-runtime-baseline.md`
5. Add dependencies only when the owning slice has started and the dependency
   passes the admission rules in `docs/dependency-policy.md`
6. Defer non-Linux platform code — use `#[cfg(target_os = "linux")]` where needed

### Step 4 — Implement With Contract Tests

1. Write contract tests FIRST based on `REPO_BEHAVIORAL_SPEC.md`
2. Implement the Rust code to pass those tests
3. For wire formats: write byte-level round-trip tests using exact values from
   `REPO_API_CONTRACTS.md` and `REPO_COMPONENTS_AND_DEPENDENCIES.md` §8

### Step 5 — Verify Parity

1. Build both: `cd old-impl && make cloudflared` and `cargo build --release --target x86_64-unknown-linux-gnu`
2. Run Go component tests against the Rust binary
3. Compare Prometheus metrics output (same names, same labels)
4. Compare CLI `--help` output (identical flag names and defaults)
5. For wire-format subsystems: capture Go output bytes, verify Rust matches

## Subsystem Porting Order

Recommended order based on dependency graph (implement bottom-up).
Go packages reference `baseline-2026.2.0/old-impl/`, spec references point to
`baseline-2026.2.0/design-audit/REPO_*.md`.

Unless otherwise stated, every Go package or file path in the tables below is
relative to `baseline-2026.2.0/old-impl/`.

### Phase 1 — Foundation (no cloudflared-specific deps)

| Priority | Subsystem | Go Package | Spec Reference | Key Contracts |
| --- | --- | --- | --- | --- |
| 1.1 | Config, credentials, and ingress normalization | `config/`, `credentials/`, `ingress/` | Behavioral §3, Behavioral §6, Config §1-§8 | YAML schema, cert.pem, UUID.json, ingress validation, no-ingress default |
| 1.2 | CLI framework | `cmd/cloudflared/` | CLI §1-§9 | All flags, env vars, defaults |
| 1.3 | Logging | `logger/` | Behavioral §2 | zerolog → tracing |
| 1.4 | Metrics server | `metrics/` | API §2, Behavioral §9 | /metrics, /ready, port fallback |
| 1.5 | Validation and TLS | `validation/`, `tlsconfig/` | Config §11, Components §2 | TLS pools, rustls |

### Phase 2 — Network and Protocol

| Priority | Subsystem | Go Package | Spec Reference | Key Contracts |
| --- | --- | --- | --- | --- |
| 2.1 | Edge discovery | `edgediscovery/` | Behavioral §19, API §14 | SRV DNS, address pool |
| 2.2 | QUIC transport | `quic/`, `connection/quic.go` | Behavioral §26-§27, API §17 | ALPN, port reuse, magic bytes |
| 2.3 | HTTP/2 transport | `connection/http2.go` | Behavioral §23-§24, API §16 | Stream dispatch, 101→200 |
| 2.4 | Datagram V2 | `connection/quic_datagram_v2.go` | Behavioral §28, API §18 | Suffix encoding, IPv4 normalization |
| 2.5 | Datagram V3 | `connection/quic_datagram_v3.go` | Behavioral §29, API §19 | Prefix encoding, flag-based IP |
| 2.6 | Cap'n Proto RPC | `tunnelrpc/` | API §6, §15 | Registration, session, config RPCs |
| 2.7 | Connection abstraction | `connection/` | Architectural §4, API §7 | TunnelConnection trait |

### Phase 3 — Runtime Core

| Priority | Subsystem | Go Package | Spec Reference | Key Contracts |
| --- | --- | --- | --- | --- |
| 3.1 | Ingress and routing | `ingress/` | Behavioral §6, Quirks §4 | Match order, punycode, catch-all |
| 3.2 | Proxy | `proxy/` | Behavioral §7-§8, Arch §19 | HTTP proxy, TCP proxy, tags |
| 3.3 | Orchestrator | `orchestration/` | Behavioral §12, Arch §18 | Copy-on-write, start-before-stop |
| 3.4 | Supervisor | `supervisor/` | Behavioral §14, Arch §11 | HA connections, reconnect, fallback |
| 3.5 | Datagram sessions | `datagramsession/` | Behavioral §21, Arch §15 | Session lifecycle, idle timeout |
| 3.6 | Flow control | `flow/` | Arch §4 | Limiter for active streams |
| 3.7 | Management service | `management/` | Behavioral §10, API §3 | WebSocket logs, JWT auth |

### Phase 4 — Auxiliary

| Priority | Subsystem | Go Package | Spec Reference | Key Contracts |
| --- | --- | --- | --- | --- |
| 4.1 | SOCKS proxy | `socks/` | Behavioral §15, Quirks §10 | CONNECT only, IP access check |
| 4.2 | IP access rules | `ipaccess/` | Behavioral §16 | CIDR allow/deny evaluation |
| 4.3 | Carrier (WebSocket) | `carrier/` | Behavioral §17 | SSH/bastion forwarding |
| 4.4 | Access commands | `cmd/cloudflared/access/` | Behavioral §18, CLI §5 | Token caching, login flow |
| 4.5 | Tail command | `cmd/cloudflared/tail/` | CLI §6, API §3 | Management log streaming |
| 4.6 | Service install (Linux) | `cmd/cloudflared/` | Behavioral §8 | systemd unit only (defer macOS/Windows) |
| 4.7 | Hello server | `hello/` | Organization §7 | Built-in origin for testing |
| 4.8 | Tracing | `tracing/` | Components §2 | OpenTelemetry propagation |

## Implementation Guidance Boundaries

Use the following documents instead of generic crate-substitution rules:

- `STATUS.md` for accepted slice order and current scaffold intent
- `docs/compatibility-scope.md` for compatibility boundaries
- `docs/go-rust-semantic-mapping.md` for concurrency and lifecycle doctrine
- `docs/dependency-policy.md` for dependency admission
- `docs/allocator-runtime-baseline.md` for allocator and runtime baseline
- `docs/adr/0001-hybrid-concurrency-model.md` for the accepted concurrency ADR

Do not treat Go constructs as automatic Rust crate choices. Choose Rust
primitives and libraries only after checking whether the owning slice is active
and whether the behavior requires them.

## Verification Checklist Per Subsystem

Before marking a subsystem as "ported", verify all items against
`baseline-2026.2.0/design-audit/`:

- [ ] All behavioral invariants from `REPO_BEHAVIORAL_SPEC.md` §30 are tested
- [ ] Wire formats produce identical bytes to Go 2026.2.0 (if applicable)
- [ ] CLI flags produce identical `--help` output
- [ ] Config parsing accepts the same YAML and rejects the same invalid YAML
- [ ] Prometheus metrics use the same names and label sets
- [ ] Error messages preserve the same user-facing text (for scripted consumers)
- [ ] Graceful shutdown completes within the same grace period semantics
- [ ] Quirks from `REPO_QUIRKS_AND_COMPATIBILITY.md` are handled or documented as intentional breaks
- [ ] Component tests from `baseline-2026.2.0/old-impl/component-tests/` pass against the Rust binary
- [ ] Binary runs correctly on `x86_64-unknown-linux-gnu`

## Current Manifest Rule

Do not use this skill to pre-seed `Cargo.toml` with a future dependency graph.

The current repository rule is:

- keep manifests sparse
- add dependencies only for the active owning slice
- keep `mimalloc` in the runnable binary
- defer async, protocol, logging, and config-parser dependencies until code for
   those slices starts landing
