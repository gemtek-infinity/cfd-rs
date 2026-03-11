# Active Surface Status

This file captures the currently admitted executable surface and the immediate
deferred scope around it.

## Active Phase 4.1 Surface

Phase 3.3 owns the QUIC tunnel core. Phase 3.4 adds the Pingora proxy seam
above it. Phase 3.5 adds the wire/protocol boundary between them. Phase 3.6
adds a narrow security/compliance operational boundary around the admitted
quiche + BoringSSL lane. Phase 3.7 admits the minimum standard-format crate
boundary required by the active runtime path. Phase 4.1 is the current
admitted slice on top of that base and adds the minimum observability and
operability surface required to run and inspect that alpha honestly.

What exists now (3.3 + 3.4a–c + 3.5 + 3.6 + 3.7 + 4.1):

- `run` enters a real quiche-based transport service under the runtime boundary
- connection/session ownership and QUIC handshake state are explicit
- runtime-owned config handoff feeds the transport identity boundary
- reconnect/restart policy remains owned by runtime supervision
- the Pingora proxy seam is admitted and confined to
  `crates/cloudflared-cli/src/proxy.rs`
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
  `crates/cloudflared-config/src/credentials.rs`
- direct third-party PEM handling remains confined to that owned credential
  boundary rather than leaking across runtime, transport, proxy, or app code
- runtime-owned observability now reports lifecycle transitions, owner-scoped
  transport/protocol/proxy state, and failure boundaries live while `run`
  executes
- the runtime now derives and reports a narrow readiness state for the current
  alpha role rather than implying broader request-serving readiness
- the runtime now emits a minimal final operability snapshot with restart,
  proxy-admission, protocol-registration, and failure counters

What the current surface does not imply:

- that registration RPC content (capnp) is implemented
- that incoming request stream handling exists
- that the admitted origin path is general proxy completeness
- that the bounded security/compliance operational boundary constitutes
  certification, whole-program compliance, or validated FIPS implementation
- that broader standard-format crate integration beyond active-slice need
  exists
- that the narrow readiness signal implies broad admin, deployment, or
  production-proof surfaces
- that packaging, installers, updaters, or deployment tooling already exist

## Deferred Within Big Phase 3

No later Big Phase 3 slice is admitted here beyond the active 3.7 minimum.

## Deferred Beyond Big Phase 3

The following remain intentionally out of the current executable-surface task:

- broader platform parity beyond Linux
- broader artifact scope beyond GNU `x86-64-v2` and `x86-64-v4`
- broader Pingora proxy completeness beyond the narrow admitted origin path
- registration RPC, incoming stream handling, and broader protocol work
  outside their later owning slices
- packaging, deployment tooling, container support, and
  certification-proving work beyond the current numbered Big Phase 3 slice list
- performance proof, failure-mode proof, and broader deployment/management
  work beyond the admitted 4.1 observability surface

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
  `crates/cloudflared-config/src/credentials.rs`
- broader certificate/key container handling, broader format coverage, and
  later protocol/runtime parsing work remain deferred

Immediate narrowness caveat:

- the admitted origin path is `http_status` only; all other origin service
  types return 502 until later slices implement real origin connections
- `PingoraProxySeam` is not a general Pingora proxy; it is a confined
  entry point for the first admitted path
- the protocol bridge carries registration events only; incoming request
  streams and registration RPC content remain deferred
