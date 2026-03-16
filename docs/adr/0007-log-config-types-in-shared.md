# ADR 0007: Log Configuration Types In cfdrs-shared

- Status: Accepted
- Date: 2026-03-16

## Context

The workspace architecture contract in [`STATUS.md`](../../STATUS.md) says
shared types belong in `cfdrs-shared` "only when more than one top-level
domain needs them."

Log configuration types (`LogLevel`, `LogFormat`, `LogConfig`, `RollingConfig`,
`FileConfig`, `ConsoleConfig`, `build_log_config`, permission constants) were
originally placed in `cfdrs-his` because the first consumer was the host
logging sink surface (HIS-063 through HIS-068).

As parity work progressed, multiple domains now need these types:

- `cfdrs-cli` needs `LogLevel` and `LogFormat` for parsing `--loglevel`,
  `--transport-loglevel`, and `--log-format-output` (CLI-003, CLI-023,
  CLI-024)
- `cfdrs-cdc` needs `LogLevel` for management token log-level filtering
  (CDC-023, CDC-024, CDC-026) — it currently maintains a separate wire
  `LogLevel` in `log_streaming.rs` with different derives and variants, which
  is intentionally kept separate as a wire-protocol type
- `cfdrs-his` needs the types for host sink wiring (HIS-063 through HIS-068)
- `cfdrs-bin` composes all of the above at runtime

Keeping the config types in `cfdrs-his` would require `cfdrs-cli` to depend
on `cfdrs-his`, violating the architecture contract that CLI, CDC, and HIS
must not depend on each other directly.

## Decision

Move log configuration types from `cfdrs-his` to `cfdrs-shared`:

### Types that move to `cfdrs-shared`

- `LogLevel` (5 variants: Debug, Info, Warn, Error, Fatal)
- `LogFormat` (Text, Json)
- `ConsoleConfig`
- `FileConfig`
- `RollingConfig`
- `LogConfig`
- `build_log_config()` function
- `LOG_FILE_PERM_MODE`, `LOG_DIR_PERM_MODE`, `DEFAULT_LOG_DIRECTORY`

### Types that stay in `cfdrs-his`

- `LogSink` trait (host sink contract — only `cfdrs-bin` implements it)
- `JOURNALCTL_COMMAND`, `JOURNALCTL_ARGS`, `FALLBACK_LOG_PATH` (host log
  collection constants for HIS-036)

### Backward compatibility

`cfdrs-his::logging` re-exports all moved types from `cfdrs-shared` so that
existing import paths continue to work. New code should import from
`cfdrs-shared` directly.

### No new dependencies

The moved types use only `std::path`, `std::str::FromStr`, and the existing
`ConfigError`/`Result` types already in `cfdrs-shared`. No new crate
dependencies are introduced.

### CDC LogLevel remains separate

The wire-protocol `LogLevel` in `cfdrs-cdc::log_streaming` is intentionally
separate. It has different derives (`Serialize`, `Deserialize`, `Ord`),
different variant set (no `Fatal`), and different semantics (Cloudflare
management WebSocket protocol). The shared `LogLevel` is the config-facing
type matching Go `--loglevel`.

## Consequences

- `cfdrs-cli` can use `LogLevel` and `LogFormat` for flag parsing without
  depending on `cfdrs-his`
- the architecture contract (CLI, CDC, HIS must not depend on each other)
  is preserved
- `cfdrs-shared` grows a `config::logging` module — this is consistent with
  `cfdrs-shared` already owning other cross-domain config types
- existing code using `cfdrs_his::logging::LogLevel` continues to compile via
  re-exports but should migrate to `cfdrs_shared::LogLevel` over time
