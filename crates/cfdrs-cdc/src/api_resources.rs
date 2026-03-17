//! Cloudflare REST API resource types (CDC-033, CDC-036, CDC-037, CDC-038,
//! CDC-039).
//!
//! These are the typed domain objects returned by the Cloudflare REST API.
//! The request/response envelope lives in [`super::api`].
//!
//! Matches `baseline-2026.2.0/cfapi/tunnel.go`, `ip_route.go`,
//! `virtual_network.go`, `hostname.go`, and `client.go`.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Tunnel CRUD (CDC-033)
// ---------------------------------------------------------------------------

/// A Cloudflare Tunnel resource.
///
/// Matches `Tunnel` in `cfapi/tunnel.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct Tunnel {
    pub id: Uuid,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub created_at: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub deleted_at: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub connections: Vec<TunnelConnection>,
}

/// A Tunnel with its token, returned from create.
///
/// Matches `TunnelWithToken` in `cfapi/tunnel.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct TunnelWithToken {
    #[serde(flatten)]
    pub tunnel: Tunnel,
    pub token: String,
}

/// A tunnel connection entry.
///
/// Matches `Connection` in `cfapi/tunnel.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct TunnelConnection {
    pub colo_name: String,
    pub id: Uuid,
    #[serde(default)]
    pub is_pending_reconnect: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub origin_ip: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub opened_at: String,
}

/// An active client (connector) entry.
///
/// Matches `ActiveClient` in `cfapi/tunnel.go`.
/// Note: Go uses `json:"conns"` not `json:"connections"`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct ActiveClient {
    pub id: Uuid,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub version: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub arch: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub run_at: String,
    /// Go serializes this as `"conns"`, not `"connections"`.
    #[serde(default, skip_serializing_if = "Vec::is_empty", rename = "conns")]
    pub connections: Vec<TunnelConnection>,
}

/// Request body for creating a new tunnel.
///
/// Matches the private `newTunnel` in `cfapi/tunnel.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct NewTunnel {
    pub name: String,
    /// Base64-encoded tunnel secret.
    pub tunnel_secret: String,
}

// ---------------------------------------------------------------------------
// Management resource (CDC-038)
// ---------------------------------------------------------------------------

/// Management resource scope for token requests.
///
/// Matches `ManagementResource` iota in `cfapi/tunnel.go`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ManagementResource {
    Logs = 0,
    Admin = 1,
    HostDetails = 2,
}

impl ManagementResource {
    /// Returns the URL path segment for this resource.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Logs => "logs",
            Self::Admin => "admin",
            Self::HostDetails => "host_details",
        }
    }
}

impl std::fmt::Display for ManagementResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// IP routes (CDC-036)
// ---------------------------------------------------------------------------

/// An IP route entry.
///
/// Matches `Route` in `cfapi/ip_route.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct Route {
    pub network: String,
    pub tunnel_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub virtual_network_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub comment: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub created_at: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub deleted_at: String,
}

/// A detailed route with tunnel name.
///
/// Matches `DetailedRoute` in `cfapi/ip_route.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct DetailedRoute {
    pub id: Uuid,
    pub network: String,
    pub tunnel_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub virtual_network_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub comment: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub created_at: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub deleted_at: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tunnel_name: String,
}

/// Request body for adding a new IP route.
///
/// Matches `NewRoute` in `cfapi/ip_route.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct NewRoute {
    pub network: String,
    pub tunnel_id: Uuid,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub comment: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub virtual_network_id: Option<Uuid>,
}

// ---------------------------------------------------------------------------
// Virtual networks (CDC-037)
// ---------------------------------------------------------------------------

/// A virtual network resource.
///
/// Matches `VirtualNetwork` in `cfapi/virtual_network.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct VirtualNetwork {
    pub id: Uuid,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub comment: String,
    #[serde(default, rename = "is_default_network")]
    pub is_default: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub created_at: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub deleted_at: String,
}

/// Request body for creating a new virtual network.
///
/// Matches `NewVirtualNetwork` in `cfapi/virtual_network.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct NewVirtualNetwork {
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub comment: String,
    #[serde(default, rename = "is_default_network")]
    pub is_default: bool,
}

