# Active Surface Status

This file captures the currently admitted executable surface and the immediate
deferred scope around it.

## Active Phase 5.1 Surface

Phase 3.3 owns the QUIC tunnel core. Phase 3.4 adds the Pingora proxy seam
above it. Phase 3.5 adds the wire/protocol boundary between them. Phase 3.6
adds a narrow security/compliance operational boundary around the admitted
quiche + BoringSSL lane. Phase 3.7 admits the minimum standard-format crate
boundary required by the active runtime path. Phase 4.1 adds the minimum
observability and operability surface required to run and inspect that alpha
honestly. Phase 4.2 adds deterministic performance validation with
stage-transition timing evidence, cold vs resumed path distinction, and
explicit regression thresholds. Phase 4.3 adds deterministic failure-mode and
recovery proof for the admitted alpha path. Phase 4.4 adds internal
deployment proof for the admitted alpha lane. Phase 5.1 is the current
admitted slice and adds broader proxy dispatch, wire-format types, and
incoming QUIC data stream handling.

What exists now (3.3 + 3.4a–c + 3.5 + 3.6 + 3.7 + 4.1 + 4.2 + 4.3 + 4.4 + 5.1):

- `run` enters a real quiche-based transport service under the runtime boundary
- connection/session ownership and QUIC handshake state are explicit
- runtime-owned config handoff feeds the transport identity boundary
- reconnect/restart policy remains owned by runtime supervision
- the Pingora proxy seam is admitted and confined to
  `crates/cloudflared-cli/src/proxy/`
- the proxy seam participates in runtime lifecycle (startup/shutdown)
- the first admitted origin/proxy path routes `http_status` ingress rules
  through the Pingora-owned seam
- origin services not yet implemented return 502 honestly
- the wire/protocol boundary is owned by
  `crates/cloudflared-cli/src/protocol.rs`
- after QUIC establishment, the transport opens the control stream
  (client-initiated bidi stream 0) at the wire/protocol boundary
- the transport sends a protocol registration event to the proxy layer
  through an explicit protocol bridge
- the proxy layer receives and acknowledges the registration event
- the runtime creates and distributes the protocol bridge endpoints
  to transport (sender) and proxy (receiver)
- runtime startup now reports the bounded security/compliance operational
  boundary for the admitted lane and keeps claims explicit and narrow
- runtime startup now enforces Linux GNU/glibc deployment-contract assumptions
  for the admitted lane and fails honestly when required host assumptions are
  missing
- the active origin-cert runtime path now uses a workspace-managed mature PEM
  crate through owned credential adapters in
  `crates/cloudflared-config/src/credentials/`
- direct third-party PEM handling remains confined to that owned credential
  boundary rather than leaking across runtime, transport, proxy, or app code
- runtime-owned observability now reports lifecycle transitions, owner-scoped
  transport/protocol/proxy state, and failure boundaries live while `run`
  executes
- the runtime now derives and reports a narrow readiness state for the current
  alpha role rather than implying broader request-serving readiness
- the runtime now emits a minimal final operability snapshot with restart,
  proxy-admission, protocol-registration, and failure counters
- transport lifecycle stage transitions are now timed relative to runtime start
  with wall-clock millisecond-resolution evidence, including handshake and
  edge-resolution stages when the real transport path is exercised
- cold-start (attempt 0) vs resumed (attempt > 0) transport paths are
  distinguished in performance evidence output
- machine-readable performance evidence lines (`perf-*`) are emitted at
  runtime finish for structured log parsing and CI gate evaluation
- 0-RTT lane configuration truth is reported as evidence (quiche+boringssl
  with early_data enabled); actual session resumption savings remain deferred
- pipeline latency from proxy admission to full readiness is measured and
  gated as a regression threshold
- handshake duration (handshaking-to-established) is measured when the real
  transport handshake path is exercised
- explicit regression thresholds gate proxy-admission, service-ready,
  readiness, restart-overhead, pipeline-latency, and total-runtime timing
- threshold violations are reported as a pass/fail gate in summary output
- evidence scope is honestly reported, distinguishing in-process harness
  timing from deferred real wire latency and 0-RTT resumption measurement
- machine-readable failure evidence lines (`failure-*`) are emitted at
  runtime finish alongside performance evidence
- reconnect/retry behavior is bounded: the runtime tracks restart budget
  consumption and reports exhaustion explicitly
- shutdown behavior is observable through lifecycle state transitions
  and child-task drain reporting
- dependency-boundary failures are reported with explicit owner and class
  at each failure event
- config-reload is honestly declared as not supported: config is frozen at
  startup handoff, no SIGHUP handler or reload command exists
- malformed YAML, invalid ingress rules, and structurally invalid config
  fields fail at the config boundary with typed, machine-readable error
  categories
- failure evidence scope is honestly reported, distinguishing in-process
  harness failure proof from deferred real transport reconnect and
  deployment-level recovery
- machine-readable deployment evidence (`deploy-*`) is emitted at runtime
  finish alongside performance and failure evidence
- the deployment contract (Linux, x86\_64, GNU/glibc, bare-metal-first,
  systemd-expected) is validated at startup and reported in evidence
