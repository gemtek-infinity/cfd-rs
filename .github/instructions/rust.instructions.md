---
applyTo: "**/*.rs,**/Cargo.toml"
---

# Rust and manifest instructions for cfd-rs

When editing Rust code or Cargo manifests in this repository:

- prefer the smallest source-grounded change
- preserve externally visible behavior over stylistic rewrites
- do not add dependencies unless the active owning slice justifies them
- follow `docs/dependency-policy.md` before changing manifests
- for first-slice work, prefer synchronous and deterministic code
- do not introduce async/runtime structure early unless the accepted slice requires it
- avoid repo-wide refactors unless explicitly requested
- if evidence is incomplete, say so explicitly
