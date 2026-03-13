# Dependency Policy

This document defines how dependencies enter the Rust rewrite workspace.

The repository already has a Rust scaffold. Dependency policy therefore exists
to keep that scaffold minimal, honest, and aligned with the accepted rewrite
slice, not to predeclare the full future dependency graph.

## Non-Negotiable Constraints

- [baseline-2026.2.0/old-impl/](../baseline-2026.2.0/old-impl/) is frozen input
- [docs/parity/source-map.csv](parity/source-map.csv) is the derived row-to-baseline routing surface
- the Rust workspace version remains `2026.2.0-alpha.202603` until changed by
  explicit baseline/versioning policy
- manifests should describe code that exists today or the currently accepted
  next slice, not speculative later slices

## Admission Principles

Dependencies are admitted only when all of the following are true:

1. the owning subsystem slice is accepted
2. the dependency is justified by source-backed behavior or test needs
3. the crate boundary that owns the dependency is clear
4. the dependency does not quietly redesign externally visible behavior
5. a standard-library alternative is not sufficient

When multiple valid choices still satisfy those rules:

- prefer stronger domain typing over generic `String` or `Vec<u8>` storage when
  the value already has a stable semantic shape such as `Uuid`, `SocketAddr`,
  `IpAddr`, `Url`, or a dedicated newtype
- prefer mature, production-ready, actively maintained crates for encoding,
  decoding, parsing, and validation work over ad hoc local representations or
  handwritten edge-case handling

## Default Workspace Rules

The default dependency truth for the workspace is now:

- normal workspace-managed third-party dependency truth should normally be
  declared in `[workspace.dependencies]`
- reviewers should expect to inspect the root manifest first when evaluating
  normal third-party dependency version and feature choices
- per-crate version declarations are exceptions and should stay local only when
  the dependency is intentionally crate-private, tool-private, experimental, or
  clearly slice-isolated
- dependency admission remains tied to active slices only; policy examples do
  not authorize future crates in advance
- root-manifest-first review is the default model even when a dependency is not
  yet widely shared across many workspace members
- centralization for reviewability and consistency is allowed before a normal
  third-party dependency is reused across many members, provided ownership and
  scope stay explicit

## Mature Standard-Format Handling

For mature, standard, security-relevant formats:

- prefer mature crates over hand-rolled parsing or encoding when the active
  slice really needs the format today
- prefer direct upstream loaders or APIs before adding extra parsing layers
  that only duplicate an existing upstream boundary
- standard-format or container crates are not the same thing as
  crypto-implementation crates
- convenience parsing or container crates must not be used to silently widen
  application-level crypto behavior

If a change chooses bespoke parsing instead of a mature crate or a direct
upstream loader, that choice should be justified explicitly.

## Cloudflare REST API Crate Gate

`cloudflare-rs` is not admitted during preparation.

Evaluation is gated only for the CDC API slice and dependent CLI flows:

- `CDC-033`
- `CDC-034`
- `CDC-038`
- CLI tunnel CRUD and management-token flows that depend on those rows

The evaluation must reject `cloudflare-rs` for:

- transport, registration, or control-stream paths
- management log sinks or `/logs` runtime handling
- service-management or host-runtime behavior
- any other runtime-critical path

The gate checklist must cover:

- endpoint coverage against the exact required rows
- response-envelope and error-mapping fit
- auth model fit
- TLS and HTTP client fit with repo constraints
- dependency footprint and maintenance status

Default decision during preparation: no admission. Record the gate and defer the
admit-or-reject decision to the CDC API implementation slice.

## Current Workspace Rule

The current workspace is still intentionally narrow, but it is no longer an
empty scaffold.

That means:

- no future transport, RPC, or async-runtime dependencies should be declared in
  manifests before code using them exists
- placeholder crates may remain dependency-free when they contain only module
  docs
- admitted dependencies should stay confined to the crates that own the active
  slice rather than being preloaded repo-wide
- normal workspace-managed third-party dependency truth should normally live in
  `[workspace.dependencies]`, with the root manifest acting as the first review
  surface for version and feature choices
- crate-local dependency truth remains acceptable when isolation is intentional,
  justified, and clearer than forced centralization

## Current Admitted Dependencies

The current manifests admit only the dependencies needed by the binary runtime
baseline, the active shared config/credentials/ingress implementation, the
active QUIC tunnel core, the admitted Pingora and observability seams, and the
existing workspace tool surface:

- `mimalloc`, `tokio`, `tokio-util`, `quiche`, `pingora-http`, `tracing`, and
  `tracing-subscriber` in `cfdrs-bin`
- shared workspace truth for `pem`, `serde`, `serde_json`, `serde_yaml`,
  `thiserror`, `url`, `uuid`, and `base64`
- `rmcp`, `schemars`, and `tokio` in [tools/mcp-cfd-rs](../tools/mcp-cfd-rs)

Reason:

- allocator policy is still a process-wide runtime baseline owned by the binary
- the admitted runtime and lifecycle shell in `cfdrs-bin` may use `tokio` and `tokio-util`
  for owned task tracking, bounded command flow, and cancellation at the binary boundary
- the admitted QUIC tunnel core in `cfdrs-bin` may use `quiche` on the locked quiche +
  BoringSSL lane for real transport ownership and handshake/session state under the runtime
  boundary
- the admitted Pingora seam in `cfdrs-bin` may use `pingora-http` inside its owned proxy
  boundary for the current narrow origin path
- the active origin-cert path may use the mature `pem` crate through owned credential
  adapters in `cfdrs-shared`
- the admitted runtime observability surface in `cfdrs-bin` may use `tracing` and
  `tracing-subscriber` for live, owner-scoped reporting at the binary boundary
