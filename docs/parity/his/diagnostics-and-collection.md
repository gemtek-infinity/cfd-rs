# HIS Feature-Group Audit: Diagnostics and Collection

## Purpose

This document audits the local diagnostics collection surface, local HTTP
endpoint exposure, and metrics/readiness behavior against the frozen Go
baseline in `baseline-2026.2.0/old-impl/`.

These are host-facing because they expose data on local network interfaces,
collect host system information, and interact with local filesystem and
process state.

## Frozen Baseline Source

Primary files:

- `diagnostic/` package — collectors, handlers, CLI command
- `metrics/readiness.go` — readiness endpoint
- `metrics/metrics.go` — metrics server lifecycle and route registration
- `cmd/cloudflared/tunnel/cmd.go` — metrics server setup
- `tunnelstate/conntracker.go` — connection state tracking

## Local HTTP Server

### Binding

- default bind: `localhost:0` (host runtime) or `0.0.0.0:0` (container)
- known port fallback: tries ports 20241–20245 sequentially before random
- flag: `--metrics ADDRESS`
- timeouts: ReadTimeout=10s, WriteTimeout=10s
- started after 500ms delay for startup ordering

### Route Inventory

| Route | Handler | Response | Purpose |
| --- | --- | --- | --- |
| `/metrics` | `promhttp.Handler()` | Prometheus text | process and custom metrics |
| `/healthcheck` | inline | `OK\n` (text/plain) | liveness probe |
| `/ready` | `ReadyServer.ServeHTTP()` | JSON, HTTP 200/503 | readiness probe |
| `/quicktunnel` | inline | `{"hostname":"..."}` | quick tunnel URL |
| `/config` | orchestrator | versioned ingress JSON | current tunnel config |
| `/diag/system` | `SystemHandler` | JSON | host system information |
| `/diag/tunnel` | `TunnelStateHandler` | JSON | tunnel connection state |
| `/diag/configuration` | `ConfigurationHandler` | JSON | CLI flags including UID |
| `/debug/pprof/*` | `http.DefaultServeMux` | binary pprof | CPU, memory, goroutines |

No authentication on any local endpoint — security relies on localhost bind
and privileged port.

### Server Lifecycle

Created in `StartServer()`, runs concurrently with tunnel via goroutine,
shutdown via context cancellation with 15s timeout.

## Readiness Endpoint (`/ready`)

### Response Shape

```json
{
  "status": 200,
  "readyConnections": 2,
  "connectorId": "550e8400-e29b-41d4-a716-446655440000"
}
```

### Semantics

- HTTP 200 when `readyConnections > 0`
- HTTP 503 when no active connections
- `status` field in body matches HTTP status code
- `connectorId` is the tunnel connector UUID
- used by Kubernetes readiness probes

## Diagnostics Collectors

### SystemCollector

Endpoint: `/diag/system`

Response:

```json
{
  "info": {
    "memoryMaximum": 16384,
    "memoryCurrent": 8192,
    "fileDescriptorMaximum": 1024,
    "fileDescriptorCurrent": 42,
    "osSystem": "linux",
    "osVersion": "...",
    "osRelease": "...",
    "architecture": "amd64",
    "diskVolumes": [
      {
        "name": "/",
        "sizeMaximum": 500000,
        "sizeCurrent": 250000
      }
    ]
  },
  "err": null
}
```

Platform-specific: `system_collector_linux.go`, `system_collector_macos.go`,
`system_collector_windows.go`.

### TunnelStateCollector

Endpoint: `/diag/tunnel`

Response:

```json
{
  "tunnelID": "<uuid>",
  "connectorID": "<uuid>",
  "connections": [
    {
      "index": 0,
      "isActive": true,
      "protocol": "quic",
      "origin": "...",
      "edge": "..."
    }
  ],
  "icmpSources": ["..."]
}
```

### ConfigurationHandler

Endpoint: `/diag/configuration`

Response: `map[string]string` with keys including `uid` (from `os.Getuid()`),
`log_file`, `log_directory`. Secret flags are excluded.

### MetricsCollector

Endpoint: `/metrics`

Prometheus text format with `build_info` gauge (labels: goversion, type,
revision, version) and all `prometheus.DefaultRegisterer` metrics.

### LogCollector

Three implementations:

- **HostLogCollector**: reads from file/directory or journalctl
  - if UID==0 and systemd service file exists on Linux: `journalctl -u
    cloudflared.service --since "2 weeks ago"`
  - otherwise: user-provided log file/directory path
  - fallback paths: `/var/log/cloudflared.err` (Linux),
    `/Library/Logs/com.cloudflare.cloudflared.err.log` (macOS)
