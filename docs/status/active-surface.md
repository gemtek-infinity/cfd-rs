# Active Surface Status

This file captures the currently admitted executable surface and the immediate
deferred scope around it.

## Active Phase 3.5 Focus

Phase 3.3 owns the QUIC tunnel core. Phase 3.4 adds the Pingora proxy seam
above it. Phase 3.5 adds the wire/protocol boundary between them.

What exists now (3.3 + 3.4a–c + 3.5):

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

What 3.5 does not imply:

- that registration RPC content (capnp) is implemented
- that incoming request stream handling exists
- that the admitted origin path is general proxy completeness
- that security/compliance operational behavior exists
- that standard-format crate integration beyond active-slice need exists
- that packaging, installers, updaters, or deployment tooling already exist

## Deferred Within Big Phase 3

The following later Big Phase 3 slices remain intentionally deferred:

- 3.6 security/compliance operational boundary
- 3.7 standard-format crate integration boundary

## Deferred Beyond Big Phase 3

The following remain intentionally out of the current executable-surface task:

- broader platform parity beyond Linux
- broader artifact scope beyond GNU `x86-64-v2` and `x86-64-v4`
- broader Pingora proxy completeness beyond the narrow admitted origin path
- registration RPC, incoming stream handling, and broader protocol work
  outside their later owning slices
- packaging, deployment tooling, container support, and
  certification-proving work beyond the current numbered Big Phase 3 slice list

## Follow-On Constraints For Later Slices

Phase 3.6 (security/compliance operational boundary):

- FIPS is part of the production-alpha lane but no operational crypto
  boundary exists yet in the Rust workspace

Phase 3.7 (standard-format crate integration boundary):

- no standard-format crate integration beyond the active-slice minimum
  has been admitted

Immediate narrowness caveat:

- the admitted origin path is `http_status` only; all other origin service
  types return 502 until later slices implement real origin connections
- `PingoraProxySeam` is not a general Pingora proxy; it is a confined
  entry point for the first admitted path
- the protocol bridge carries registration events only; incoming request
  streams and registration RPC content remain deferred
