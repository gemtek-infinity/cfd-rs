# cloudflared-config Test Harness

This directory owns the parity harness and fixtures for the accepted first
slice only:

- config discovery/loading/normalization
- credentials surface
- ingress normalization, ordering, and defaulting

It must not grow into a general runtime or transport test area before those
subsystems start.

Current state:

- fixture inventory and seed fixtures exist
- no subsystem behavior is implemented yet
- no executable parity runner exists yet

Source-of-truth rule:

- use `baseline-2026.2.0/old-impl/` code and tests first
- use `baseline-2026.2.0/design-audit/` second

Do not modify frozen inputs from this test area.
