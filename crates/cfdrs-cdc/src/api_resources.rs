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
}
