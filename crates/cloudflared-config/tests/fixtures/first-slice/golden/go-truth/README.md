# Go Truth Outputs

This directory is intentionally empty in Phase 1A.

Future Go-capture runs must write one canonical JSON file per fixture ID using
the envelope documented in `../schema/README.md`.

The first-slice harness must fail if a fixture is selected for comparison and
its `go-truth/<fixture-id>.json` file is missing.
