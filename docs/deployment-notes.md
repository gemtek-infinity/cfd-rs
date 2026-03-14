# Deployment Notes

Operator-facing deployment notes for the admitted Linux production-alpha lane.

These notes describe the current deployment contract, build-to-run flow,
operational caveats, and known gaps for the declared alpha surface.

For the complete gap inventory across all three parity domains, see the live
implementation checklists:

- [docs/parity/cli/implementation-checklist.md](parity/cli/implementation-checklist.md) — CLI command surface (32 rows)
- [docs/parity/cdc/implementation-checklist.md](parity/cdc/implementation-checklist.md) — Cloudflare contracts (44 rows)
- [docs/parity/his/implementation-checklist.md](parity/his/implementation-checklist.md) — host interactions (74 rows)

## Deployment Contract

The alpha deployment contract is narrow and explicit:

- **Platform**: Linux only, `x86_64-unknown-linux-gnu`
- **Operational baseline**: GNU/glibc (not musl)
- **Service model**: supervised long-running service (systemd expected)
- **Deployment stance**: bare-metal-first (not container-first)
- **Filesystem**: operator-managed host paths for executable, config,
  credentials, and logs

The governing ADR is [docs/adr/0005-deployment-contract.md](adr/0005-deployment-contract.md).

## Build-To-Run Flow

### Prerequisites

- Linux x86\_64 host with GNU/glibc
- Rust stable toolchain (`rustup toolchain install stable`)
- Working internet for crate downloads (first build only)

### Generic local build

Normal entry:

```bash
just validate-app
```

Raw reference build:

```bash
cargo build --release --locked -p cfdrs-bin
```

The resulting binary is at `target/release/cloudflared`.

### Lane-specific build (shipped artifact lanes)

For `x86-64-v2` (baseline):

```bash
just preview-build x86-64-v2
```

For `x86-64-v4` (AVX-512 capable):

```bash
just preview-build x86-64-v4
```

The resulting binary is at `target/x86_64-unknown-linux-gnu/release/cloudflared`.

### Validate startup

```bash
./cloudflared --config /path/to/config.yml validate
```

Expected output includes `OK: admitted alpha startup surface validated`.

### Run

```bash
./cloudflared --config /path/to/config.yml run
```

The runtime will:

1. validate the deployment contract (Linux, x86\_64, GNU/glibc markers)
2. report the security/compliance boundary
3. probe for systemd supervision environment
4. accept runtime-owned config handoff
5. start the proxy seam and transport core
6. enter the runtime command dispatch loop
7. emit operability, performance, failure, and deployment evidence at finish

### Minimal config example

```yaml
tunnel: 00000000-0000-0000-0000-000000000000
ingress:
  - service: http_status:503
```

The tunnel UUID and credentials file are required for a real QUIC transport
connection. Without them, the runtime starts, validates the deployment
contract, and exits with an honest failure.

## Operational Caveats

For the detailed gap inventory behind these caveats, see the parity ledgers
linked above.

- **Alpha only**: this is a production-alpha surface, not a hardened release
- **Limited origin dispatch**: `http_status` and `hello_world` are admitted;
  HTTP-origin dispatch is wired but returns 502 until actual proxying exists;
  remaining origin service types return 502 honestly
- **No Cap'n Proto registration RPC**: the bounded control-stream
  registration exchange is not parity-complete with the frozen Cap'n Proto
  registration protocol (CDC-001, CDC-002)
- **No origin-cert registration content**: only the credentials-file path
  currently emits bounded registration request content on the control stream
- **No stream round-trip**: incoming QUIC streams are accepted and parsed,
  but are not yet round-tripped through origin and back to edge (CDC-011,
  CDC-012, CDC-018)
- **No config reload**: config is frozen at startup; no SIGHUP handler or
  reload command exists (HIS-041, HIS-042)
- **No broad proxy**: the proxy seam is confined to the first admitted path
- **Signal handling**: SIGTERM and SIGINT trigger clean shutdown; no other
  signals are handled

## Known Deployment Gaps

These gaps are intentional at the current alpha stage. For the complete
host-interaction gap inventory with priority, evidence status, and divergence
records, see [docs/parity/his/implementation-checklist.md](parity/his/implementation-checklist.md).

- **No systemd unit file**: the deployment contract expects systemd
  supervision, but no unit file is shipped (HIS-012 through HIS-017)
- **No installer**: no package (deb, rpm) or install script exists
  (HIS-012, HIS-013)
- **No container image**: container deployment is not part of the alpha
  contract
- **No updater**: no automatic update mechanism exists (HIS-046, HIS-047)
- **No log rotation**: runtime logging can now write to stderr, `--logfile`,
  and `--log-directory`, but no rotation or journal integration is implemented
  (HIS-063 through HIS-065)
- **No firewall rules**: no network policy or firewall configuration is
  shipped
- **No user/group management**: the binary runs as the invoking user; no
  dedicated service account is created

## Evidence At Runtime

The runtime emits machine-readable evidence lines at finish:

- `deploy-contract:` — the governing deployment contract
- `deploy-host-validation:` — whether host assumptions passed
- `deploy-glibc-markers:` — whether GNU/glibc markers were found
- `deploy-systemd-supervision:` — whether systemd environment was detected
- `deploy-binary-path:` — the binary's resolved path
- `deploy-config-path:` — the config file path used
- `deploy-filesystem-contract:` — filesystem ownership model
- `deploy-known-gaps:` — declared deployment gaps
- `deploy-operational-caveats:` — declared operational caveats
- `deploy-evidence-scope:` — what this evidence covers vs what remains deferred

These are emitted alongside `perf-*` (performance), `failure-*` (failure-mode),
and `operability-*` (operability) evidence.

## CI Artifact Build

The merge workflow ([.github/workflows/on-pr-merge.yml](../.github/workflows/on-pr-merge.yml)) produces preview
artifacts for both shipped lanes:

- `cloudflared-{sha}-linux-x86_64-gnu-x86-64-v2.tar.gz`
- `cloudflared-{sha}-linux-x86_64-gnu-x86-64-v4.tar.gz`

Each artifact includes the binary, README, LICENSE, and a SHA-256 checksum.
Artifacts are retained for 30 days.

The artifact filename schema is defined in [docs/build-artifact-policy.md](build-artifact-policy.md).
