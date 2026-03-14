# cfdrs-shared

Narrowly admitted shared types for `cloudflared`.

## Owns

- shared config, credential, ingress, and error types used by more than one domain
- artifact conversion types used by shared parity evidence

## Rules

- keep single-owner types out of this crate
- do not use this crate as a convenience dump for cross-domain shortcuts
- preserve the dependency contract in [`STATUS.md`](../../STATUS.md)

## Governing docs

- [`STATUS.md`](../../STATUS.md)
- [`docs/phase-5/roadmap.md`](../../docs/phase-5/roadmap.md)
- [`REWRITE_CHARTER.md`](../../REWRITE_CHARTER.md)
