# Go Truth Outputs

This directory now holds the checked-in Go truth artifacts for the accepted
first-slice fixture surface.

The files are generated via:

- `python3 tools/first_slice_parity.py capture-go-truth`

Generation model:

- the Python harness stages a small checked-in Go helper in a temporary module
- that helper imports the frozen Go baseline from
 `baseline-2026.2.0/old-impl/` using a local `replace`
- the helper writes one canonical JSON file per fixture ID using the envelope
 documented in `../schema/README.md`

The first-slice harness must fail if a fixture is selected for comparison and
its `go-truth/<fixture-id>.json` file is missing.

Current truthfulness rule:

- these artifacts make the compare loop real
- they do not imply that Rust parity is complete
- the full compare still reports live mismatches for part of the accepted
 first-slice surface
