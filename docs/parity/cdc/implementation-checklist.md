# CDC Implementation Checklist

## Purpose

This document is the live parity ledger for interactions between cloudflared
and Cloudflare-managed services, APIs, and contracts.

This includes:

- registration RPC and related registration content
- control-stream lifecycle behavior
- per-stream request and response contracts
- management and log-streaming contracts
- metrics and readiness contracts where externally relevant
- Cloudflare API interactions used by command surfaces

This document does not claim parity from Rust code shape alone.

It records:

- the frozen contract that must be matched
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
| CDC-001 | registration RPC schema | tunnelrpc/proto/tunnelrpc.capnp and design-audit API contracts | registration uses the frozen Cap'n Proto schema, method ordering, and field semantics | cloudflared-proto plus current QUIC transport | audited, partial | minimal | open gap | schema compare, codec tests, control-stream golden tests | critical | current Rust logical registration types exist, but Cap'n Proto schema parity is not proven |
| CDC-002 | registration wire encoding | tunnelrpc registration client and server transport path | actual registration encoding and decoding must match the frozen wire contract | current QUIC transport | audited, partial | weak | open gap | real wire codec tests, frozen-fixture registration exchange tests | critical | current path appears JSON-shaped for bounded exchange and is not enough to claim Cap'n Proto wire parity |
| CDC-003 | registration response semantics | registration response contract and connection details shape | success vs error semantics and returned fields must match the frozen baseline | cloudflared-proto plus current QUIC transport | audited, partial | weak | open gap | response golden tests, connection-details contract tests | high | logical response helpers exist, but the actual response contract is not parity-backed |
| CDC-004 | control stream lifecycle | transport lifecycle and registration lifecycle baseline | control stream open, registration sent, lifecycle events emitted, completion observed, and failures handled as upstream expects | current protocol plus current QUIC transport | audited, partial | partial | open gap | lifecycle integration tests, control-stream event tests | high | current runtime reports some stages, but lifecycle parity is not yet proven |
| CDC-005 | ConnectRequest schema | QUIC metadata protocol schema | per-stream request shape, enum values, and metadata fields must match the frozen protocol | cloudflared-proto | audited, partial | minimal | open gap | schema compare, wire roundtrip tests, enum-value tests | critical | current logical types are promising, but logical type presence is not parity proof |
| CDC-006 | ConnectRequest wire framing | incoming stream parsing path | actual stream framing and parsing must match the frozen wire format | current QUIC transport | audited, partial | partial local tests only | open gap | frozen-fixture wire tests, framing tests, malformed-input tests | critical | current parser exists and local tests exist, but there is no frozen-baseline-backed proof yet |
| CDC-007 | ConnectResponse schema | QUIC metadata protocol schema | per-stream response error and metadata shape must match the frozen protocol | cloudflared-proto | audited, partial | minimal | open gap | schema compare, response contract tests | high | current logical response helpers exist, but response parity is not proven |
| CDC-008 | incoming stream round-trip | stream-serving runtime path | request must be accepted, processed, proxied where implemented, and returned through the full tunnel path | current proxy plus current QUIC transport | audited, partial | weak | open gap | end-to-end stream tests, origin round-trip tests | critical | current docs explicitly say round-trip parity is not yet implied |
| CDC-009 | management service routes | management service contract | ping, host details, logs, and diag-gated routes must match the frozen lane-relevant baseline | none in current Rust | audited, absent | not present | open gap | endpoint contract tests, route inventory tests | critical | hidden CLI helpers depend on this area too; treat contract ownership as CDC even where invocation begins from CLI |
| CDC-010 | log streaming contract | tail and management surfaces | remote log streaming session behavior, limits, output shaping, and auth expectations must match baseline | none in current Rust | audited, absent | not present | open gap | websocket or session tests, output-shaping tests | high | belongs to CDC, not merely CLI, because the contract is Cloudflare-facing |
| CDC-011 | readiness response contract | metrics and readiness surface | externally visible readiness endpoint shape and status semantics must match the frozen baseline where lane-relevant | current runtime | audited, absent | not present | open gap | HTTP contract tests, readiness semantic tests | high | current runtime tracks readiness internally, but not the baseline external endpoint contract |
| CDC-012 | metrics endpoint contract | metrics surface | externally visible metrics endpoint and exported metric contract must match the lane-relevant baseline surface | none in current Rust | audited, absent | not present | open gap | endpoint tests, metrics-scrape tests, metric-name and label checks | medium | keep this scoped to frozen-lane parity rather than every possible metric surface |
| CDC-013 | Cloudflare API request and response contracts | command-to-Cloudflare API baseline and design-audit API contracts | tunnel, route, vnet, token, and management helpers must use stable request and response shapes | none in current Rust | not audited | not present | unknown | API contract inventory, golden JSON tests, request-shape tests | critical | large surface; split into feature-group documents before claiming useful audit coverage |
| CDC-014 | management auth behavior | management token and route-gating behavior | token requirements, auth failure behavior, and route gating must match baseline | none in current Rust | audited, absent | not present | open gap | auth contract tests, route-gating tests | high | crosses CLI, CDC, and HIS, but the network-facing contract is CDC-owned |
| CDC-015 | diagnostics exposure via management | management and diagnostic route behavior | diagnostic routes must be conditionally exposed and gated as in the frozen baseline | none in current Rust | audited, absent | not present | open gap | route exposure tests, gating tests | medium | coordinate with HIS diagnostics implementation, but keep route contract ownership in CDC |
| CDC-016 | protocol event model | transport-to-proxy seam and lifecycle reporting | runtime-visible protocol events should accurately represent upstream contract transitions | current protocol module | audited, partial | partial local tests only | open gap | lifecycle mapping tests, event-transition tests | medium | useful internal seam, but internal event coverage alone is not parity proof |

## Immediate Work Queue

1. extract the field-level registration schema and method set from the frozen Cap'n Proto baseline
2. record the actual frozen registration wire encoding and framing behavior separately from Rust logical-type coverage
3. compare current Rust registration and stream types against the frozen schemas field by field
4. inventory current Rust actual wire behavior and record where it differs from the frozen contract
5. inventory management routes, auth gates, and diagnostics exposure from the frozen baseline
6. inventory log-streaming session behavior, limits, and output contract from the frozen baseline
7. inventory externally relevant readiness and metrics contracts for the declared lane
8. split this ledger into feature-group documents for registration RPC, stream contracts, management and diagnostics, and metrics and readiness once the next audit pass lands
