# Golden Output Rules

Final checked-in parity goldens should use canonical JSON emitted by the
first-slice harness report kinds:

- `discovery-report.v1.json`
- `normalized-config.v1.json`
- `credential-report.v1.json`
- `ingress-report.v1.json`
- `error-report.v1.json`

Rules:

- prefer explicit checked-in golden files over approval-style snapshots
- keep one golden file per fixture ID
- compare exact canonical JSON when a harness report schema exists
- use structural or error-category comparison only where this is documented in
  `crates/cloudflared-config/tests/fixtures/first-slice/fixture-index.toml`
  and the applicable owning test code
