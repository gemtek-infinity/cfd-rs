# Rust Actual Outputs

This directory holds canonical JSON emitted by the Rust-side shared-behavior emitter.

Generation command:

- `python3 tools/shared_behavior_parity.py emit-rust-actual`

Rules:

- one JSON file per emitted fixture ID
- the envelope must follow `golden/schema/README.md`
- no file here implies parity unless matching Go truth exists and compare passes
