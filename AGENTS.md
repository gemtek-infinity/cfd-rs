# Cloudflared — Go → Rust Rewrite

Cloudflare's command-line tunneling daemon. This branch (`fork/rust-rewrite`)
tracks the production-grade rewrite from Go to Rust.

- **Reference version**: `2026.2.0` (Go implementation in `baseline-2026.2.0/old-impl/`)
- **Primary target**: `x86_64-unknown-linux-gnu`
- **Behavioral spec**: `baseline-2026.2.0/design-audit/` (6,090 lines, 175+ sections)
- **Rust workspace version rule**: `<go-release>-alpha.YYYYmm`

The Go source in `baseline-2026.2.0/old-impl/` is the **source of truth** for behavioral parity.
The `baseline-2026.2.0/design-audit/` folder contains the exhaustive specification
extracted from that source — every behavior, contract, quirk, and wire format
needed to produce a byte-for-byte compatible Rust replacement.

The Rust workspace version must track the Go compatibility baseline rather than
an independent Rust-only version line. For the current baseline, use
`2026.2.0-alpha.202603`.

Both `baseline-2026.2.0/old-impl/` and `baseline-2026.2.0/design-audit/` are
frozen inputs to the rewrite program. Do not modify files under either directory
during normal rewrite work. If a conflict is found, fix the Rust workspace or
the top-level governance docs, not the frozen Go reference or the extracted
setpoint docs.

## Repository Layout

```text
cloudflared/
├── AGENTS.md                      # This file — rewrite rules and context
├── SKILLS.md                      # Rewrite workflow skill with porting order
├── LICENSE                        # Apache 2.0
└── baseline-2026.2.0/             # Frozen rewrite inputs
    ├── design-audit/              # Behavioral spec extracted from Go 2026.2.0
    │   ├── REPO_REFERENCE.md      # Master index (start here)
    │   ├── REPO_BEHAVIORAL_SPEC.md
    │   ├── REPO_API_CONTRACTS.md
    │   ├── REPO_CLI_INVENTORY.md
    │   ├── REPO_CONFIG_CONTRACT.md
    │   ├── REPO_ARCHITECTURE_DEEP_DIVE.md
    │   ├── REPO_ORGANIZATION_CATALOG.md
    │   ├── REPO_COMPONENTS_AND_DEPENDENCIES.md
    │   ├── REPO_QUIRKS_AND_COMPATIBILITY.md
    │   ├── REPO_DIAGRAMS.md       # 18 Mermaid diagrams
    │   ├── REPO_SOURCE_INDEX.md
    │   └── REPO_AUDIT_CHECKLIST.md
    └── old-impl/                  # Go 2026.2.0 reference (read-only)
        ├── cmd/cloudflared/            # CLI entry points
        ├── connection/                 # QUIC + HTTP/2 transports
        ├── supervisor/                 # HA connection manager
        ├── orchestration/              # Config hot-reload
        ├── ingress/                    # Routing rules
        ├── proxy/                      # Request proxying
        ├── tunnelrpc/                  # Cap'n Proto schemas
        ├── management/                 # Management HTTP/WS
        ├── metrics/                    # Prometheus + readiness
        ├── go.mod                      # Go dependencies
        ├── Makefile                    # Build/test/lint
        └── ...                         # 60+ more packages
```

## Documentation Map

All reference documents live in `baseline-2026.2.0/design-audit/`. Start with
`REPO_REFERENCE.md`.

