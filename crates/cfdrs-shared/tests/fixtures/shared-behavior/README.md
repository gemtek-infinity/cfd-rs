# Shared-Behavior Fixtures

This tree holds the evergreen fixture inventory for shared config,
credentials, and ingress parity evidence.

Categories:

- `config-discovery/`
- `yaml-config/`
- `credentials-origin-cert/`
- `ingress-normalization/`
- `ordering-defaulting/`
- `invalid-input/`
- `golden/`

Rules:

- fixture IDs are defined in `fixture-index.toml`
- every fixture must cite frozen Go source or test references
- category directories stay explicit on purpose
- checked-in goldens are preferred over approval-style snapshots