- config, credential, and ingress normalization work is active in
  `cfdrs-shared`, so its admitted slice dependencies now exist honestly in
  manifests
- credentials-file handling now depends on the mature `base64` crate so the
  tunnel secret is decoded into owned bytes at the config boundary rather than
  being carried as an encoded string into runtime and transport code
- several active rewrite crates are centralized in the root manifest already because
  root-manifest-first review and feature consistency are part of the accepted workspace policy,
  not merely an after-the-fact consequence of broad sharing
- [tools/mcp-cfd-rs](../tools/mcp-cfd-rs) is a real workspace tool, so its private dependencies may
  exist locally without authorizing those crates for rewrite crates by default
- libraries still must not set the global allocator or preload later-slice
  dependencies speculatively

## Boundary Rules

- keep third-party APIs behind local boundaries or adapters when the concern is
  security-relevant, protocol-relevant, or likely to churn
- parsing and encoding crates should not leak through unrelated public APIs
  when a local boundary keeps ownership and later review clearer
- direct upstream APIs should be used where they already solve the problem
  cleanly; extra abstraction layers should be justified, not assumed
- standard-format/container crates remain distinct from crypto-implementation
  crates and must not be treated as blanket permission for new crypto behavior

## Dependency Admission By Slice

This section records the admission gates and ongoing rules for dependencies
organized by owning slice. Dependencies whose slices are active are now in
workspace manifests. Dependencies whose slices have not yet started remain
deferred.

### Config, Credentials, And Ingress Normalization (admitted)

Slice is active. These dependencies are in `[workspace.dependencies]`:

- `serde`
- `serde_json`
- `serde_yaml`
- `url`
- `uuid`
- `thiserror`

Ongoing rules:

- `serde_yaml` is tolerated for parity work even though the upstream crate line
  carries a deprecation marker; it must remain a deliberate compatibility
  choice, not a default convenience dependency
- if a more precise YAML strategy is later required, that change needs explicit
  compatibility review rather than silent substitution

### Async Control-Plane And Data-Plane (admitted)

Slice is active. These dependencies are in `[workspace.dependencies]`:

- `tokio`
- `tokio-util`

Ongoing rules:

- their use must follow [docs/go-rust-semantic-mapping.md](go-rust-semantic-mapping.md)
- do not add alternative channel/runtime frameworks by default
- do not use convenience crates to bypass the explicit crypto and transport
  governance already frozen elsewhere

### Protocol And Wire Slices (deferred)

Admit only when protocol implementation starts:

- `bytes`
- `capnp`

Rules:

- admission must be tied to exact wire and schema preservation work
- do not add protocol libraries speculatively because the crate name exists

### Logging And Observability (admitted)

Slice is active. These dependencies are in `[workspace.dependencies]`:

- `tracing`
- `tracing-subscriber`

### Shared-Behavior Evidence Support

These are not scaffold defaults. Admit them only when shared-behavior evidence
code or implementation tests actually need them.

- `tempfile`: acceptable for deterministic filesystem-layout tests and config
  discovery harness cases
- `assert_cmd`: defer until a real CLI surface exists for the owning slice
- snapshot-style crates such as `insta`: avoid by default; prefer explicit
  checked-in golden files in fixture directories
- diff helpers such as `pretty_assertions`: avoid by default; use standard
  assertion output unless a concrete review burden justifies them

Rules:

- harness dev-dependencies for shared-behavior evidence belong in
  [crates/cfdrs-shared/Cargo.toml](../crates/cfdrs-shared/Cargo.toml), not the workspace root
- do not add snapshot tooling merely to make approval easier; first prefer
  stable JSON or text goldens checked into the repo
- shared-behavior checked-in goldens belong under
  [crates/cfdrs-shared/tests/fixtures/shared-behavior/golden/](../crates/cfdrs-shared/tests/fixtures/shared-behavior/golden/)
- CLI-process test helpers are premature until the Rust CLI actually emits the
  relevant surface

## Disallowed By Default

These require an explicit decision record before admission:

- repo-wide actor frameworks
- alternative async runtimes
- unbounded-channel libraries as a primary coordination primitive
- allocator libraries other than the accepted process allocator baseline
- Cloudflare-owned crates that are not already proven to be the best fit for an
  active slice
- speculative HTTP, QUIC, or RPC client/server stacks before the owning slice
  starts

## Crate Ownership Rules

Current crate intent is:

- `cfdrs-bin` owns process-level concerns such as allocator setup and,
  later, runtime initialization
- `cfdrs-shared` owns config types, credentials, ingress normalization, and
  error taxonomy
- `cfdrs-his` owns filesystem config discovery and host interaction services
- `cfdrs-cli` owns CLI command surface, parsing, and dispatch
- `cfdrs-cdc` owns Cloudflare-facing RPC and wire contracts

Do not accumulate dependencies in `cfdrs-shared` just because it looks like
shared infrastructure.

Do not centralize a dependency into `[workspace.dependencies]` merely because it
could exist someday; centralize normal workspace-managed third-party dependency
truth there by default, but keep crate-local declarations when isolation is the
clearer and more intentional choice.

An important case for centralization is true multi-member sharing, but that is
not the only valid reason; reviewability and consistent feature truth are also
valid reasons when the dependency is a normal workspace-managed third-party
crate and the owning scope remains clear.

## Dependency Change Checklist

Before adding a dependency, document all of the following in the change:

1. owning slice
2. owning crate
3. source-backed reason
4. why the standard library is insufficient
5. whether the dependency affects external behavior, wire bytes, config
   semantics, or shutdown behavior
6. whether the dependency should live in `[workspace.dependencies]` or remain
  intentionally crate-local, and why
7. whether the change relies on a mature crate, a direct upstream loader, or an
  explicitly justified bespoke boundary

If any of those answers are unclear, the dependency should not be added yet.