- **DockerLogCollector**: tail container logs
- **KubernetesLogCollector**: extract pod logs

### NetworkCollector

Traceroute to Cloudflare regions:

- targets: `region1.v2.argotunnel.com` and `region2.v2.argotunnel.com`
  (IPv4 and IPv6)
- configurable hops (default 5) and timeout (default 5s)
- returns hop array with RTT measurements

## Diagnostic CLI Command

```
cloudflared tunnel diag [options]
```

### Flags

| Flag | Purpose |
| --- | --- |
| `--metrics ADDRESS` | target specific instance |
| `--diag-container-id CONTAINER` | extract from Docker container |
| `--diag-pod-id POD` | extract from Kubernetes pod |
| `--no-diag-logs` | skip log collection |
| `--no-diag-metrics` | skip metrics collection |
| `--no-diag-system` | skip system info collection |
| `--no-diag-runtime` | skip pprof collection |
| `--no-diag-network` | skip traceroute collection |

### Diagnostic Bundle (ZIP output)

11 jobs producing these artifacts:

| File | Content | Toggleable |
| --- | --- | --- |
| `systeminformation.json` | system collector output | `--no-diag-system` |
| `metrics.txt` | Prometheus text | `--no-diag-metrics` |
| `tunnelstate.json` | connection state | always |
| `cli-configuration.json` | CLI flags + UID | always |
| `configuration.json` | versioned tunnel config | always |
| `heap.pprof` | memory profile | `--no-diag-runtime` |
| `goroutine.pprof` | goroutine dump | `--no-diag-runtime` |
| `network.json` | traceroute JSON | `--no-diag-network` |
| `raw-network.txt` | raw traceroute output | `--no-diag-network` |
| `cloudflared_logs.txt` | collected logs | `--no-diag-logs` |
| `task-result.json` | per-job success/failure | auto-generated |

### Instance Discovery

The diagnostic client auto-discovers running instances by trying known ports
20241–20245. Errors:

- `ErrMetricsServerNotFound` — no running instance found
- `ErrMultipleMetricsServerFound` — multiple instances, lists them
- `ErrLogConfigurationIsInvalid` — log config unavailable

### Key Constants

| Constant | Value |
| --- | --- |
| `defaultTimeout` | 15 seconds |
| `defaultCollectorTimeout` | 10 seconds |
| `twoWeeksOffset` | -14 days |
| `tailMaxNumberOfLines` | 10000 |

## Local Management HTTP Service (Host-Facing Aspects)

The management service is exposed via tunnel ingress rules, NOT on a local
listener. However, it serves host-local information:

### Host Details Endpoint

`GET /host_details` returns:

```json
{
  "connector_id": "...",
  "ip": "192.168.1.42",
  "hostname": "custom:my-label"
}
```

- `ip`: determined by dialing the edge service IP and reading local address
- `hostname`: custom label (via `--connector-label`) or `os.Hostname()`

### Diagnostics Flag

`--management-diagnostics` enables `/metrics` and `/debug/pprof/*` on the
management route (in addition to the local metrics server).

## ICMP Proxy

### Baseline Behavior

**Source:** `ingress/icmp_linux.go`, `cmd/cloudflared/tunnel/configuration.go`

The ICMP proxy creates raw sockets for proxied ICMP echo requests through
the tunnel:

- opens raw sockets via `net.ListenPacket()` for ICMP and ICMPv6
- checks `/proc/sys/net/ipv4/ping_group_range` at startup to verify process
  GID is within the permitted range
- logs warning if ping group check fails; silently disables ICMP proxy
  (does not error out)
- source IP configurable via `--icmpv4-src` and `--icmpv6-src` flags
  (env: `TUNNEL_ICMPV4_SRC`, `TUNNEL_ICMPV6_SRC`)
- auto-detects source IP by dialing `192.168.0.1:53` to read local address
  if flags not specified
- requires `CAP_NET_RAW` capability or GID within ping_group_range
- each (src_ip, dst_ip, echo_id) tuple maps to a kernel-managed port
- quick tunnels explicitly disable ICMP routing

### Rust State

Not implemented. No raw ICMP socket handling or ping group privilege check.

## Hello World Test Server

### Baseline Behavior

**Source:** `hello/hello.go`, `ingress/origin_service.go`

The `hello_world` ingress service starts a localhost TLS listener for
connectivity verification:

