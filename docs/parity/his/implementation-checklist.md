# HIS Implementation Checklist

## Purpose

This document is the live parity ledger for interactions between cloudflared
and the local host and host services.

This includes:

- filesystem effects
- config discovery and default creation behavior
- credentials and local file lookup behavior where host-owned
- service installation and supervision behavior
- diagnostics collection
- watcher and reload behavior
- local endpoint exposure
- environment and privilege assumptions
- deployment-layout and host-path expectations

This document does not claim parity from Rust code shape alone.

It records:

- the frozen host-facing behavior or contract that must be matched
- the current Rust owner, if any
- the current Rust implementation state
- the current evidence maturity
- whether a gap or divergence is open
- the tests required before parity can be claimed

## Checklist Field Vocabulary

The table uses three different status fields.

### Rust status now

Use only these values:

- not audited
- audited, absent
- audited, partial
- audited, parity-backed
- audited, intentional divergence
- blocked

### Parity evidence status

Preferred values:

- not present
- minimal
- weak
- partial
- parity-backed
- first-slice evidence exists
- partial local tests only

If a new value is needed later, add it deliberately and keep it short.

### Divergence status

Preferred values:

- none recorded
- open gap
- intentional divergence
- unknown
- blocked

## Seeded Checklist

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-001 | config discovery search order | config discovery contract and first-slice baseline fixtures | default search directories and filename order must match the frozen baseline | cloudflared-config discovery | audited, partial | first-slice evidence exists | open gap | parity compare tests, discovery fixture tests, final-phase checklist carryover | high | one of the strongest existing host-facing areas, but broader final-phase closure is still open |
| HIS-002 | config auto-create behavior | config discovery contract and first-slice baseline fixtures | missing-config behavior, default file creation, and logDirectory semantics must match the frozen baseline where applicable | cloudflared-config discovery | audited, partial | first-slice evidence exists | open gap | filesystem-effect tests, config creation tests | high | partly grounded by the accepted first-slice harness, but not broad HIS closure |
| HIS-003 | config file loading and normalization | config contract and first-slice baseline fixtures | YAML loading, warnings, no-ingress defaulting, and normalization behavior must match the frozen baseline within the admitted surface | cloudflared-config | audited, partial | first-slice evidence exists | open gap | config golden tests, parity compare tests | medium | historical first-slice area; keep the admitted scope explicit and do not over-claim broader HIS parity |
| HIS-004 | credentials file lookup and parsing | credential surface and local file lookup behavior | tunnel credentials and origin-cert lookup behavior on the host must match the frozen baseline | cloudflared-config credentials | audited, partial | first-slice evidence exists | open gap | credential parity tests, path lookup tests, file error-behavior tests | high | ownership may later split between HIS and CDC, but host lookup behavior is HIS-facing |
| HIS-005 | service install and uninstall on Linux | linux service command behavior | Linux service command behavior, generated assets, enablement, and uninstall side effects must match the frozen baseline on the declared lane | none in current Rust | audited, absent | not present | open gap | host-integration tests, command tests, template and side-effect tests | critical | major gap on the declared Linux lane |
| HIS-006 | systemd expectation and detection | linux service behavior and deployment contract | systemd detection and service-management behavior must match the frozen baseline where lane-relevant | current deployment evidence only | audited, partial | weak | open gap | host-detection tests, service tests, supervision-path tests | high | current Rust reports systemd detection evidence, but does not provide service-install parity |
| HIS-007 | filesystem layout expectations | deployment contract and service asset paths | executable, config, credential, log, and runtime state path expectations must match the frozen baseline where operator-visible | current deployment evidence only | audited, partial | weak | open gap | path tests, side-effect tests, deployment-layout tests | high | current repo evidence is contract-level and honesty-oriented, not host-behavior parity |
| HIS-008 | diagnostics local collection | diagnostics contract and collectors | local diagnostics collection and emitted artifact coverage must match the frozen baseline where lane-relevant | none in current Rust | audited, absent | not present | open gap | collector tests, output-shape tests, endpoint-driven diagnostics tests | critical | includes system information, file-descriptor state, tunnel state, memory details, and config-related artifacts where baseline exposes them |
| HIS-009 | local metrics endpoint exposure | local metrics server contract | local endpoint set, bind behavior, and endpoint availability conditions must match the frozen baseline | none in current Rust | audited, absent | not present | open gap | HTTP endpoint tests, bind-behavior tests, endpoint inventory tests | high | keep local host exposure separate from CDC-visible remote contracts; baseline includes `/metrics`, `/healthcheck`, `/ready`, `/quicktunnel`, `/config`, and `/debug/` conditions |
| HIS-010 | local readiness endpoint | local process HTTP API readiness contract | `/ready` local behavior, JSON response shape, and HTTP 200 or 503 semantics must match the frozen baseline | current runtime readiness only | audited, absent | not present | open gap | local HTTP readiness tests, response-shape tests, active-connection semantic tests | high | internal readiness tracking is not enough to claim the local endpoint contract |
| HIS-011 | watcher and reload behavior | config manager and watcher behavior | config watch, file-change handling, and reload semantics must match the frozen baseline | none in current Rust | audited, absent | not present | open gap | watcher tests, reload integration tests, failure-mode tests | critical | current Rust explicitly declares no config reload support |
| HIS-012 | privilege and environment assumptions | host environment behavior and deployment contract | UID, environment, privilege, and related host assumptions exposed to runtime and diagnostics paths must match the frozen baseline where lane-relevant | current deployment and summary evidence only | audited, partial | weak | open gap | environment-behavior tests, privilege-sensitive path tests | medium | keep this separate from generic deployment notes so host assumptions stay auditable |
| HIS-013 | local management exposure | local host-facing management service behavior | local route exposure, bind expectations, and host details surface must match the frozen baseline where present | none in current Rust | audited, absent | not present | open gap | local endpoint tests, route exposure tests, host-detail response tests | high | overlaps CDC at the external contract edge, but host-side exposure and local bind behavior are HIS-owned |
| HIS-014 | updater and host integration | updater and service host effects | updater-related host behavior, timers, restart semantics, and service integration side effects must match the frozen baseline where lane-relevant | none in current Rust | audited, absent | not present | open gap | command tests, filesystem tests, timer and restart-behavior tests | medium | audit lane relevance explicitly before widening beyond the declared Linux production-alpha surface |
| HIS-015 | deployment evidence scope versus host parity | deployment contract, deployment notes, and current runtime deployment evidence | current deployment evidence must remain honest about host assumptions and must not be mistaken for full host-behavior parity | current runtime deployment code | audited, intentional divergence | partial local tests only | intentional divergence | divergence note, evidence-scope tests, host-parity gap tests | medium | keep the current bounded proof surface explicit while broader HIS parity remains unimplemented |

## Immediate Work Queue

1. inventory Linux service install and uninstall behavior from the frozen baseline, including generated assets and side effects
2. inventory local metrics, readiness, config, quicktunnel, debug, and diagnostics endpoints from the frozen baseline
3. inventory diagnostics collector surfaces and output shapes
4. inventory watcher and reload behavior, including failure and recovery semantics
5. classify which host behaviors are required for the declared Linux lane versus compatibility-only or later-surface behavior
6. keep existing deployment-assumption evidence explicitly separate from actual host-behavior parity
7. create feature-group documents for service behavior, filesystem and layout, diagnostics, and reload behavior once the next audit pass lands
