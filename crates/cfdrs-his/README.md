# cfdrs-his

Host interaction services for cloudflared.

## Ownership

This crate owns:

- config file discovery and lookup paths (XDG, system paths, flag overrides)
- credential file lookup and origin cert locator behavior
- filesystem layout contracts (config dir, cert dir, log dir)
- service installation and uninstallation (systemd unit generation)
- systemd integration (unit templates, update timer, enable/start)
- supervision integration boundaries
- watcher and reload host interactions (file watcher, reload action loop)
- remote config update behavior
- local HTTP endpoint exposure (metrics server, ready endpoint, Prometheus)
- signal handling (SIGTERM, SIGHUP, SIGINT behavior)
- logging to files and structured output
- environment and privilege assumptions (UID, capabilities)
- deployment evidence (runtime checks, version artifacts)

This crate does not own:

- user-facing CLI grammar or help text (`cfdrs-cli`)
- process startup or runtime orchestration (`cfdrs-bin`)
- Cloudflare-facing RPC, wire, or stream contracts (`cfdrs-cdc`)
- transport or proxy implementation (`cfdrs-cdc`)
- cross-domain shared types or error plumbing (`cfdrs-shared`)

## Governing parity docs

- `docs/parity/his/implementation-checklist.md` — 74-row HIS parity ledger
- `docs/parity/his/service-installation.md`
- `docs/parity/his/filesystem-and-layout.md`
- `docs/parity/his/diagnostics-and-collection.md`
- `docs/parity/his/reload-and-watcher.md`

## Baseline surfaces

HIS-001 through HIS-074 from the HIS parity ledger. 45 lane-required items,
27 deferred, 2 non-lane.

Key baseline sources:

- `config/` — config discovery and loading
- `credentials/` — credential file lookup
- `cmd/cloudflared/linux_service.go` — service install/uninstall
- `supervisor/` — supervision and reconnection
- `watcher/` — file watcher
- `diagnostic/` — diagnostics collection
- `metrics/` — metrics and readiness endpoints
- `signal/` — signal handling

## Current status

Filesystem config discovery IO now lives in this crate. The former
`cloudflared-config` crate has been dissolved — discovery IO functions
(`find_default_config_path`, `find_or_create_config_path`,
`discover_config`) were moved here. Discovery types and constants remain
in `cfdrs-shared`.

New HIS implementations land directly here.

## Known gaps and next work

- Implement service install and uninstall (entirely absent — critical)
- Implement systemd unit template generation (absent — critical)
- Implement local HTTP endpoints: metrics server, ready, Prometheus (absent)
- Implement file watcher and reload action loop (absent)
- Implement signal handling behavior (absent)
- Implement logging file artifacts (absent)
