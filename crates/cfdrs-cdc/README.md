# cfdrs-cdc

Cloudflare data center contracts for cloudflared.

## Ownership

This crate owns:

- registration RPC contracts (Cap'n Proto schema, binary encoding, method set)
- wire and stream contracts (ConnectRequest/ConnectResponse framing, codecs)
- incoming stream round-trip behavior
- management protocol interactions (routes, auth middleware)
- log-streaming contracts (WebSocket session, limits, output shaping)
- externally relevant metrics and readiness contracts
- Cloudflare REST API client boundaries (tunnel CRUD, route, vnet)
- datagram and UDP session contracts
- transport lifecycle (QUIC connection management)
- proxy seam between transport and origin dispatch
- CDC-owned codec and wire encoding logic

This crate does not own:

- user-facing CLI grammar or help text (`cfdrs-cli`)
- process startup or runtime orchestration (`cfdrs-bin`)
- host-facing service install behavior or filesystem layout (`cfdrs-his`)
- config discovery, credential file lookup, or watcher behavior (`cfdrs-his`)
- generic shared infrastructure that is not CDC-specific (`cfdrs-shared`)

## Governing parity docs

- `docs/parity/cdc/implementation-checklist.md` — 44-row CDC parity ledger
- `docs/parity/cdc/registration-rpc.md`
- `docs/parity/cdc/stream-contracts.md`
- `docs/parity/cdc/management-and-diagnostics.md`
- `docs/parity/cdc/metrics-readiness-and-api.md`

## Baseline surfaces

CDC-001 through CDC-044 from the CDC parity ledger. 40 lane-required items,
4 deferred.

Key baseline sources:

- `tunnelrpc/` — Cap'n Proto schema and RPC
- `connection/` — QUIC connection and control stream
- `quic/` — QUIC transport
- `proxy/` — proxy dispatch
- `management/` — management service
- `cfapi/` — Cloudflare REST API client
- `datagramsession/` — UDP session management

## Current status

Partially populated. Contains:

- `src/lib.rs` — module declarations and public re-exports
- `src/registration.rs` — TunnelAuth, ConnectionOptions, ConnectionDetails,
  RegisterConnectionRequest, RegisterConnectionResponse
- `src/stream.rs` — ConnectRequest, ConnectResponse, ConnectionType, Metadata

Protocol bridge, transport, and proxy code is temporarily housed in
`crates/cfdrs-bin/` due to tight coupling with runtime types. These modules
compose CDC type boundaries and will be extracted here when runtime interface
types are formalized.

## Known gaps and next work

- Extract protocol bridge from `cfdrs-bin/src/protocol.rs` to cfdrs-cdc
- Extract transport from `cfdrs-bin/src/transport/` to cfdrs-cdc
- Extract proxy seam from `cfdrs-bin/src/proxy/` to cfdrs-cdc
- Implement registration wire encoding (Cap'n Proto binary — highest-risk gap)
- Implement stream framing and codec
- Implement management service and log streaming (entirely absent)
- Implement Cloudflare REST API client (entirely absent)
