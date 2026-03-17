# CDC Feature Group: Metrics, Readiness, And API Contracts

## Scope

This document covers three CDC surfaces:

1. the externally visible readiness endpoint contract
2. the metrics endpoint contract (Prometheus scrape surface)
3. the Cloudflare REST API client contracts used by CLI command flows

The readiness and metrics endpoints are exposed on the local metrics HTTP
server. While the server binding and lifecycle are HIS-owned, the response
contracts are CDC-owned because they are consumed by external systems
(Kubernetes probes, Prometheus scrapers, Cloudflare management).

## Readiness Contract

Source: [baseline-2026.2.0/metrics/readiness.go](../../../baseline-2026.2.0/metrics/readiness.go)

### Endpoint

`GET /ready` on the metrics HTTP server.

### Response Shape

```json
{
  "status": 200,
  "readyConnections": 1,
  "connectorId": "uuid-string"
}
```

### Semantics

- HTTP 200 if `tracker.CountActiveConns() > 0` (at least one active edge
  connection)
- HTTP 503 if no active connections
- `status` field mirrors the HTTP status code
- `readyConnections` is the count of active edge connections
- `connectorId` is the connector UUID

### Usage

Intended for Kubernetes readiness probes and external health-check systems.

## Metrics Contract

Source: [baseline-2026.2.0/metrics/metrics.go](../../../baseline-2026.2.0/metrics/metrics.go)

### Endpoint

`GET /metrics` on the metrics HTTP server.

Served by `promhttp.Handler()` from `prometheus/client_golang`.

### Known Exported Metrics

| Metric Name | Type | Labels | Source |
| --- | --- | --- | --- |
| `cloudflared_build_info` | gauge | version, revision, go_version | main |
| `capnp_server_operations_total` | counter | operation, lane | tunnelrpc/metrics |
| `capnp_server_operation_errors_total` | counter | operation, lane | tunnelrpc/metrics |
| `capnp_client_operations_total` | counter | operation, lane | tunnelrpc/metrics |
| `capnp_client_operation_errors_total` | counter | operation, lane | tunnelrpc/metrics |

Additional metrics are registered by various subsystems (tunnel state,
transport, proxy) but the above are the explicitly named ones in the CDC
surface.

### Other Local HTTP Endpoints

| Endpoint | Response | Purpose |
| --- | --- | --- |
| `/healthcheck` | text `OK\n` | simple liveness probe |
| `/quicktunnel` | `{"hostname":"<hostname>"}` | quick tunnel public URL |
| `/config` | versioned config JSON or 500 error | current tunnel config |
| `/debug/...` | Go pprof handlers | profiling (CPU, memory, goroutines) |

## Cloudflare REST API Client Contracts

Source: [baseline-2026.2.0/cfapi/](../../../baseline-2026.2.0/cfapi/)

### Client Interface

The `cfapi.Client` interface composes four sub-interfaces:

- `TunnelClient`
- `HostnameClient`
- `IPRouteClient`
- `VnetClient`

### TunnelClient Methods

| Method | HTTP | Endpoint | Purpose |
| --- | --- | --- | --- |
| `CreateTunnel(name, secret)` | POST | `/accounts/{accountTag}/cfd_tunnel` | create named tunnel |
| `GetTunnel(tunnelID)` | GET | `/accounts/{accountTag}/cfd_tunnel/{tunnelID}` | fetch tunnel details |
| `GetTunnelToken(tunnelID)` | GET | `/accounts/{accountTag}/cfd_tunnel/{tunnelID}/token` | get tunnel secret |
| `GetManagementToken(tunnelID, resource)` | GET | `/accounts/{accountTag}/cfd_tunnel/{tunnelID}/management` | get management JWT |
| `ListTunnels(filter)` | GET | `/accounts/{accountTag}/cfd_tunnel?...` | list tunnels |
| `ListActiveClients(tunnelID)` | GET | `/accounts/{accountTag}/cfd_tunnel/{tunnelID}/connections` | connected instances |
| `DeleteTunnel(tunnelID, cascade)` | DELETE | `/accounts/{accountTag}/cfd_tunnel/{tunnelID}` | delete tunnel |
| `CleanupConnections(tunnelID, params)` | DELETE | `/accounts/{accountTag}/cfd_tunnel/{tunnelID}/connections?...` | force-kill connections |

