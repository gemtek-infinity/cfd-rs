# Rust Actual Outputs

This directory is reserved for canonical JSON emitted by the Rust-side first
slice harness path.

Current state in Phase 1B.2:

- config discovery and config loading fixtures can now emit Rust actual reports
- the files are generated via `python3 tools/first_slice_parity.py emit-rust-actual`
- the output remains incomplete for first-slice categories that are still out of
 scope for this phase

The directory should remain reviewable and stable:

- one JSON file per emitted fixture id
- envelope shape must follow `golden/schema/README.md`
- no file here should imply Go parity unless the corresponding Go truth exists
