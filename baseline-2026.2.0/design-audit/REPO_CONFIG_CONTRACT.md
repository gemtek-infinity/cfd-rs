# Cloudflared Configuration And Contract Appendix

This appendix expands the configuration, endpoint, and runtime-contract parts of the main reference into a more explicit schema and rule set.

## 1. Config Discovery And Filesystem Effects

### 1.1 Default Config Filenames

- `config.yml`
- `config.yaml`

### 1.2 Default Search Directories

Search order is:

1. `~/.cloudflared`
2. `~/.cloudflare-warp`
3. `~/cloudflare-warp`
4. `/etc/cloudflared` on non-Windows
5. `/usr/local/etc/cloudflared` on non-Windows

### 1.3 Default Primary Paths

- default config directory on non-Windows: `/usr/local/etc/cloudflared`
- default log directory on non-Windows: `/var/log/cloudflared`
- Windows config directory uses `CFDPATH` or `ProgramFiles(x86)\cloudflared` when present

### 1.4 Auto-Create Behavior

`FindOrCreateConfigPath()` will:

1. return the first existing config path if one exists
2. otherwise create the default config directory
3. create a new config file at the default config path
4. create log directory if possible
5. write a minimal YAML config containing `logDirectory`

This is a real side effect and should be treated as contract-relevant for service mode and onboarding flows.

## 2. Config File Loader Rules

### 2.1 Caching And Re-Read

- The loader caches the parsed config in a package-global variable.
- If the requested `config` path changes, it re-reads the file.
- If no config file is found and the user did not explicitly set `--config`, the loader returns `ErrNoConfigFile`.

### 2.2 Warning Path For Unknown Keys

- The file is decoded once normally.
- Then decoded again with YAML `KnownFields(true)`.
- Unknown keys are surfaced as warnings rather than hard parse failures in that second pass.

Implication:

- config typos can remain non-fatal while still being detectable
- validation tooling should preserve this distinction

## 3. Top-Level YAML Schema

Current repository-visible top-level YAML structure:

```yaml
tunnel: <uuid>
ingress:
  - hostname: example.com
    path: /regex
    service: https://localhost:8443
    originRequest: {}
warp-routing:
  connectTimeout: 5s
  maxActiveFlows: 1000
  tcpKeepAlive: 30s
originRequest:
  connectTimeout: 30s
logDirectory: /var/log/cloudflared
```

Top-level keys with current repository meaning:

| Key | Type | Meaning |
| --- | --- | --- |
| `tunnel` | string | named-tunnel UUID reference |
| `ingress` | list | ordered ingress rules |
| `warp-routing` | object | private-routing tuning |
| `originRequest` | object | defaults applied to ingress rule origin behavior |
| `logDirectory` | string | root-level setting written by auto-created config |

## 4. Ingress Schema

### 4.1 Rule Object

| Key | Type | Meaning |
| --- | --- | --- |
| `hostname` | string | exact hostname or wildcard subdomain matcher |
| `path` | string | regex-like path matcher |
| `service` | string | origin target or synthetic service selector |
| `originRequest` | object | per-rule override of origin-request defaults |

### 4.2 Ingress Invariants

- At least one rule is required for explicit ingress mode.
- The last rule must be catch-all, meaning no hostname or path filter.
- Hostname patterns may use at most one wildcard and only for subdomain position.
- Hostname cannot contain a port.
- Internal rules are checked before user rules.
- User rules are matched in order.
- The last validated rule is assumed to match everything.

### 4.3 Service Encodings

Repository-visible service encodings include:

- `http://...`
- `https://...`
- `unix:/path`
- `unix+tls:/path`
- `http_status:<code>`
- bastion service forms
- warp-routing/private-routing reserved services

### 4.4 CLI-Origin Compatibility Rule

When the user relies on `--url`, `--unix-socket`, `--hello-world`, or `--bastion`, cloudflared synthesizes a single ingress rule rather than requiring an explicit `ingress` block.

If neither explicit ingress nor CLI-origin inputs exist, the runtime uses a synthetic default origin that returns HTTP 503.

## 5. `originRequest` Schema

### 5.1 Top-Level `originRequest`

These keys exist both as global defaults and per-rule overrides.