- binds to `127.0.0.1:0` (auto-assigned port)
- uses self-signed TLS certificate from `tlsconfig.GetHelloCertificate()`
- serves routes: `/` (test page), `/uptime`, `/ws` (WebSocket), `/sse`
  (Server-Sent Events), `/_health`
- activated via ingress rule `service: hello_world` or `--hello-world` flag
- stops on `shutdownC` signal via `httpServer.Close()`

### Rust State

Config parsing recognizes `hello_world` as `IngressService::HelloWorld` and
`--hello-world` flag is parsed. The actual listener, TLS cert generation,
and route handlers are not implemented (deferred to Phase 5.1).

## Current Rust State

### What exists

- readiness state machine tracking lifecycle and subsystem gates
  (`crates/cloudflared-cli/src/runtime/state/readiness.rs`)
- operability reporting with status and metrics to stdout
  (`crates/cloudflared-cli/src/runtime/state/operability.rs`)
- deployment evidence including systemd detection and known gaps
  (`crates/cloudflared-cli/src/runtime/state/deployment_evidence.rs`)
- failure evidence with restart budget tracking
  (`crates/cloudflared-cli/src/runtime/state/failure.rs`)
- performance timing milestones
  (`crates/cloudflared-cli/src/runtime/state/timing.rs`)

### What is missing

- local HTTP server (no metrics listener)
- `/metrics` Prometheus endpoint
- `/healthcheck` liveness endpoint
- `/ready` readiness HTTP endpoint (state machine exists but not HTTP-exposed)
- `/quicktunnel` endpoint
- `/config` endpoint
- all `/diag/*` diagnostic endpoints
- `/debug/pprof/*` profiling endpoints
- `tunnel diag` CLI command
- diagnostic bundle ZIP generation
- system information collection (memory, fd, disk)
- tunnel state collection via HTTP
- network traceroute diagnostics
- log collection from journalctl or files
- instance auto-discovery via known ports
- host details endpoint
- ICMP proxy raw socket handling
- ping group privilege check
- ICMP source IP auto-detection
- `hello_world` listener and route handlers

## Lane Classification

| Surface | Lane-required | Notes |
| --- | --- | --- |
| local HTTP server with metrics binding | yes | operator observability |
| `/ready` endpoint with JSON shape | yes | Kubernetes integration |
| `/healthcheck` endpoint | yes | liveness probe |
| `/metrics` Prometheus endpoint | yes | monitoring integration |
| system info collector (Linux) | yes | diagnostics |
| tunnel state collector | yes | diagnostics |
| `tunnel diag` CLI command | yes | operator diagnostics |
| host log collection | yes | diagnostics |
| ICMP proxy raw socket | yes | proxied ICMP echo through tunnel |
| ping group privilege check | yes | host capability gate |
| ICMP source IP flags | medium | ICMP proxy configuration |
| `hello_world` ingress listener | medium | test/verification service |
| network traceroute | medium | useful but not critical path |
| `/quicktunnel` endpoint | medium | quick tunnel flow |
| `/config` endpoint | medium | remote config visibility |
| pprof profiling | low | debugging aid |
| Docker/K8s log collectors | low | container diagnostics |
| host details endpoint | medium | management service dependency |

## Gap Summary

| Gap | Severity | Notes |
| --- | --- | --- |
| local HTTP metrics server absent | critical | no observability surface |
| `/ready` endpoint absent | critical | blocks Kubernetes readiness |
| `/healthcheck` endpoint absent | high | blocks liveness probes |
| `/metrics` Prometheus endpoint absent | critical | blocks monitoring |
| `tunnel diag` command absent | high | blocks operator diagnostics |
| system info collector absent | high | diagnostic bundle dependency |
| tunnel state HTTP collector absent | high | diagnostic bundle dependency |
| diagnostic bundle ZIP generation absent | high | `tunnel diag` output |
| ICMP proxy raw socket absent | high | proxied ICMP echo |
| ping group privilege check absent | high | host capability gate |
| `hello_world` listener/handler absent | medium | config parsed, handler deferred |
| instance auto-discovery absent | medium | diagnostic client dependency |
| log collection from journalctl absent | medium | host log diagnostics |
| network traceroute absent | medium | network diagnostics |
| ICMP source IP flags absent | medium | ICMP configuration |
| `/config` endpoint absent | medium | remote config visibility |
| `/quicktunnel` endpoint absent | medium | quick tunnel flow |
| pprof endpoints absent | low | debugging aid |
