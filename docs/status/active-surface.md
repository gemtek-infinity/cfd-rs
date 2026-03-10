# Active Surface Status

This file captures the currently admitted executable surface and the immediate
deferred scope around it.

## Active Phase 3.4 Focus

Phase 3.3 owns the QUIC tunnel core. Phase 3.4 adds the Pingora proxy seam
above it.

What exists now (3.3 + 3.4a–c):

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
- the transport core still stops honestly after QUIC establishment where
  later wire/protocol registration does not yet exist

What 3.4 does not imply:

- that the admitted origin path is general proxy completeness
- that wire/protocol behavior beyond the transport-owned boundary exists
- that security/compliance operational behavior exists
- that standard-format crate integration beyond active-slice need exists
- that packaging, installers, updaters, or deployment tooling already exist

## Deferred Within Big Phase 3

The following later Big Phase 3 slices remain intentionally deferred:

- 3.5 wire/protocol boundary
- 3.6 security/compliance operational boundary
- 3.7 standard-format crate integration boundary

## Deferred Beyond Big Phase 3

The following remain intentionally out of the current executable-surface task:

- broader platform parity beyond Linux
- broader artifact scope beyond GNU `x86-64-v2` and `x86-64-v4`
- broader Pingora proxy completeness beyond the narrow admitted origin path
- wire/protocol, security/compliance, and standard-format integration work
  outside their later owning slices
- packaging, deployment tooling, container support, and
  certification-proving work beyond the current numbered Big Phase 3 slice list

## Follow-On Constraints For Later Slices

Phase 3.5 (wire/protocol boundary):

- the proxy seam currently has no wire-level integration with the QUIC
  transport; 3.5 must bridge the transport session to the proxy layer
- the transport core still stops at QUIC establishment; 3.5 must carry
  protocol registration through that boundary

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