- glibc runtime markers, systemd supervision, binary path, and config path
  are reported in deployment evidence
- known deployment gaps (no systemd unit, no installer, no container image,
  no updater, no log rotation) are declared explicitly
- operational caveats (alpha-only, limited origin dispatch, no Cap'n Proto
  registration RPC, no origin-cert registration content, no stream
  round-trip, no config reload) are declared explicitly
- deployment evidence scope is honestly bounded to in-process contract
  validation
- operator-facing deployment notes exist in `docs/deployment-notes.md`
  matching the declared deployment contract from ADR-0005
- the CI merge workflow produces lane-specific preview artifacts for both
  shipped GNU lanes (x86-64-v2 and x86-64-v4)
- wire-format types for per-stream request/response exchange are owned by
  `crates/cloudflared-proto/` (ConnectionType, ConnectRequest,
  ConnectResponse, Metadata)
- registration RPC type boundaries (TunnelAuth, ConnectionOptions,
  ConnectionDetails) are defined in `crates/cloudflared-proto/`
- the control stream now carries a bounded credentials-file registration
  request/response exchange; successful responses produce
  `RegistrationComplete` events with connection UUID and location
- the proxy seam now dispatches broader origin services: HelloWorld returns
  a real HTML response, Http-origin dispatch is wired (returns 502 until
  actual proxying is implemented), and unimplemented services are labeled
  honestly
- the origin dispatch surface is owned by
  `crates/cloudflared-cli/src/proxy/origin.rs`
- after QUIC establishment the transport enters a stream-serving loop that
  accepts server-initiated bidi streams, parses ConnectRequest wire format,
  and forwards IncomingStream events through the protocol bridge
- the protocol bridge now carries IncomingStream and RegistrationComplete
  events in addition to the original registration event
- the transport lifecycle includes a ServingStreams stage after Established

What the current surface does not imply:

- that Cap'n Proto registration RPC parity is implemented
- that origin-cert identity currently emits registration content
- that incoming streams are round-tripped through origin and back to edge
- that the admitted origin dispatch is general proxy completeness (actual
  HTTP proxying, WebSocket upgrade, TCP streaming remain deferred)
- that the bounded security/compliance operational boundary constitutes
  certification, whole-program compliance, or validated FIPS implementation
- that broader standard-format crate integration beyond active-slice need
  exists
- that the narrow readiness signal implies broad admin, deployment, or
  production-proof surfaces
- that performance evidence implies real QUIC wire latency measurement,
  0-RTT session resumption savings, or end-to-end request latency
- that failure-mode evidence implies real QUIC transport reconnect,
  deployment-level process recovery, or config-reload behavior
- that deployment evidence implies real systemd integration, package-manager
  delivery, container support, or log-rotation integration
- that packaging, installers, updaters, or deployment tooling already exist

## Deferred Within Big Phase 3

No later Big Phase 3 slice is admitted here beyond the active 3.7 minimum.

## Deferred Beyond Big Phase 3

The following remain intentionally out of the current executable-surface task:

- broader platform parity beyond Linux
- broader artifact scope beyond GNU `x86-64-v2` and `x86-64-v4`
- broader Pingora proxy completeness beyond the admitted origin-dispatch
  surface (actual HTTP proxying, WebSocket upgrade, TCP streaming)
- Cap'n Proto registration RPC parity, origin-cert registration content,
  and full request stream round-trip through origin and back to edge
- packaging, deployment tooling, container support, and
  certification-proving work beyond the current numbered Big Phase 3 slice list
- broader deployment/management work beyond the admitted 4.4 deployment proof
  surface and 5.1 stream-serving surface (real systemd unit files, installers,
  container images, updaters, log rotation)
- broader performance proof beyond the admitted harness path

## Follow-On Constraints For Later Slices

Phase 3.6 (security/compliance operational boundary):

- the bounded operational crypto/TLS surface is now explicitly reported at
  runtime startup and scoped to the quiche + BoringSSL transport lane only
- Linux GNU/glibc deployment-contract assumptions are now enforced at startup
- the operational boundary is not certification, not whole-program FIPS, and
  not validated compliance proof — those remain Big Phase 4 work

Phase 3.7 (standard-format crate integration boundary):

- the admitted standard-format boundary is limited to PEM container handling
  needed by the active origin-cert runtime path
- the PEM crate enters through owned credential adapters in
  `crates/cloudflared-config/src/credentials/`
- broader certificate/key container handling, broader format coverage, and
  later protocol/runtime parsing work remain deferred

Immediate narrowness caveat:

- the admitted origin dispatch handles `http_status` and `hello_world`;
  Http-origin dispatch is wired but returns 502 until actual proxying;
  remaining origin types return 502 honestly
- `PingoraProxySeam` is not a general Pingora proxy; it is a confined
  entry point for the admitted dispatch surface
- incoming QUIC data streams are accepted and parsed but not yet
  round-tripped through origin and back to edge
- bounded credentials-file registration exchange exists; Cap'n Proto parity
  and origin-cert registration content remain deferred