### HostnameClient Methods

| Method | HTTP | Endpoint | Purpose |
| --- | --- | --- | --- |
| `RouteTunnel(tunnelID, route)` | PUT | `/zones/{zoneTag}/tunnels/{tunnelID}/routes` | hostname→tunnel mapping |

### IPRouteClient Methods

| Method | HTTP | Endpoint | Purpose |
| --- | --- | --- | --- |
| `ListRoutes(filter)` | GET | `/accounts/{accountTag}/teamnet/routes?...` | list IP routes |
| `AddRoute(newRoute)` | POST | `/accounts/{accountTag}/teamnet/routes` | create CIDR→tunnel |
| `DeleteRoute(id)` | DELETE | `/accounts/{accountTag}/teamnet/routes/{routeID}` | remove route |
| `GetByIP(params)` | GET | `/accounts/{accountTag}/teamnet/routes?ip=...` | lookup by IP |

### VnetClient Methods

| Method | HTTP | Endpoint | Purpose |
| --- | --- | --- | --- |
| `CreateVirtualNetwork(newVnet)` | POST | `/accounts/{accountTag}/teamnet/virtual_networks` | create vnet |
| `ListVirtualNetworks(filter)` | GET | `/accounts/{accountTag}/teamnet/virtual_networks?...` | list vnets |
| `DeleteVirtualNetwork(id, force)` | DELETE | `/accounts/{accountTag}/teamnet/virtual_networks/{vnetID}` | remove vnet |
| `UpdateVirtualNetwork(id, updates)` | PATCH | `/accounts/{accountTag}/teamnet/virtual_networks/{vnetID}` | modify vnet |

### HTTP Request Contract

Every API request includes:

- `User-Agent` header
- `Authorization: Bearer <token>`
- `Accept: application/json;version=1`
- `Content-Type: application/json` (when body present)
- timeout: 15 seconds
- transport: HTTP/2 enabled

### Response Envelope Contract

```json
{
  "success": true,
  "errors": [],
  "messages": [],
  "result": "...",
  "result_info": {
    "count": 1,
    "page": 1,
    "per_page": 20,
    "total_count": 1
  }
}
```

### Status Code → Error Mapping

| Status | Error |
| --- | --- |
| 200 | success |
| 400 | `ErrBadRequest` |
| 401 / 403 | `ErrUnauthorized` |
| 404 | `ErrNotFound` |
| other | formatted API failure |

### Tunnel Filter Query Parameters

From `TunnelFilter`:

- `name`, `name_prefix`, `exclude_prefix`
- `is_deleted=false` (default)
- `existed_at`
- `uuid`
- `per_page`, `page`

### IP Route Filter Query Parameters

From `IpRouteFilter`:

- `tun_types=cfd_tunnel`
- `is_deleted`, `network_subset`, `network_superset`
- `comment`, `tunnel_id`, `virtual_network_id`
- `per_page`, `page`

### Virtual Network Filter Query Parameters

From `VnetFilter`:

- `id`, `name`, `is_default`, `is_deleted`
- `per_page`

## Cleanup Parameters Contract

`CleanupConnections(tunnelID, params)` accepts `CleanupParams`:

- `ForClient(clientID)` encodes `connector_id` query parameter
- without `ForClient()`, all stale connections are cleaned

## Current Rust API And Metrics Surface

### Readiness

**Status: parity-backed.** `cfdrs-bin` serves `/ready` with the baseline JSON
shape, connector UUID, and HTTP 200/503 semantics derived from the admitted
runtime connection tracker.

### Metrics

**Status: parity-backed for the admitted local HTTP contract.**

- `/metrics` serves Prometheus text from the runtime registry.
- `/healthcheck` returns exact Go text `OK\n`.
- `/quicktunnel` returns `{"hostname":"..."}` JSON.
- `/config` returns the versioned runtime config snapshot.

### Cloudflare REST API Client

**Status: parity-backed for admitted CLI flows.** `cfdrs-bin::api_client`
implements the tunnel CRUD, route, virtual-network, cleanup, list-filter, and
management-token request surfaces used by the closed CLI rows.

## Gap Summary

| Gap | Severity | Detail |
| --- | --- | --- |
| no open readiness or local metrics contract gaps in this feature group | low | remaining Proof Closure API work is tracked in other rows (for example CDC-039) |
