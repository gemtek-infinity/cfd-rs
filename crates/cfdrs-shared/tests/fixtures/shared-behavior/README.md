# Shared-Behavior Fixtures

This tree holds the evergreen fixture inventory for shared config,
credentials, and ingress parity evidence.

Categories:

- [`config-discovery/`](config-discovery/)
- [`yaml-config/`](yaml-config/)
- [`credentials-origin-cert/`](credentials-origin-cert/)
- [`ingress-normalization/`](ingress-normalization/)
- [`ordering-defaulting/`](ordering-defaulting/)
- [`invalid-input/`](invalid-input/)
- [`golden/`](golden/)

Rules:

- fixture IDs are defined in [`fixture-index.toml`](fixture-index.toml)
- every fixture must cite frozen Go source or test references
- category directories stay explicit on purpose
- checked-in goldens are preferred over approval-style snapshots
