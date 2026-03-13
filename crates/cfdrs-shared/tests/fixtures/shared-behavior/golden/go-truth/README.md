# Go Truth Outputs

This directory holds the checked-in Go truth artifacts for shared-behavior fixtures.

Generation command:

- `python3 tools/shared_behavior_parity.py capture-go-truth`

Rules:

- one canonical JSON file per fixture ID
- missing Go truth must fail comparison for any selected fixture
- these files make the compare loop real for the covered shared surfaces only
- they do not imply broader rewrite parity by themselves
