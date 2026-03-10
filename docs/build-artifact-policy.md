# Build And Artifact Policy

This document defines the build, CI, and shipped-artifact policy for the frozen
Linux production-alpha lane.

It exists to keep local developer builds, CI validation, and shipped artifact
claims aligned without implying broader implementation or release automation
than the repository actually has.

## Active lane

This policy applies to the active lane only:

- Linux only
- target triple: `x86_64-unknown-linux-gnu`
- shipped GNU artifacts only:
  - `x86-64-v2`
  - `x86-64-v4`

This policy does not admit musl as an active alpha artifact target.

## Local developer builds

Local developer builds should remain generic by default.

That means:

- repo-default local builds must not silently hardcode a shipped CPU lane
- `.cargo/config.toml` may carry generic release tuning
- lane-specific `RUSTFLAGS` belong only in explicit build workflows or explicit
  local commands

Current repo posture:

- `.cargo/config.toml` keeps generic release tuning only
- no repo-default target CPU lane is hardcoded for normal local builds

## CI validation policy

PR CI may be narrower than release or manual artifact builds.

Current PR CI policy:

- validate the generic Linux workspace
- run formatting, `cargo check`, `cargo clippy`, and `cargo test`
- do not treat PR validation as proof that both shipped CPU lanes are fully
  operational unless lane-specific artifact builds also run

Current workflow mapping:

- `.github/workflows/on-pr-push.yml` validates the generic workspace

## Release artifact policy

The shipped artifact policy for the production-alpha lane is GNU only and lane
explicit.

Shipped release artifacts are exactly:

- `x86-64-v2`
- `x86-64-v4`

No other CPU tiers are admitted by this policy.
No musl artifacts are admitted by this policy.

## Artifact filename schema

The lane must appear in the artifact filename.

For release artifacts, the schema is:

- `cloudflared-{version}-linux-x86_64-gnu-{lane}.tar.gz`

Examples:

- `cloudflared-2026.2.0-alpha.202603-linux-x86_64-gnu-x86-64-v2.tar.gz`
- `cloudflared-2026.2.0-alpha.202603-linux-x86_64-gnu-x86-64-v4.tar.gz`

For non-release preview artifacts produced by merge or manual workflows, a git
SHA may replace `{version}` while keeping the same suffix ordering:

- `cloudflared-{git-sha}-linux-x86_64-gnu-{lane}.tar.gz`

## Checksum filename schema

Checksum filenames must make the covered artifact obvious.

The schema is:

- `{artifact_filename}.sha256`

Examples:

- `cloudflared-2026.2.0-alpha.202603-linux-x86_64-gnu-x86-64-v2.tar.gz.sha256`
- `cloudflared-2026.2.0-alpha.202603-linux-x86_64-gnu-x86-64-v4.tar.gz.sha256`

## Workflow matrix policy

The matrix policy is intentionally split by workflow type.

PR validation workflows:

- may stay narrower than release or manual artifact workflows
- should validate the generic Linux workspace
- should not silently select a shipped CPU lane by default

Merge or manual artifact workflows:

- may emit lane-specific preview artifacts
- should use the explicit GNU lane matrix:
  - `x86-64-v2`
  - `x86-64-v4`
- should keep lane naming explicit in uploaded artifacts and checksum files

Current repo posture:

- PR CI validates the generic workspace
- merge or manual preview artifact workflows may be more specific than PR CI
- full release publication and packaging automation are still outside this
  policy slice unless the workflow actually exists

## Explicit non-goals

This policy does not:

- implement transport / TLS / crypto ADR decisions beyond the already locked
  lane statements
- implement Pingora critical-path behavior
- define FIPS-in-alpha operational behavior
- define the deployment contract
- admit packaging, installers, updater work, or broader platform support
- imply that both lanes are production-proven merely because artifact builds can
  be emitted
