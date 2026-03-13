# Stage 3.1: Scope Triage

## Purpose

This document records the scope pruning and divergence triage performed before
the Stage 3 refactor.

Every gap and divergence identified during the Stage 1 audit (150 rows across
three parity ledgers) has been explicitly classified so the refactor operates
on a clean, bounded scope.

This document names what is excluded and why.

The confirmed refactor scope at the end of this document names exactly what
will be moved into the target 5-crate map.

## Lane Definition

The active lane for production alpha is:

- Linux only
- target triple: `x86_64-unknown-linux-gnu`
- shipped GNU artifacts: `x86-64-v2`, `x86-64-v4`
- systemd is the governing service expectation for the alpha contract
  (ADR-0005)
- bare-metal-first deployment stance (ADR-0005)
- FIPS belongs in the lane as a governance boundary, not as an implementation
  claim (ADR-0004)

Production alpha means: feature-complete 1:1 in behavior and surface to
frozen `2026.2.0` on the declared Linux lane, performance proven, not every
edge case necessarily covered.

Source: `REWRITE_CHARTER.md`, `docs/compatibility-scope.md`,
`docs/adr/0004-fips-in-alpha-definition.md`,
`docs/adr/0005-deployment-contract.md`.

## Classification Scheme

Every audited gap or divergence is classified into exactly one of four
categories:

| Classification | Meaning |
| --- | --- |
| lane-required | Must be implemented and placed in the 5-crate map for production alpha |
| deferred | Lane-relevant but not blocking production alpha; named for post-alpha work |
| compatibility-only | Present in frozen baseline but already deprecated or removed; requires an error stub, not a working implementation |
| non-lane | Out of scope for this lane entirely; not required for production alpha on the declared Linux lane |

Lane-required is the default. Items not listed below are lane-required.

## Classification Summary

| Domain | Lane-required | Deferred | Compatibility-only | Non-lane | Total |
| --- | --- | --- | --- | --- | --- |
| CLI | 23 | 6 | 3 | 0 | 32 |
| CDC | 40 | 4 | 0 | 0 | 44 |
| HIS | 45 | 27 | 0 | 2 | 74 |
| **Total** | **108** | **37** | **3** | **2** | **150** |

---

## Non-Lane Items (Excluded From Refactor)

These items are not binary behavior and are outside the scope of the Rust
rewrite entirely.

| ID | Feature | Reason |
| --- | --- | --- |
| HIS-056 | `postinst.sh` behavior | Packaging script; not Rust binary behavior. Packaging assets are a deployment concern, not a crate-level parity target. |
| HIS-057 | `postrm.sh` behavior | Packaging script; not Rust binary behavior. Same rationale as HIS-056. |

---

## Compatibility-Only Items (Error Stubs Required)

These features are already deprecated or removed in the frozen baseline. The
baseline behavior is an error message, not a working feature. The parity
target is the exact error stub behavior (exit code, stderr text, error URL).

| ID | Feature | Baseline behavior | Required parity |
| --- | --- | --- | --- |
| CLI-025 | `proxy-dns` removal | Top-level `proxy-dns` prints deprecation error with URL to DNS-over-HTTPS alternative; `tunnel proxy-dns` shows removed-since-2026.2.0 error | Exact error text, stderr placement, exit code |
| CLI-026 | `db-connect` removal | `tunnel db-connect` shows removed-command error via `cliutil.RemovedCommand`; exits 255 | Exact error text, exit code (255) |
| CLI-027 | classic tunnel deprecation | Classic tunnel invocation paths produce "Classic tunnels have been deprecated, please use Named Tunnels" error | Exact error text, exit code |

---

## Deferred Items (Lane-Relevant, Post-Alpha)

These items are lane-relevant and will be needed for full production parity,
but are not blocking production alpha. Each deferral has an explicit reason.

### CLI Deferred (6 items)

| ID | Feature | Priority | Deferral reason |
| --- | --- | --- | --- |
| CLI-006 | `update` command | high | Requires external update infrastructure (`update.argotunnel.com`) and auto-update timer. Depends on HIS-046 through HIS-049 which are also deferred. |
| CLI-016 | `tunnel info` | medium | Requires active connector listing via API. Lower operational priority than core tunnel lifecycle commands. |
| CLI-017 | `tunnel ready` | medium | Requires local metrics endpoint (HIS-024, HIS-025) which is lane-required but not yet implemented. CLI entry for this command is simple once the endpoint exists. |
| CLI-018 | `tunnel diag` | medium | Diagnostics collection bundle (HIS-032 through HIS-040) is deferred as a cohesive subsystem. |
| CLI-021 | `tunnel ingress` (hidden) | medium | Hidden debug subcommand for inspecting ingress rules. Low operational priority. |
| CLI-024 | `management` subtree (hidden) | medium | Entirely hidden admin tooling. The management token flow is accessible through both this and the `tail token` path. |

