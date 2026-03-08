# First-Slice Fixtures

This fixture tree is the seed inventory for the accepted first slice.

Categories:

- config discovery cases
- config loading cases
- credentials source references
- CLI-origin synthesis cases
- golden output placeholders and report-shape rules

Rules:

- fixture IDs are defined in `fixture-index.toml`
- every fixture must cite a frozen Go test or spec section
- do not copy frozen Go PEM fixtures unless a runner requires local copies
- use checked-in explicit goldens, not approval-style snapshots, when final
  harness reports are generated
