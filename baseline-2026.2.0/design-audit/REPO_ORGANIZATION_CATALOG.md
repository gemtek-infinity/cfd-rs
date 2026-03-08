# Cloudflared Organization Catalog

This document provides an exhaustive first-party repository organization map for rewrite and onboarding work.

Current layout note:

- The current rewrite branch root is intentionally minimal: `AGENTS.md`, `SKILLS.md`, `LICENSE`, `old-impl/`, and `setpoint-docs-2026.2.0/`.
- The historical Go repository layout described below is now rooted at `old-impl/`.

## 1. Top-Level Files

| Path | Role |
| --- | --- |
| `AGENTS.md` | repo-specific rewrite rules and production constraints |
| `SKILLS.md` | rewrite workflow and subsystem porting order |
| `LICENSE` | repository license |
| `old-impl/` | complete Go 2026.2.0 reference implementation |
| `setpoint-docs-2026.2.0/` | extracted behavioral and contract documentation set |

## 2. Top-Level Directories

All first-party top-level directories from the Go reference implementation,
which now lives under `old-impl/`:

| Directory | Role | Classification |
| --- | --- | --- |
| `carrier` | Access forwarding helpers over websocket-like tunnels | support/runtime |
| `cfapi` | Cloudflare REST API clients, filters, and models | contract-critical |
| `cfio` | I/O helpers and copy utilities | support/runtime |
| `client` | connector/client config and feature selection support | runtime |
| `cmd` | executable command trees and OS service entrypoints | runtime-critical |
| `component-tests` | Python end-to-end behavior tests | test-only but contract-relevant |
| `config` | config schema, search, and load | contract-critical |
| `connection` | edge transport and connection abstractions | runtime-critical |
| `credentials` | origin cert and tunnel credential handling | contract-critical |
| `datagramsession` | UDP/ICMP session multiplexing | runtime-critical |
| `diagnostic` | diagnostics collection and serving | support/operational |
| `edgediscovery` | edge resolution and protocol discovery | runtime-critical |
| `features` | feature flag selection and deprecation filtering | runtime-critical |
| `fips` | FIPS-mode compile-time behavior helpers | build/runtime |
| `flow` | active flow limiting | runtime-critical |
| `hello` | built-in hello-world service | support/runtime |
| `ingress` | routing rules, origin modeling, middleware | runtime-critical |
| `internal` | minimal/reserved internal area | support |
| `ipaccess` | IP-based access filtering | runtime-critical when configured |
| `logger` | logging setup and formatting | support/runtime |
| `management` | management service, tokens, events, sessions | contract-critical |
| `metrics` | metrics and readiness server | contract-critical |
| `mocks` | generated mocks | test-only |
| `orchestration` | config hot reload and origin proxy ownership | runtime-critical |
| `overwatch` | service lifecycle helper layer | support/runtime |
| `packet` | packet parsing/serialization | runtime-critical |
| `proxy` | forwarding bridge to origin | runtime-critical |
| `quic` | QUIC utilities and datagram support | runtime-critical |
| `release` | release artifacts/resources | tooling |
| `retry` | retry/backoff helpers | runtime-critical |
| `signal` | signal handling helpers | runtime-critical |
| `socks` | SOCKS proxy support | support/runtime |
| `sshgen` | SSH cert generation helpers | support |
| `stream` | stream helpers | support/runtime |
| `supervisor` | tunnel lifecycle and reconnect logic | runtime-critical |
| `tlsconfig` | TLS configuration building | runtime-critical |
| `token` | Access token handling | support/contract-relevant |
| `tracing` | OpenTelemetry and trace propagation | support/runtime |
| `tunnelrpc` | RPC schemas and adapters | protocol-critical |
| `tunnelstate` | active connection state tracking | runtime-critical |
| `validation` | input and token validation helpers | support/contract-relevant |
| `vendor` | vendored third-party code | external code |
| `watcher` | config file watch support | runtime-critical when used |
| `websocket` | websocket helpers | support/runtime |

Reference top-level files that also live under `old-impl/` and remain relevant
for rewrite work:

- `README.md`
- `CHANGES.md`
- `Makefile`
- `go.mod`
- `Dockerfile*`
- `check-fips.sh`
- `postinst.sh`, `postrm.sh`
- `github_release.py`, `github_message.py`, `release_pkgs.py`
- `cloudflared.wxs`, `wix.json`

## 3. Command Tree Organization

`cmd/cloudflared` is the executable root.

Important subareas:

- root app and command registration
- OS-specific service files
- `tunnel` subtree
- `access` subtree
- `tail` command
- `management` command
- `cliutil` shared helpers
- `flags` constants

## 4. Runtime Core Organization

The runtime core is centered on these directories:

- `supervisor`
- `connection`
- `orchestration`
- `ingress`
- `proxy`
- `datagramsession`
- `metrics`
- `management`

This is the part of the repo most likely to be reimplemented in a rewrite.

## 5. Contract-Critical Organization

The directories most tightly tied to external compatibility are:

- `cmd/cloudflared`
- `config`
- `credentials`
- `cfapi`
- `metrics`
- `management`
- `tunnelrpc`

## 6. Test Organization

Tests are distributed rather than centralized.

Patterns:

- package-local Go unit tests in most runtime directories
- dedicated Python component tests in `component-tests`
- generated mocks in `mocks`

## 7. Package Detail Expansions

Packages listed in §2 with brief roles are expanded here for rewrite and onboarding completeness.

### 7.1 `ipaccess`

IP-based access filtering for SOCKS proxy and ingress flows.

Key types:

- `Policy`: holds `defaultAllow bool` and `rules []Rule`. Evaluation is first-match-wins.
- `Rule`: holds `ipNet *net.IPNet`, `ports []int` (sorted), `allow bool`.

Port matching uses binary search (`sort.SearchInts`). Empty port list means all ports match.

Used by: `socks.StandardRequestHandler`, ingress IP rules config.

### 7.2 `token`

Access JWT token handling with file-based storage, locking, and refresh.

Key functions:

- `FetchTokenWithRedirectRecovery()`: top entry point.
- `getTokenIfExists()`: check disk, return if valid, delete if expired.
- `exchangeOrgTokenForAppToken()`: SSO token exchange.
- `RunTransfer()`: full browser-based auth.

Lock mechanics: file lock at `tokenPath + ".lock"`, 7-retry exponential backoff, SIGINT/SIGTERM cleanup handlers, stale lock force-delete after max retries.

Token expiry: `jwtPayload` struct with `Exp` field checked against `time.Now().Unix()`. Expired tokens are deleted from disk.

Storage paths: derived from `(appDomain, appAUD, "token")` for app tokens, `authDomain` for org tokens.

### 7.3 `carrier`

Access TCP-over-WebSocket forwarding and bastion destination resolution.

Key functions:

- `StartForwarder()`: binds a local TCP listener and tunnels each accepted connection through a WebSocket.
- `ResolveBastionDest()`: reads `Cf-Access-Jump-Destination` header from the request to determine the bastion target.
- `SetBastionDest()`: writes the bastion destination header onto outgoing requests.

Used by: `access tcp`, bastion mode ingress.

### 7.4 `socks`

SOCKS5 proxy implementation.

Key types:

- `StandardRequestHandler`: holds `dialer Dialer` and `accessPolicy *ipaccess.Policy`.

Command support: CONNECT only. BIND and ASSOCIATE return `commandNotSupported`.

FQDN handling: destinations provided as FQDNs are resolved to IP before the access policy check. This means DNS resolution failure looks like a policy denial (`ruleFailure`).

### 7.5 `edgediscovery`

Edge address discovery via DNS.

Key functions:

- `ResolveEdge()`: SRV lookup (`_v2-origintunneld._tcp.argotunnel.com`) with DoT fallback (`cloudflare-dns.com:853`).
- `StaticEdge()`: hardcoded addresses for testing.
- `Edge.GetAddr(connIndex)`: prefers address previously used by the same connection index.

Address pool: `allregions.Regions` manages per-region pools with used/unused tracking and connectivity error marking.

### 7.6 `features`

Feature flag selection and deprecation filtering.

Key functions:

- `DefaultFeatures()`: returns default feature list.
- `dedupAndRemoveFeatures()`: silently strips deprecated features.
- `DatagramVersion()`: selects v2/v3 based on CLI flags, remote percentage, or default.

Default features: `allow_remote_config`, `serialized_headers`, `support_datagram_v2`, `support_quic_eof`, `management_logs`.

### 7.7 `retry`

Backoff primitives used by supervisor and connection retry logic.

Key type: `BackoffHandler` with configurable max retries, min/max delay, and jitter. Used by `supervisor.protocolFallback`.

### 7.8 `signal`

Graceful shutdown signal handling.

Coordinates SIGINT/SIGTERM trapping. Used across supervisor, token, and daemon lifecycle code.

## 8. Rewrite-Oriented Organization Summary

If rewriting from scratch, the repo can be mentally partitioned into:

1. external contract layer: CLI, config, API, metrics, management, RPC schema
2. runtime engine layer: supervisor, connection, orchestration, ingress, proxy
3. support/integration layer: edgediscovery, credentials, cfapi, diagnostics, tracing, service installers
4. enforcement layer: ipaccess, token, socks, carrier, flow, signal

The enforcement layer is often overlooked but contains contract-critical behavior around authentication, authorization, and connection management.