### CDC Deferred (4 items)

| ID | Feature | Priority | Deferral reason |
| --- | --- | --- | --- |
| CDC-027 | management CORS | medium | Enables Cloudflare dashboard browser access to management routes. Not required for CLI-based `tail` and management workflows which use direct WebSocket. |
| CDC-028 | diagnostics conditional exposure | medium | `/metrics` and `/debug/pprof` conditional registration on management service. Debug tooling, not core operational. |
| CDC-032 | `/quicktunnel` endpoint response | low | Quick tunnel URL exposure is a convenience feature. Quick tunnel itself requires external API support. |
| CDC-039 | hostname routing API | medium | Legacy DNS routing via zones. The primary routing path uses `tunnel route ip` and `tunnel route dns` which are lane-required (CDC-036). |

### HIS Deferred (27 items)

#### SysV init (2 items)

| ID | Feature | Priority | Deferral reason |
| --- | --- | --- | --- |
| HIS-016 | SysV init script generation | high | ADR-0005 states: "systemd is the governing service expectation for the alpha contract" and "broad multi-init support is not part of the governing alpha contract." |
| HIS-023 | SysV init script exact content | high | Same as HIS-016. SysV is a fallback for non-systemd hosts, not the alpha contract. |

#### Diagnostics subsystem (9 items)

| ID | Feature | Priority | Deferral reason |
| --- | --- | --- | --- |
| HIS-032 | `tunnel diag` CLI command | high | Diagnostics collection is a cohesive subsystem. Deferring as a unit is cleaner than partial implementation. |
| HIS-033 | system information collector | high | Part of diagnostics subsystem. |
| HIS-034 | tunnel state collector | high | Part of diagnostics subsystem. |
| HIS-035 | CLI configuration collector | medium | Part of diagnostics subsystem. |
| HIS-036 | host log collector | medium | Part of diagnostics subsystem. |
| HIS-037 | network traceroute collector | medium | Part of diagnostics subsystem. |
| HIS-038 | diagnostic instance discovery | medium | Part of diagnostics subsystem. Port scanning for running instances. |
| HIS-039 | `/diag/system` HTTP endpoint | high | Part of diagnostics subsystem. Served on local metrics server. |
| HIS-040 | `/diag/tunnel` HTTP endpoint | high | Part of diagnostics subsystem. Served on local metrics server. |

#### Updater subsystem (4 items)

| ID | Feature | Priority | Deferral reason |
| --- | --- | --- | --- |
| HIS-046 | `update` CLI command | high | Requires external update infrastructure (`update.argotunnel.com`). Self-update mechanism is operationally useful but not lane-blocking. |
| HIS-047 | auto-update timer | high | Depends on updater implementation. The systemd update timer template (HIS-014) generates the timer unit, but the actual update binary behavior is deferred. |
| HIS-048 | update exit codes | medium | Depends on updater implementation. Exit code 11 (success/restart) protocol integrates with systemd. |
| HIS-049 | package manager detection | medium | Depends on updater implementation. `.installedFromPackageManager` marker file. |

#### Local HTTP convenience endpoints (3 items)

| ID | Feature | Priority | Deferral reason |
| --- | --- | --- | --- |
| HIS-028 | `/quicktunnel` endpoint | medium | Quick tunnel URL exposure. Convenience feature, not operational monitoring. |
| HIS-029 | `/config` endpoint | medium | Remote config visibility via metrics server. Useful for debugging remotely-managed tunnels but not operational-critical. |
| HIS-030 | `/debug/pprof/*` endpoints | low | Runtime profiling endpoints. Debugging aid, not operational. Auth explicitly disabled in baseline (`trace.AuthRequest` returns true). |

#### Environment and privilege (2 items)

| ID | Feature | Priority | Deferral reason |
| --- | --- | --- | --- |
| HIS-050 | UID detection for diagnostics | medium | Only gates diagnostic log collection path (journalctl vs file). Deferred because diagnostics subsystem is deferred. |
| HIS-051 | terminal detection | medium | Gates auto-update behavior. Deferred because updater is deferred. |

