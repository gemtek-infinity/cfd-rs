# First-Slice Fixtures

This fixture tree is the seed inventory for the accepted first slice.

Categories:

- config discovery cases in `config-discovery/`
- YAML/config parse cases in `yaml-config/`
- credentials and origin-cert cases in `credentials-origin-cert/`
- ingress normalization cases in `ingress-normalization/`
- ordering and defaulting cases in `ordering-defaulting/`
- invalid-input cases in `invalid-input/`
- golden contracts and captured outputs in `golden/`

Rules:

- fixture IDs are defined in `fixture-index.toml`
- every fixture must cite a frozen Go test or spec section
- category directories are explicit on purpose; avoid adding multi-purpose mixed
  fixture files at the tree root
- do not copy frozen Go PEM fixtures unless a runner requires local copies
- use checked-in explicit goldens, not approval-style snapshots, when final
  harness reports are generated

Phase 1A boundary:

- this tree defines the taxonomy and report contract only
- `golden/go-truth/` is intentionally empty until Go capture runs exist
- `golden/rust-actual/` is intentionally empty until Phase 1B code can emit
  reports for comparison
