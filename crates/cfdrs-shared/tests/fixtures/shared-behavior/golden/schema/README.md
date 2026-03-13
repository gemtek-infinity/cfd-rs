# Shared-Behavior Golden Schema

All shared-behavior parity artifacts use a checked-in canonical JSON envelope.

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

- `schema_version`: envelope version
- `fixture_id`: must match `fixture-index.toml`
- `producer`: `go-truth` or `rust-actual`
- `report_kind`: accepted shared-behavior report kind
- `comparison`: copied from `fixture-index.toml`
- `source_refs`: frozen Go source or test references
- `payload`: canonical JSON object or array emitted by the relevant runner

Accepted report kinds:

- `discovery-report.v1`
- `normalized-config.v1`
- `credential-report.v1`
- `ingress-report.v1`
- `error-report.v1`