| Key | Type | Meaning |
| --- | --- | --- |
| `connectTimeout` | duration | timeout for establishing origin connection |
| `tlsTimeout` | duration | timeout for TLS handshake to origin |
| `tcpKeepAlive` | duration | TCP keepalive to origin |
| `noHappyEyeballs` | bool | disable IPv4/v6 fallback heuristic |
| `keepAliveConnections` | int | max idle keepalive connections |
| `keepAliveTimeout` | duration | idle connection close timeout |
| `httpHostHeader` | string | override HTTP Host header |
| `originServerName` | string | override TLS server name |
| `matchSNIToHost` | bool | derive SNI from target host |
| `caPool` | string | CA bundle for origin TLS validation |
| `noTLSVerify` | bool | disable origin TLS verification |
| `disableChunkedEncoding` | bool | disable chunked transfer encoding |
| `bastionMode` | bool | run as bastion/jump host mode |
| `proxyAddress` | string | local proxy listen address |
| `proxyPort` | uint | local proxy listen port |
| `proxyType` | string | proxy mode, usually `socks` or empty |
| `ipRules` | list | IP allow/deny filtering rules |
| `http2Origin` | bool | use HTTP/2 to origin |
| `access` | object | Access validation requirements for origin traffic |

### 5.2 `access` Subobject

| Key | Type | Meaning |
| --- | --- | --- |
| `required` | bool | require Access-authenticated requests |
| `teamName` | string | Access team/org identifier |
| `audTag` | list of string | accepted audience values |
| `environment` | string | environment label |

Validation rule:

- If `required` is true and `audTag` is set, `teamName` cannot be blank.

### 5.3 `ipRules` Subobject

| Key | Type | Meaning |
| --- | --- | --- |
| `prefix` | string | CIDR-like prefix |
| `ports` | list of int | target ports |
| `allow` | bool | allow or deny |

## 6. `warp-routing` Schema

| Key | Type | Meaning |
| --- | --- | --- |
| `connectTimeout` | duration | private-routing connection timeout |
| `maxActiveFlows` | uint64 | limit concurrent active private flows |
| `tcpKeepAlive` | duration | keepalive for private-routing TCP |

Historical note:

- older `enabled` flag is no longer supported for local config paths

## 7. Custom Duration Contract

`CustomDuration` exists because JSON and YAML semantics differ.

- YAML accepts normal Go duration forms such as `3s`, `24h`, `5m`.
- JSON marshal emits seconds.
- JSON unmarshal expects integer-like seconds.

Breakage risk:

- changing this would silently alter config-manager and diagnostics serialization semantics

## 8. Credentials And Certificate Contract

### 8.1 Tunnel Credentials JSON

Repository-visible fields used in tunnel credentials and tokens:

- `AccountTag`
- `TunnelSecret`
- `TunnelID`
- `Endpoint` optional

Semantics:

- credentials file is created with restrictive mode `0400`
- existing file path is rejected rather than overwritten

### 8.2 Origin Certificate File

Expected default file name:

- `cert.pem`

Lookup rule:

- first `cert.pem` found in default config search directories

Repository-visible encoded content:

- PEM blocks
- active token block type `ARGO TUNNEL TOKEN`
- JSON payload containing `zoneID`, `accountID`, `apiToken`, optional `endpoint`

Decode rules:

- legacy `PRIVATE KEY` and `CERTIFICATE` blocks are tolerated for compatibility scanning
- multiple token blocks are rejected
- missing `ZoneID` or `APIToken` is rejected

## 9. Endpoint Contract Appendix

### 9.1 Local Metrics Server Endpoints

| Endpoint | Method | Condition | Meaning |
| --- | --- | --- | --- |
| `/metrics` | GET | always on metrics server | Prometheus metrics |
| `/healthcheck` | GET | always | returns `OK` |
| `/ready` | GET | if ready server configured | readiness JSON and HTTP 200/503 |
| `/quicktunnel` | GET | always | quick tunnel hostname JSON |
| `/config` | GET | if orchestrator configured | versioned config JSON |
| `/debug/` | GET | always mounted via default mux | pprof and trace-related debug endpoints |

### 9.2 Readiness Response Shape

```json
{
  "status": 200,
  "readyConnections": 1,
  "connectorId": "uuid"
}
```

Semantics:

- 200 only when `readyConnections > 0`
- 503 otherwise

### 9.3 Management Service Endpoints

