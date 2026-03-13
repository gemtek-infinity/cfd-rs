# First-Slice Golden Schema

All first-slice parity artifacts use a checked-in canonical JSON envelope.

Filename convention:

- `go-truth/<fixture-id>.json`
- `rust-actual/<fixture-id>.json`

Envelope shape:

```json
{
  "schema_version": 1,
  "fixture_id": "config-basic-named-tunnel",
  "producer": "go-truth",
  "report_kind": "normalized-config.v1",
  "comparison": "exact-json",
  "source_refs": [
    "baseline-2026.2.0/old-impl/config/configuration_test.go::TestConfigFileSettings"
  ],
  "payload": {}
}
```

Field rules:

- `schema_version`: integer contract version for the artifact envelope
- `fixture_id`: must match a [fixture-index.toml](../../fixture-index.toml) entry exactly
- `producer`: `go-truth` or `rust-actual`
- `report_kind`: one of the accepted first-slice report kinds
- `comparison`: copied from [fixture-index.toml](../../fixture-index.toml) for auditability
- `source_refs`: frozen Go test or spec references backing the artifact
- `payload`: canonical JSON object or array emitted by the relevant runner

Accepted report kinds for Phase 1A planning:

- `discovery-report.v1`
- `normalized-config.v1`
- `credential-report.v1`
- `ingress-report.v1`
- `error-report.v1`

Phase 1A does not populate these files yet. It only freezes their shape.