#### ICMP proxy (3 items)

| ID | Feature | Priority | Deferral reason |
| --- | --- | --- | --- |
| HIS-069 | ICMP proxy raw socket | high | Specialized proxy feature requiring CAP_NET_RAW or ping_group membership. Complete subsystem that can be added independently. |
| HIS-070 | ping group range check | high | Linux-specific privilege gate for ICMP. Part of ICMP subsystem. |
| HIS-071 | ICMP source IP flags | medium | `--icmpv4-src` and `--icmpv6-src` configuration. Part of ICMP subsystem. |

#### Miscellaneous (4 items)

| ID | Feature | Priority | Deferral reason |
| --- | --- | --- | --- |
| HIS-061 | `--pidfile` flag | medium | Optional systemd integration. PID written after tunnel connects, not on startup. Low urgency. |
| HIS-072 | `hello_world` ingress listener | medium | Built-in test server with TLS listener and routes. Convenience and testing feature, not operational. Rust parses `hello_world` config but has no listener. |
| HIS-073 | gracenet socket inheritance | medium | Zero-downtime process restart via FD passing. Optimization for update cycle, depends on updater. |
| HIS-074 | process self-restart on update | medium | Fork-and-replace after update. Depends on updater implementation and gracenet (HIS-073). |

---

## Confirmed Refactor Scope

The refactor will operate on exactly 108 lane-required surfaces plus 3
compatibility-only error stubs. This is the set that will be moved into the
5-crate map.

### Crate Ownership Map

| Target crate | Lane-required surfaces | Ownership |
| --- | --- | --- |
| `cfdrs-bin` | Process entrypoint, runtime composition, lifecycle orchestration, supervision composition. Not a parity domain — composes CLI, CDC, and HIS. | Owns startup, shutdown, runtime bootstrap, and the top-level state machine. |
| `cfdrs-cli` | CLI-001 through CLI-005, CLI-007 through CLI-015, CLI-019, CLI-020, CLI-022, CLI-023, CLI-025 through CLI-030, CLI-032. 26 items (23 lane-required + 3 compatibility-only error stubs). | Owns the command tree, help text, flag parsing, env bindings, exit codes, formatting, and user-visible dispatch. |
| `cfdrs-cdc` | CDC-001 through CDC-026, CDC-029 through CDC-031, CDC-033 through CDC-038, CDC-040 through CDC-044. 40 items. | Owns registration RPC, stream contracts, management service, log streaming, metrics and readiness contracts, Cloudflare REST API, datagram/UDP sessions, and credential encoding. |
| `cfdrs-his` | HIS-001 through HIS-015, HIS-017 through HIS-022, HIS-024 through HIS-027, HIS-031, HIS-041 through HIS-045, HIS-052 through HIS-055, HIS-058 through HIS-060, HIS-062 through HIS-068. 45 items. | Owns config discovery, credentials lookup, service installation, systemd integration, local HTTP endpoints, watcher and reload, signal handling, logging, and deployment evidence. |
| `cfdrs-shared` | Narrowly admitted cross-domain primitives: error plumbing, config types used by both CDC and HIS, credential types referenced by both CLI dispatch and CDC registration. | Must not become a dump crate. Admission justified by audit evidence showing limited cross-domain overlap. |

### Deferred Items by Future Target Crate

When deferred items are eventually implemented, they will land in these crates:

| Target crate | Deferred items |
| --- | --- |
| `cfdrs-cli` | CLI-006, CLI-016, CLI-017, CLI-018, CLI-021, CLI-024 |
| `cfdrs-cdc` | CDC-027, CDC-028, CDC-032, CDC-039 |
| `cfdrs-his` | HIS-016, HIS-023, HIS-028 through HIS-030, HIS-032 through HIS-040, HIS-046 through HIS-051, HIS-061, HIS-069 through HIS-074 |

---

## Exit Condition Checklist

Stage 3.1 exit conditions from `FINAL_PLAN.md`:

- every gap and divergence from the audit has an explicit classification: **yes**
  (150 rows classified; 108 lane-required, 37 deferred, 3 compatibility-only,
  2 non-lane)
- the refactor scope is bounded and justified from audit evidence: **yes**
  (see Confirmed Refactor Scope above)
- non-lane and deferred behaviors are named, not silently ignored: **yes**
  (see Non-Lane, Compatibility-Only, and Deferred sections above)
- the refactor can proceed on a known surface without scope creep: **yes**
  (the 111 surfaces in scope are enumerated by ID)
