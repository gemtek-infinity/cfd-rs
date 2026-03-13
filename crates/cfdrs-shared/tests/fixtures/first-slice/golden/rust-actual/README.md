# Rust Actual Outputs

This directory is reserved for canonical JSON emitted by the Rust-side first
slice harness path.

Current state after Phase 1B.6:

- config discovery and config loading fixtures can emit Rust actual reports
- credentials/origin-cert fixtures can emit Rust actual reports
- ingress normalization and ordering/defaulting fixtures can emit Rust actual reports
- the files are generated via `python3 tools/first_slice_parity.py emit-rust-actual`
- the compare workflow can also emit fresh temporary Rust actual artifacts on demand
- the accepted first-slice compare now runs green against the checked-in Go truth
- the output remains intentionally limited to the accepted first-slice categories

The directory should remain reviewable and stable:

- one JSON file per emitted fixture id
- envelope shape must follow [golden/schema/README.md](../schema/README.md)
- no file here should imply Go parity unless the corresponding Go truth exists