| Document | Purpose | Sections |
| --- | --- | --- |
| `REPO_REFERENCE.md` | **Master index** — overview, mental model, cross-references | 20 |
| `REPO_BEHAVIORAL_SPEC.md` | Every observable behavior (startup, ingress, proxy, shutdown, retry, datagram encoding, protocol fallback) | 30 |
| `REPO_API_CONTRACTS.md` | Wire formats, RPC schemas (Cap'n Proto), HTTP endpoints, management WebSocket, Prometheus metrics, DNS edge discovery | 22 |
| `REPO_CLI_INVENTORY.md` | Every command, subcommand, flag, env var, default, hidden/deprecated path | 9 |
| `REPO_CONFIG_CONTRACT.md` | YAML schema, config discovery, ingress rules, originRequest, credential files, duration encoding | 13 |
| `REPO_ARCHITECTURE_DEEP_DIVE.md` | Layered arch, supervisor, orchestrator, proxy dispatch, datagram sessions, start-before-stop pattern | 19 |
| `REPO_ORGANIZATION_CATALOG.md` | Package-to-responsibility map for all 40+ directories | 8 |
| `REPO_COMPONENTS_AND_DEPENDENCIES.md` | External libraries, services, protocols, forked deps, wire-critical constants, rewrite crate mapping | 9 |
| `REPO_QUIRKS_AND_COMPATIBILITY.md` | Non-obvious behaviors, platform quirks, deprecations, compatibility traps | 16 |
| `REPO_DIAGRAMS.md` | 18 Mermaid diagrams (startup, protocol FSM, request routing, datagram encoding, orchestrator, etc.) | 18 |
| `REPO_SOURCE_INDEX.md` | Topic → source file → test file quick-lookup | 5 |
| `REPO_AUDIT_CHECKLIST.md` | Gate checklist for verifying completeness of each subsystem | 6 |

## Essential Commands

### Go Reference Build (for parity testing)

```bash
# Build the Go reference binary
cd baseline-2026.2.0/old-impl && make cloudflared
cd baseline-2026.2.0/old-impl && TARGET_OS=linux TARGET_ARCH=amd64 make cloudflared

# Run Go tests for behavioral reference
cd baseline-2026.2.0/old-impl && make test
cd baseline-2026.2.0/old-impl && go test -run TestName ./pkg
cd baseline-2026.2.0/old-impl && go test -race ./...

# Component tests (Python integration — run against either binary)
cd baseline-2026.2.0/old-impl/component-tests && python -m pytest test_file.py::test_name
```

### Rust Build (primary target)

```bash
# Build for primary target
cargo build --release --target x86_64-unknown-linux-gnu

# Run tests
cargo test
cargo test -- --test-threads=1  # for tests requiring serial execution

# Lint
cargo clippy -- -D warnings
cargo fmt --check

# Wire-format parity check (compare against Go binary output)
# Build both, then run component tests against each
```

## Rewrite Ground Rules

### Target Platform

The primary build target is `x86_64-unknown-linux-gnu`. Platform-specific
behavior for macOS and Windows exists in the Go source (service installation,
UDP workarounds) but is **deferred** — implement Linux first, then extend.

Relevant platform quirks to defer:

- macOS `OOBCapablePacketConn` UDP workaround (Quirks §14)
- Windows Service Manager integration (Behavioral §8)
- macOS launchd plist service installation (Behavioral §8)

### Behavioral Contracts (MUST preserve)

Every item in `REPO_BEHAVIORAL_SPEC.md` §30 "Behavioral Invariants For Rewrite"
and `REPO_ARCHITECTURE_DEEP_DIVE.md` §10 "Rewrite Preservation Rules" is a
non-negotiable contract. Summary:

- CLI flag names, env var names, config YAML keys, and their defaults
- Wire formats: datagram V2 suffix encoding, V3 prefix encoding, Cap'n Proto RPC
  schemas, protocol magic bytes, QUIC ALPN `argotunnel`, TLS ServerName `cftunnel.com`
- HTTP header contracts: `Cf-Cloudflared-*`, `Cf-Warp-Tag-*`, ResponseMeta serialization
- Metrics endpoint paths and Prometheus metric names
- Management WebSocket event shapes
- Ingress matching semantics (order, catch-all, punycode dual-match)
- Protocol selection and fallback order (QUIC → HTTP/2)
- Graceful shutdown sequence and grace period behavior
- Readiness semantics (at least one connected tunnel = ready)
- Config file search order and credential file format

### Wire-Critical Constants

See `REPO_COMPONENTS_AND_DEPENDENCIES.md` §8 for the exact bytes, magic values,
and protocol constants that must be reproduced identically.

### Architecture (CAN change internals)

The Rust port may restructure internals freely as long as:

1. All externally visible behaviors from the behavioral spec are preserved
2. All wire formats from the API contracts are byte-compatible
3. All CLI and config contracts produce identical user-facing behavior

See `REPO_ARCHITECTURE_DEEP_DIVE.md` §16 "Replaceable Versus Non-Replaceable
Internals" for the detailed breakdown.

### Quirks (MUST replicate unless intentionally fixing)

`REPO_QUIRKS_AND_COMPATIBILITY.md` documents behaviors that look like bugs but
may be relied upon by users. Each quirk must be either:

1. Replicated identically in the Rust port, or
2. Explicitly listed as a breaking change with migration guidance

Key examples: punycode dual-matching, IPv4-mapped-IPv6 normalization in V2
datagrams, SOCKS5 FQDN-before-IP-check ordering, metrics port try-in-order
fallback.

## Boundaries

### Always Do

- Read the relevant `baseline-2026.2.0/design-audit/REPO_*.md` section before implementing a subsystem
- Read the Go source in `baseline-2026.2.0/old-impl/` for the subsystem being ported
- Validate behavioral parity against the Go 2026.2.0 reference
- Preserve all wire formats and protocol constants exactly
- Keep the existing Cargo workspace scaffold minimal and honest
- Follow `docs/dependency-policy.md` before adding dependencies to any crate
- Follow `docs/allocator-runtime-baseline.md` for allocator and runtime admission
- Follow `docs/go-rust-semantic-mapping.md` and
  `docs/adr/0001-hybrid-concurrency-model.md` for concurrency structure
- Keep `mimalloc` configured only in the runnable binary crate
- Handle all errors explicitly with context when a subsystem actually introduces
  typed or ad-hoc error handling
- Use `tracing` only when the owning subsystem slice has started and structured
  logging is part of the implemented behavior
- Write tests that verify contract-level behavior, not just internals
- Target `x86_64-unknown-linux-gnu` first; defer other platforms

### Ask First Before

- Changing any wire format or protocol constant
- Adding/removing CLI flags or config keys
- Modifying Cap'n Proto schemas
- Changing metrics names or label sets
- Altering management endpoint paths or response shapes
- Dropping support for any documented quirk/compatibility behavior
- Adding platform targets beyond `x86_64-unknown-linux-gnu`

### Never Do

- Assume a Go behavior is "wrong" without checking `REPO_QUIRKS_AND_COMPATIBILITY.md`
- Change wire encoding without updating the behavioral spec
- Treat the repository as a blank-slate Rust project
- Invent new top-level subsystem boundaries that are not supported by the
  current repository structure
- Preload speculative dependencies into manifests before the owning slice starts
- Adopt a repo-wide actor framework
- Skip testing for new functionality
- Commit secrets, credentials, or sensitive data
- Ignore platform-specific behaviors documented in the quirks file
- Use `unsafe` without documented justification and safety proof
- Edit files under `baseline-2026.2.0/` during routine rewrite work

## Security Considerations

- Post-quantum encryption support via QUIC (see `REPO_BEHAVIORAL_SPEC.md` §11)
- Credential handling: origin certs, tunnel credentials JSON, access tokens
  (see `REPO_CONFIG_CONTRACT.md` §8)
- TLS configuration: custom CA pools, origin server name verification
- Management endpoint: JWT token-gated access (see `REPO_API_CONTRACTS.md` §3)
- No `unsafe` blocks without explicit safety comments
- Use `rustls` (not OpenSSL) for TLS — pure Rust, auditable

## Skills

See `SKILLS.md` for the rewrite workflow skill that maps Go subsystems to Rust
implementation tasks with verification steps.