| Endpoint | Methods | Condition | Meaning |
| --- | --- | --- | --- |
| `/ping` | GET, HEAD | always | management liveness |
| `/logs` | GET | always | WebSocket log stream |
| `/host_details` | GET | always | connector ID, preferred private IP, hostname/label |
| `/metrics` | GET | diagnostics enabled | remote metrics via management service |
| `/debug/pprof/{heap\|goroutine}` | GET | diagnostics enabled | remote pprof snapshots |

Access control:

- token query middleware gates the management service

### 9.4 Tail Filtering Contract

Valid filter values enforced by code:

- levels: `debug`, `info`, `warn`, `error`
- events: `cloudflared`, `http`, `tcp`, `udp`
- sampling: `(0.0, 1.0]`

## 10. Metrics Bind-Address Contract

### 10.1 Default Addresses

- host runtime default address: `localhost:0`
- virtual runtime default address: `0.0.0.0:0`

### 10.2 Known Address Probe Order

Host runtime probe order:

- `localhost:20241`
- `localhost:20242`
- `localhost:20243`
- `localhost:20244`
- `localhost:20245`

Virtual runtime probe order:

- `0.0.0.0:20241`
- `0.0.0.0:20242`
- `0.0.0.0:20243`
- `0.0.0.0:20244`
- `0.0.0.0:20245`

Selection rule:

1. if explicit metrics address given, bind it directly
2. if default address used, try known addresses in order
3. if all fail, bind random port via default address

## 11. Protocol And PQ Contract Appendix

### 11.1 Valid Protocol Selector Values

- `auto`
- `quic`
- `http2`

### 11.2 PQ Rule

- Post-quantum strict mode forces QUIC.
- If user explicitly selects non-QUIC transport while PQ is enforced, startup fails.

### 11.3 Edge Address Rule

- `--region` and credential `Endpoint` are mutually exclusive sources for resolved region selection.

### 11.4 Bind-Address Rule

- invalid edge bind address is fatal
- edge IP version may be overridden to match the bind address, with warning rather than fatal error in some paths

## 12. IP Rules Schema Detail

### 12.1 `ipRules` Entry Structure

```yaml
ipRules:
  - prefix: "192.168.1.0/24"
    ports: [80, 443]
    allow: true
  - prefix: "10.0.0.0/8"
    ports: []
    allow: false
```

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `prefix` | string | yes | IPv4 or IPv6 CIDR block |
| `ports` | list of int | no | Port list (1-65535); empty = all ports |
| `allow` | bool | yes | true = allow, false = deny |

Evaluation order: first rule whose CIDR contains the IP and whose port list matches (or is empty) wins. If no rule matches, the default policy applies.

Port validation: ports are sorted during construction and checked via binary search at runtime.

### 12.2 Bastion Mode Config

```yaml
originRequest:
  bastionMode: true
```

When `bastionMode` is true, the ingress rule acts as a jump host. The actual destination is specified per-request via the `Cf-Access-Jump-Destination` header from the client.

Bastion mode can also be activated by setting the service to `bastion` in the ingress rule.

### 12.3 Proxy Config Fields

```yaml
originRequest:
  proxyAddress: "127.0.0.1"
  proxyPort: 1080
  proxyType: "socks"
```

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `proxyAddress` | string | none | Local proxy listen address |
| `proxyPort` | uint | none | Local proxy listen port |
| `proxyType` | string | empty | Proxy mode; `"socks"` enables SOCKS5 handler; empty uses default bidirectional stream |

### 12.4 Credentials Endpoint Field

The tunnel credentials JSON file may contain an optional `Endpoint` field:

```json
{
  "AccountTag": "...",
  "TunnelSecret": "...",
  "TunnelID": "...",
  "Endpoint": "region1.argotunnel.com:7844"
}
```

When present, this overrides the default edge endpoint for this tunnel. Mutually exclusive with `--region` flag.

## 13. Contract Breakage Questions For Config Work

Any change should be reviewed with these prompts:

- Does it alter config file discovery or creation side effects?
- Does it rename or remove any YAML key?
- Does it change `originRequest` interpretation in single-origin CLI mode?
- Does it change no-ingress behavior or catch-all validation?
- Does it change JSON duration semantics?
- Does it change IP rules evaluation order or default policy?
- Does it change bastion mode activation conditions?
- Does it change proxy type handling or SOCKS stream handler selection?
- Does it change credentials endpoint field semantics?
- Does it change cert/token lookup order?
- Does it change endpoint exposure conditions or response schema?
