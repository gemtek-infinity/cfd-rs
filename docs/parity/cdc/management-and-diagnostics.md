# CDC Feature Group: Management And Diagnostics

## Scope

This document covers the management service HTTP routes, WebSocket log
streaming contract, diagnostics exposure, and management authentication
behavior as defined by the frozen Go baseline.

## Management Service Routes

Source: `baseline-2026.2.0/old-impl/management/service.go`

The management service is exposed via a chi router on the management tunnel
hostname (typically `management.argotunnel.com`).

### Route Inventory

| Route | Method | Auth | Condition | Purpose |
| --- | --- | --- | --- | --- |
| `/ping` | GET, HEAD | token query middleware + CORS | always | heartbeat/keepalive |
| `/logs` | GET → WebSocket | token query middleware | always | log streaming session |
| `/host_details` | GET | token query middleware + CORS | always | connector identity |
| `/metrics` | GET | token query middleware + CORS | `enableDiagServices=true` | Prometheus metrics |
| `/debug/pprof/{heap\|goroutine}` | GET | token query middleware + CORS | `enableDiagServices=true` | Go profiling |

### CORS Configuration

- allowed origins: `https://*.cloudflare.com`
- credentials: allowed
- max age: 300 seconds

### Ping Behavior

Returns HTTP 200 with empty body. Supports both GET and HEAD.

### Host Details Response

Source: `baseline-2026.2.0/old-impl/management/service.go`

```json
{
  "connector_id": "<UUID>",
  "ip": "<private-IP>",
  "hostname": "custom:<label>" | "<system-hostname>" | "unknown"
}
```

Field semantics:

- `connector_id`: always present (connector UUID)
- `ip`: the service IP; may be omitted if derivation fails
- `hostname`: connector label if configured, else system hostname, else
  `"unknown"`

## Authentication Middleware

Source: `baseline-2026.2.0/old-impl/management/middleware.go`

### Token Validation

- requires `?access_token=<JWT>` query parameter
- parses JWT via `ParseToken(accessToken)`
- caches parsed claims in request context (`accessClaimsCtxKey`)
- applied to all management routes

### Error Response

Missing or invalid token returns:

```json
{
  "success": false,
  "errors": [{"code": 1001, "message": "missing access_token query parameter"}]
}
```

HTTP status: 400 (Bad Request)

### Management Token Resource Types

Source: `baseline-2026.2.0/old-impl/cfapi/client.go`

`GetManagementToken(tunnelID, resource)` supports three resource scopes:

| Resource | Meaning |
| --- | --- |
| `logs` | log streaming access |
| `admin` | administrative access |
| `host_details` | host details read access |

## WebSocket Log Streaming Contract

Source: `baseline-2026.2.0/old-impl/management/events.go` and `session.go`

### Client → Server Events

#### EventStartStreaming

```json
{
  "type": "start_streaming",
  "filters": {
    "events": ["cloudflared", "http", "tcp", "udp"],
    "level": "debug",
    "sampling": 0.5
  }
}
```

- `events`: filter by log event type (optional)
- `level`: minimum log level (optional; null = all)
- `sampling`: 0.0-1.0 sampling rate (optional; 1.0 = all)

#### EventStopStreaming

```json
{
  "type": "stop_streaming"
}
```

### Server → Client Events

#### EventLog

```json
{
  "type": "logs",
  "logs": [
    {
      "time": "<ISO8601>",
      "level": "info",
      "message": "request from 192.168.1.1",
      "event": "http",
      "fields": {}
    }
  ]
}
```

### Log Event Types

| Value | String | Meaning |
| --- | --- | --- |
| 0 | `cloudflared` | cloudflared operational events (default) |
| 1 | `http` | HTTP proxy events |
| 2 | `tcp` | TCP proxy events |
| 3 | `udp` | UDP proxy events |

### Log Levels

| Value | String |
| --- | --- |
| 0 | `debug` |
| 1 | `info` |
| 2 | `warn` |
| 3 | `error` |

### WebSocket Close Codes

| Code | Constant | Reason |
| --- | --- | --- |
| 4001 | `StatusInvalidCommand` | expected start streaming as first event |
| 4002 | `StatusSessionLimitExceeded` | limit exceeded for streaming sessions |
| 4003 | `StatusIdleLimitExceeded` | session was idle for too long |

### Session Behavior

Source: `baseline-2026.2.0/old-impl/management/session.go`

- per-client session with `active` flag
- buffered listener channel (window = 30 entries; drops when full)
- filters applied per session
- sampling via random sampler
- first message must be `start_streaming` or connection is closed with 4001

## Diagnostics Exposure

Source: `baseline-2026.2.0/old-impl/management/service.go` and
`diagnostic/` package

### Conditional Routes

The `/metrics` and `/debug/pprof/...` routes are only registered when
`enableDiagServices=true`. This is typically controlled by the
`--management-diagnostics` flag (default: true).

### Pprof Endpoints

Only `heap` and `goroutine` profiles are exposed:

- `/debug/pprof/heap`
- `/debug/pprof/goroutine`

Other Go pprof endpoints are not registered.

## Current Rust Management Surface

**Status: entirely absent**

No management service, management routes, WebSocket log streaming, or
management authentication exist in the current Rust implementation.

## Gap Summary

| Gap | Severity | Detail |
| --- | --- | --- |
| management service absent | critical | no management HTTP server |
| management auth middleware absent | critical | no JWT token validation |
| `/ping` route absent | high | heartbeat not available |
| `/logs` WebSocket endpoint absent | critical | log streaming not available |
| `/host_details` route absent | high | connector identity not available |
| `/metrics` management route absent | medium | remote metrics not available (local metrics are HIS-owned) |
| `/debug/pprof` routes absent | medium | remote profiling not available |
| CORS configuration absent | medium | no cross-origin access |
| WebSocket event protocol absent | critical | start/stop streaming and log delivery not implemented |
| session management absent | high | no per-session state, buffering, sampling, or filtering |
| WebSocket close code contract absent | medium | custom close codes not defined |
| management token resource types absent | medium | no token resource scoping |
| diagnostics conditional exposure absent | medium | no `enableDiagServices` gating |