/// Partial update for a virtual network.
///
/// Matches `UpdateVirtualNetwork` in `cfapi/virtual_network.go`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct UpdateVirtualNetwork {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "is_default_network"
    )]
    pub is_default: Option<bool>,
}

// ---------------------------------------------------------------------------
// Hostname routing (CDC-039)
// ---------------------------------------------------------------------------

/// DNS route request body.
///
/// Matches `DNSRoute.MarshalJSON()` in `cfapi/hostname.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct DnsRouteRequest {
    #[serde(rename = "type")]
    pub route_type: String,
    pub user_hostname: String,
    #[serde(default)]
    pub overwrite_existing: bool,
}

impl DnsRouteRequest {
    pub fn new(user_hostname: String, overwrite_existing: bool) -> Self {
        Self {
            route_type: "dns".to_string(),
            user_hostname,
            overwrite_existing,
        }
    }
}

/// Load-balancer route request body.
///
/// Matches `LBRoute.MarshalJSON()` in `cfapi/hostname.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct LbRouteRequest {
    #[serde(rename = "type")]
    pub route_type: String,
    pub lb_name: String,
    pub lb_pool: String,
}

impl LbRouteRequest {
    pub fn new(lb_name: String, lb_pool: String) -> Self {
        Self {
            route_type: "lb".to_string(),
            lb_name,
            lb_pool,
        }
    }
}

/// DNS route result.
///
/// Matches `DNSRouteResult` in `cfapi/hostname.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct DnsRouteResult {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cname: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
}

/// Load-balancer route result.
///
/// Matches `LBRouteResult` in `cfapi/hostname.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct LbRouteResult {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub load_balancer: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub pool: String,
}

// ---------------------------------------------------------------------------
// Hostname route envelope (CDC-039)
// ---------------------------------------------------------------------------

/// A hostname route request sent to the zone-level routing endpoint.
///
/// Matches the polymorphic `HostnameRoute` interface in `cfapi/hostname.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostnameRoute {
    Dns(DnsRouteRequest),
    Lb(LbRouteRequest),
}

impl Serialize for HostnameRoute {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Dns(req) => req.serialize(serializer),
            Self::Lb(req) => req.serialize(serializer),
        }
    }
}

/// Result from a hostname route operation.
///
/// Matches the polymorphic `HostnameRouteResult` interface in
/// `cfapi/hostname.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostnameRouteResult {
    Dns(DnsRouteResult),
    Lb(LbRouteResult),
}

impl HostnameRouteResult {
    /// One-line success summary matching Go's `SuccessSummary()`.
    pub fn success_summary(&self) -> String {
        match self {
            Self::Dns(r) => {
                format!("{} => {} (CNAME {})", r.name, r.cname, r.cname)
            }
            Self::Lb(r) => {
                format!("load_balancer: {}, pool: {}", r.load_balancer, r.pool)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Filter types (CDC-033, CDC-036, CDC-037)
// ---------------------------------------------------------------------------

/// Query parameters for listing tunnels.
///
/// Matches `TunnelFilter` builder methods in `cfapi/tunnel_filter.go`.
#[derive(Debug, Clone, Default)]
pub struct TunnelFilter {
    pub name: Option<String>,
    pub name_prefix: Option<String>,
    pub exclude_prefix: Option<String>,
    pub is_deleted: Option<bool>,
    pub existed_at: Option<String>,
    pub tunnel_id: Option<Uuid>,
    pub per_page: Option<u32>,
    pub page: Option<u32>,
}

impl TunnelFilter {
    pub fn by_name(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            is_deleted: Some(false),
            ..Default::default()
        }
    }

    pub fn to_query_pairs(&self) -> Vec<(&str, String)> {
        let mut pairs = Vec::new();

        if let Some(ref v) = self.name {
            pairs.push(("name", v.clone()));
        }

        if let Some(ref v) = self.name_prefix {
            pairs.push(("name_prefix", v.clone()));
        }

        if let Some(ref v) = self.exclude_prefix {
            pairs.push(("exclude_prefix", v.clone()));
        }

        if let Some(v) = self.is_deleted {
            pairs.push(("is_deleted", v.to_string()));
        }

        if let Some(ref v) = self.existed_at {
            pairs.push(("existed_at", v.clone()));
        }

        if let Some(v) = self.tunnel_id {
            pairs.push(("uuid", v.to_string()));
        }

        if let Some(v) = self.per_page {
            pairs.push(("per_page", v.to_string()));
        }

        if let Some(v) = self.page {
            pairs.push(("page", v.to_string()));
        }

        pairs
    }
}

/// Query parameters for listing IP routes.
///
/// Matches `IpRouteFilter` builder methods in `cfapi/ip_route_filter.go`.
#[derive(Debug, Clone, Default)]
pub struct IpRouteFilter {
    pub is_deleted: Option<bool>,
    pub network_subset: Option<String>,
    pub network_superset: Option<String>,
    pub existed_at: Option<String>,
    pub tunnel_id: Option<Uuid>,
    pub virtual_network_id: Option<Uuid>,
    pub comment: Option<String>,
    pub per_page: Option<u32>,
    pub page: Option<u32>,
}

impl IpRouteFilter {
    pub fn to_query_pairs(&self) -> Vec<(&str, String)> {
        let mut pairs = vec![("tun_types", "cfd_tunnel".to_string())];

        if let Some(v) = self.is_deleted {
            pairs.push(("is_deleted", v.to_string()));
        }

        if let Some(ref v) = self.network_subset {
            pairs.push(("network_subset", v.clone()));
        }

        if let Some(ref v) = self.network_superset {
            pairs.push(("network_superset", v.clone()));
        }

        if let Some(ref v) = self.existed_at {
            pairs.push(("existed_at", v.clone()));
        }

        if let Some(v) = self.tunnel_id {
            pairs.push(("tunnel_id", v.to_string()));
        }

        if let Some(v) = self.virtual_network_id {
            pairs.push(("virtual_network_id", v.to_string()));
        }

        if let Some(ref v) = self.comment {
            pairs.push(("comment", v.clone()));
        }

        if let Some(v) = self.per_page {
            pairs.push(("per_page", v.to_string()));
        }

        if let Some(v) = self.page {
            pairs.push(("page", v.to_string()));
        }

        pairs
    }
}

/// Query parameters for listing virtual networks.
///
/// Matches `VnetFilter` builder methods in
/// `cfapi/virtual_network_filter.go`.
#[derive(Debug, Clone, Default)]
pub struct VnetFilter {
    pub id: Option<Uuid>,
    pub name: Option<String>,
    pub is_default: Option<bool>,
    pub is_deleted: Option<bool>,
    pub per_page: Option<u32>,
}

impl VnetFilter {
    pub fn by_name(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            is_deleted: Some(false),
            ..Default::default()
        }
    }

    pub fn to_query_pairs(&self) -> Vec<(&str, String)> {
        let mut pairs = Vec::new();

        if let Some(v) = self.id {
            pairs.push(("id", v.to_string()));
        }

        if let Some(ref v) = self.name {
            pairs.push(("name", v.clone()));
        }

        if let Some(v) = self.is_default {
            pairs.push(("is_default", v.to_string()));
        }

        if let Some(v) = self.is_deleted {
            pairs.push(("is_deleted", v.to_string()));
        }

        if let Some(v) = self.per_page {
            pairs.push(("per_page", v.to_string()));
        }

        pairs
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Tunnel types (CDC-033) -------------------------------------------

    #[test]
    fn tunnel_json_keys_match_go() {
        let t = Tunnel {
            id: Uuid::nil(),
            name: "test-tun".to_string(),
            created_at: "2025-01-01T00:00:00Z".to_string(),
            deleted_at: String::new(),
            connections: vec![],
        };
        let json = serde_json::to_string(&t).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert!(v.get("id").is_some());
        assert!(v.get("name").is_some());
        assert!(v.get("created_at").is_some());
        // deleted_at empty → omitted
        assert!(v.get("deleted_at").is_none());
    }

    #[test]
    fn active_client_uses_conns_key() {
        let c = ActiveClient {
            id: Uuid::nil(),
            features: vec!["feat".to_string()],
            version: "2026.2.0".to_string(),
            arch: "linux_amd64".to_string(),
            run_at: "2025-01-01T00:00:00Z".to_string(),
            connections: vec![TunnelConnection {
                colo_name: "DFW".to_string(),
                id: Uuid::nil(),
                is_pending_reconnect: false,
                origin_ip: String::new(),
                opened_at: String::new(),
            }],
        };
        let json = serde_json::to_string(&c).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
        // Go uses "conns" not "connections"
        assert!(v.get("conns").is_some(), "expected 'conns' key");
        assert!(v.get("connections").is_none());
    }

    #[test]
    fn tunnel_with_token_flattens() {
        let twt = TunnelWithToken {
            tunnel: Tunnel {
                id: Uuid::nil(),
                name: "tun".to_string(),
                created_at: String::new(),
                deleted_at: String::new(),
                connections: vec![],
            },
            token: "abc123".to_string(),
        };
        let json = serde_json::to_string(&twt).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
        // token is at top level, not nested
        assert_eq!(v["token"], "abc123");
        assert_eq!(v["name"], "tun");
    }

    // -- Management resource (CDC-038) ------------------------------------

    #[test]
    fn management_resource_str_matches_go() {
        assert_eq!(ManagementResource::Logs.as_str(), "logs");
        assert_eq!(ManagementResource::Admin.as_str(), "admin");
        assert_eq!(ManagementResource::HostDetails.as_str(), "host_details");
    }

    #[test]
    fn management_resource_display() {
        assert_eq!(ManagementResource::Logs.to_string(), "logs");
        assert_eq!(ManagementResource::Admin.to_string(), "admin");
        assert_eq!(ManagementResource::HostDetails.to_string(), "host_details");
    }

    // -- Virtual network (CDC-037) ----------------------------------------

    #[test]
    fn vnet_uses_is_default_network_key() {
        let vn = VirtualNetwork {
            id: Uuid::nil(),
            name: "default".to_string(),
            comment: String::new(),
            is_default: true,
            created_at: String::new(),
            deleted_at: String::new(),
        };
        let json = serde_json::to_string(&vn).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
        // Go uses "is_default_network"
        assert!(v.get("is_default_network").is_some());
        assert!(v.get("is_default").is_none());
    }

    #[test]
    fn update_vnet_omits_none_fields() {
        let u = UpdateVirtualNetwork {
            name: Some("new-name".to_string()),
            comment: None,
            is_default: None,
        };
        let json = serde_json::to_string(&u).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert!(v.get("name").is_some());
        assert!(v.get("comment").is_none());
        assert!(v.get("is_default_network").is_none());
    }

    // -- IP routes (CDC-036) ----------------------------------------------

    #[test]
    fn detailed_route_json_keys_match_go() {
        let r = DetailedRoute {
            id: Uuid::nil(),
            network: "10.0.0.0/8".to_string(),
            tunnel_id: Uuid::nil(),
            virtual_network_id: None,
            comment: "test".to_string(),
            created_at: String::new(),
            deleted_at: String::new(),
            tunnel_name: "tun".to_string(),
        };
        let json = serde_json::to_string(&r).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert!(v.get("network").is_some());
        assert!(v.get("tunnel_id").is_some());
        assert!(v.get("tunnel_name").is_some());
        assert!(v.get("virtual_network_id").is_none()); // None → omitted
    }

    // -- Hostname routing (CDC-039) ---------------------------------------

    #[test]
    fn dns_route_request_type_is_dns() {
        let req = DnsRouteRequest::new("example.com".to_string(), true);
        let json = serde_json::to_string(&req).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(v["type"], "dns");
        assert_eq!(v["user_hostname"], "example.com");
        assert_eq!(v["overwrite_existing"], true);
    }

    #[test]
    fn lb_route_request_type_is_lb() {
        let req = LbRouteRequest::new("my-lb".to_string(), "pool-1".to_string());
        let json = serde_json::to_string(&req).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(v["type"], "lb");
        assert_eq!(v["lb_name"], "my-lb");
        assert_eq!(v["lb_pool"], "pool-1");
    }

    /// CDC-038: `ManagementResource` repr values match Go's iota (0, 1, 2).
    #[test]
    fn management_resource_repr_matches_go_iota() {
        assert_eq!(ManagementResource::Logs as u8, 0);
        assert_eq!(ManagementResource::Admin as u8, 1);
        assert_eq!(ManagementResource::HostDetails as u8, 2);
    }

    /// CDC-033: `ActiveClient` deserializes from Go-shaped JSON with
    /// the `"conns"` key (not `"connections"`).
    #[test]
    fn active_client_deserialize_go_json_with_conns() {
        let go_json = r#"{
            "id": "00000000-0000-0000-0000-000000000000",
            "features": ["allow_remote_config"],
            "version": "2026.2.0",
            "arch": "linux_amd64",
            "run_at": "2025-01-01T00:00:00Z",
            "conns": [
                {
                    "colo_name": "DFW",
                    "id": "00000000-0000-0000-0000-000000000000",
                    "is_pending_reconnect": false,
                    "origin_ip": "",
                    "opened_at": ""
                }
            ]
        }"#;
        let client: ActiveClient = serde_json::from_str(go_json).expect("deserialize");
        assert_eq!(client.version, "2026.2.0");
        assert_eq!(client.connections.len(), 1);
        assert_eq!(client.connections[0].colo_name, "DFW");
    }

    /// CDC-039: DNS and LB route results deserialize from Go-shaped JSON.
    #[test]
    fn dns_and_lb_route_results_deserialize_go_json() {
        // DNS result
        let dns_json = r#"{"cname":"example.com.cdn.cloudflare.net","name":"example.com"}"#;
        let dns: DnsRouteResult = serde_json::from_str(dns_json).expect("deserialize dns");
        assert_eq!(dns.cname, "example.com.cdn.cloudflare.net");
        assert_eq!(dns.name, "example.com");

        // LB result
        let lb_json = r#"{"load_balancer":"my-lb.example.com","pool":"pool-1"}"#;
        let lb: LbRouteResult = serde_json::from_str(lb_json).expect("deserialize lb");
        assert_eq!(lb.load_balancer, "my-lb.example.com");
        assert_eq!(lb.pool, "pool-1");
    }

    // -- Hostname route envelope (CDC-039) --------------------------------

    #[test]
    fn hostname_route_dns_serializes_correctly() {
        let route = HostnameRoute::Dns(DnsRouteRequest::new("example.com".to_string(), true));
        let json = serde_json::to_string(&route).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(v["type"], "dns");
        assert_eq!(v["user_hostname"], "example.com");
    }

    #[test]
    fn hostname_route_lb_serializes_correctly() {
        let route = HostnameRoute::Lb(LbRouteRequest::new("lb-1".to_string(), "pool-1".to_string()));
        let json = serde_json::to_string(&route).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(v["type"], "lb");
        assert_eq!(v["lb_name"], "lb-1");
    }

    #[test]
    fn hostname_route_result_success_summary() {
        let dns = HostnameRouteResult::Dns(DnsRouteResult {
            cname: "new".to_string(),
            name: "example.com".to_string(),
        });
        assert!(dns.success_summary().contains("example.com"));

        let lb = HostnameRouteResult::Lb(LbRouteResult {
            load_balancer: "updated".to_string(),
            pool: "new".to_string(),
        });
        assert!(lb.success_summary().contains("updated"));
    }

    // -- Filter types (CDC-033, CDC-036, CDC-037) -------------------------

    #[test]
    fn tunnel_filter_by_name_sets_is_deleted_false() {
        let f = TunnelFilter::by_name("my-tunnel");
        let pairs = f.to_query_pairs();
        assert!(pairs.iter().any(|(k, v)| *k == "name" && v == "my-tunnel"));
        assert!(pairs.iter().any(|(k, v)| *k == "is_deleted" && v == "false"));
    }

    #[test]
    fn tunnel_filter_query_pairs() {
        let f = TunnelFilter {
            name_prefix: Some("prod-".to_string()),
            per_page: Some(50),
            page: Some(2),
            ..Default::default()
        };
        let pairs = f.to_query_pairs();
        assert!(pairs.iter().any(|(k, v)| *k == "name_prefix" && v == "prod-"));
        assert!(pairs.iter().any(|(k, v)| *k == "per_page" && v == "50"));
        assert!(pairs.iter().any(|(k, v)| *k == "page" && v == "2"));
    }

    #[test]
    fn ip_route_filter_always_includes_tun_types() {
        let f = IpRouteFilter::default();
        let pairs = f.to_query_pairs();
        assert!(pairs.iter().any(|(k, v)| *k == "tun_types" && v == "cfd_tunnel"));
    }

    #[test]
    fn vnet_filter_by_name_sets_is_deleted_false() {
        let f = VnetFilter::by_name("default");
        let pairs = f.to_query_pairs();
        assert!(pairs.iter().any(|(k, v)| *k == "name" && v == "default"));
        assert!(pairs.iter().any(|(k, v)| *k == "is_deleted" && v == "false"));
    }
}
