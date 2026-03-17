//! Reqwest-based Cloudflare REST API client (CDC-033 through CDC-039).
//!
//! Implements the [`CloudflareApiClient`] trait defined in `cfdrs-cdc`
//! using `reqwest::blocking::Client`. This module lives in `cfdrs-bin`
//! (composition root) to keep the HTTP transport dependency out of the
//! contract crate.

use reqwest::blocking::Client;
use reqwest::header::{self, HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;
use uuid::Uuid;

use cfdrs_cdc::api::{
    API_ACCEPT_HEADER, AUTHORIZATION_BEARER_PREFIX, ApiClientConfig, ApiClientError, ApiResponse,
    CloudflareApiClient, DEFAULT_API_TIMEOUT,
};
use cfdrs_cdc::api_resources::{
    ActiveClient, DetailedRoute, DnsRouteResult, HostnameRoute, HostnameRouteResult, IpRouteFilter,
    LbRouteResult, ManagementResource, NewRoute, NewTunnel, NewVirtualNetwork, Route, Tunnel, TunnelFilter,
    TunnelWithToken, UpdateVirtualNetwork, VirtualNetwork, VnetFilter,
};

// ---------------------------------------------------------------------------
// Client struct
// ---------------------------------------------------------------------------

/// Concrete Cloudflare REST API client backed by `reqwest::blocking`.
pub struct ReqwestApiClient {
    client: Client,
    config: ApiClientConfig,
}

impl ReqwestApiClient {
    /// Build a new client from the given configuration.
    ///
    /// Sets default auth and accept headers so each request does not need
    /// to repeat them.
    pub fn new(config: ApiClientConfig) -> Result<Self, ApiClientError> {
        let mut default_headers = HeaderMap::new();
        default_headers.insert(header::ACCEPT, HeaderValue::from_static(API_ACCEPT_HEADER));

        let auth_value = format!("{}{}", AUTHORIZATION_BEARER_PREFIX, config.auth_token);
        let header_val = HeaderValue::from_str(&auth_value)
            .map_err(|e| ApiClientError::Transport(format!("invalid auth header: {e}")))?;
        default_headers.insert(header::AUTHORIZATION, header_val);

        let client = Client::builder()
            .timeout(DEFAULT_API_TIMEOUT)
            .user_agent(&config.user_agent)
            .default_headers(default_headers)
            .build()
            .map_err(transport_error)?;

        Ok(Self { client, config })
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

impl ReqwestApiClient {
    /// Send GET and parse the response envelope.
    fn get_envelope(&self, url: &str) -> Result<ApiResponse, ApiClientError> {
        let response = self.client.get(url).send().map_err(transport_error)?;

        parse_response(response)
    }

    /// Send GET with query params and parse the response envelope.
    fn get_envelope_with_query(
        &self,
        url: &str,
        params: &[(&str, String)],
    ) -> Result<ApiResponse, ApiClientError> {
        let full_url = build_query_url(url, params);

        let response = self.client.get(&full_url).send().map_err(transport_error)?;

        parse_response(response)
    }

    /// Paginated GET — fetches all pages and collects results.
    ///
    /// Matches `fetchExhaustively` in `cfapi/base_client.go`.
    fn fetch_paginated<T: DeserializeOwned>(
        &self,
        url: &str,
        base_params: &[(&str, String)],
    ) -> Result<Vec<T>, ApiClientError> {
        let mut all_results: Vec<T> = Vec::new();
        let mut page = 1u32;

        loop {
            let page_str = page.to_string();
            let mut params: Vec<(&str, String)> = base_params.to_vec();
            params.push(("page", page_str));

            let api_resp = self.get_envelope_with_query(url, &params)?;
            api_resp.check()?;

            let items: Vec<T> = api_resp.parse_result()?;
            let item_count = items.len();
            all_results.extend(items);

            let should_continue = match &api_resp.result_info {
                Some(info) => item_count >= info.per_page && all_results.len() < info.total_count,
                None => false,
            };

            if !should_continue {
                break;
            }

            page += 1;
        }

        Ok(all_results)
    }
}

/// Build a URL with query parameters appended.
///
/// Values are simple ASCII (UUIDs, booleans, numbers, filter strings)
/// and do not require percent-encoding.
fn build_query_url(base: &str, params: &[(&str, String)]) -> String {
    if params.is_empty() {
        return base.to_string();
    }

    let query: String = params
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");

    format!("{base}?{query}")
}

/// Map an HTTP response to an `ApiResponse`, checking for status-code
/// level errors first.
fn parse_response(response: reqwest::blocking::Response) -> Result<ApiResponse, ApiClientError> {
    let status = response.status();

    if let Some(err) = map_status_error(status) {
        return Err(err);
    }

    response.json::<ApiResponse>().map_err(transport_error)
}

/// Convert a reqwest error to `ApiClientError::Transport`.
fn transport_error(e: reqwest::Error) -> ApiClientError {
    ApiClientError::Transport(e.to_string())
}

/// Map HTTP status codes to `ApiClientError` for non-200 responses.
///
/// Matches the switch in `cfapi/base_client.go:sendRequest`.
fn map_status_error(status: reqwest::StatusCode) -> Option<ApiClientError> {
    match status.as_u16() {
        200 | 201 => None,
        401 | 403 => Some(ApiClientError::Unauthorized),
        400 => Some(ApiClientError::BadRequest),
        404 => Some(ApiClientError::NotFound),
        409 => Some(ApiClientError::TunnelNameConflict),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Trait implementation
// ---------------------------------------------------------------------------

impl CloudflareApiClient for ReqwestApiClient {
    // -- TunnelClient (CDC-033) -------------------------------------------

    fn create_tunnel(&self, name: &str, tunnel_secret: &[u8]) -> Result<TunnelWithToken, ApiClientError> {
        use base64::Engine;
        let body = NewTunnel {
            name: name.to_string(),
            tunnel_secret: base64::engine::general_purpose::STANDARD.encode(tunnel_secret),
        };

        let url = self.config.account_tunnel_url();

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .map_err(transport_error)?;

        let api_resp = parse_response(response)?;
        api_resp.check()?;
        api_resp.parse_result()
    }

    fn get_tunnel(&self, tunnel_id: Uuid) -> Result<Tunnel, ApiClientError> {
        let url = format!("{}/{}", self.config.account_tunnel_url(), tunnel_id);
        let api_resp = self.get_envelope(&url)?;
        api_resp.check()?;
        api_resp.parse_result()
    }

    fn get_tunnel_token(&self, tunnel_id: Uuid) -> Result<String, ApiClientError> {
        let url = format!("{}/{}/token", self.config.account_tunnel_url(), tunnel_id);
        let api_resp = self.get_envelope(&url)?;
        api_resp.check()?;
        api_resp.parse_result()
    }

    fn delete_tunnel(&self, tunnel_id: Uuid, cascade: bool) -> Result<(), ApiClientError> {
        let url = format!(
            "{}/{}?cascade={}",
            self.config.account_tunnel_url(),
            tunnel_id,
            cascade
        );

        let response = self.client.delete(&url).send().map_err(transport_error)?;

        let api_resp = parse_response(response)?;
        api_resp.check()?;
        Ok(())
    }

    fn list_tunnels(&self, filter: &TunnelFilter) -> Result<Vec<Tunnel>, ApiClientError> {
        let url = self.config.account_tunnel_url();
        let pairs = filter.to_query_pairs();
        let params: Vec<(&str, String)> = pairs.iter().map(|(k, v)| (*k, v.clone())).collect();
        self.fetch_paginated(&url, &params)
    }

    fn list_active_clients(&self, tunnel_id: Uuid) -> Result<Vec<ActiveClient>, ApiClientError> {
        let url = format!("{}/{}/connections", self.config.account_tunnel_url(), tunnel_id);
        let api_resp = self.get_envelope(&url)?;
        api_resp.check()?;
        api_resp.parse_result()
    }

    fn cleanup_connections(&self, tunnel_id: Uuid, connector_id: Option<Uuid>) -> Result<(), ApiClientError> {
        let base_url = format!("{}/{}/connections", self.config.account_tunnel_url(), tunnel_id);

        let url = match connector_id {
            Some(id) => format!("{}?client_id={}", base_url, id),
            None => base_url,
        };

        let response = self.client.delete(&url).send().map_err(transport_error)?;

        let api_resp = parse_response(response)?;
        api_resp.check()?;
        Ok(())
    }

    // -- IPRouteClient (CDC-036) ------------------------------------------

    fn list_routes(&self, filter: &IpRouteFilter) -> Result<Vec<DetailedRoute>, ApiClientError> {
        let url = self.config.account_route_url();
        let pairs = filter.to_query_pairs();
        let params: Vec<(&str, String)> = pairs.iter().map(|(k, v)| (*k, v.clone())).collect();
        self.fetch_paginated(&url, &params)
    }

    fn add_route(&self, new_route: &NewRoute) -> Result<Route, ApiClientError> {
        let url = self.config.account_route_url();

        let response = self
            .client
            .post(&url)
            .json(new_route)
            .send()
            .map_err(transport_error)?;

        let api_resp = parse_response(response)?;
        api_resp.check()?;
        api_resp.parse_result()
    }

    fn delete_route(&self, route_id: Uuid) -> Result<(), ApiClientError> {
        let url = format!("{}/{}", self.config.account_route_url(), route_id);

        let response = self.client.delete(&url).send().map_err(transport_error)?;

        let api_resp = parse_response(response)?;
        api_resp.check()?;
        Ok(())
    }

    fn get_route_by_ip(&self, ip: &str, vnet_id: Option<Uuid>) -> Result<DetailedRoute, ApiClientError> {
        let base_url = format!("{}/ip/{}", self.config.account_route_url(), ip);

        let url = match vnet_id {
            Some(id) => format!("{}?virtual_network_id={}", base_url, id),
            None => base_url,
        };

        let api_resp = self.get_envelope(&url)?;
        api_resp.check()?;
        api_resp.parse_result()
    }

    // -- VnetClient (CDC-037) ---------------------------------------------

    fn create_virtual_network(&self, new_vnet: &NewVirtualNetwork) -> Result<VirtualNetwork, ApiClientError> {
        let url = self.config.account_vnet_url();

        let response = self
            .client
            .post(&url)
            .json(new_vnet)
            .send()
            .map_err(transport_error)?;

        let api_resp = parse_response(response)?;
        api_resp.check()?;
        api_resp.parse_result()
    }

    fn list_virtual_networks(&self, filter: &VnetFilter) -> Result<Vec<VirtualNetwork>, ApiClientError> {
        // Go: single-page fetch (not paginated unlike tunnels/routes).
        let url = self.config.account_vnet_url();
        let pairs = filter.to_query_pairs();
        let params: Vec<(&str, String)> = pairs.iter().map(|(k, v)| (*k, v.clone())).collect();
        let api_resp = self.get_envelope_with_query(&url, &params)?;
        api_resp.check()?;
        api_resp.parse_result()
    }

    fn delete_virtual_network(&self, id: Uuid, force: bool) -> Result<(), ApiClientError> {
        let url = format!("{}?force={}", self.vnet_url(id), force);

        let response = self.client.delete(&url).send().map_err(transport_error)?;

        let api_resp = parse_response(response)?;
        api_resp.check()?;
        Ok(())
    }

    fn update_virtual_network(&self, id: Uuid, updates: &UpdateVirtualNetwork) -> Result<(), ApiClientError> {
        let url = self.vnet_url(id);

        let response = self
            .client
            .patch(&url)
            .json(updates)
            .send()
            .map_err(transport_error)?;

        let api_resp = parse_response(response)?;
        api_resp.check()?;
        Ok(())
    }

    // -- Management (CDC-038) ---------------------------------------------

    fn get_management_token(
        &self,
        tunnel_id: Uuid,
        resource: ManagementResource,
    ) -> Result<String, ApiClientError> {
        let url = format!(
            "{}/{}/management/{}",
            self.config.account_tunnel_url(),
            tunnel_id,
            resource
        );

        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({"resource": resource.as_str()}))
            .send()
            .map_err(transport_error)?;

        let api_resp = parse_response(response)?;
        api_resp.check()?;
        api_resp.parse_result()
    }

    // -- HostnameClient (CDC-039) -----------------------------------------

    fn route_tunnel(
        &self,
        tunnel_id: Uuid,
        route: &HostnameRoute,
    ) -> Result<HostnameRouteResult, ApiClientError> {
        let url = format!("{}/{}/routes", self.config.zone_tunnel_url(), tunnel_id);

        let response = self
            .client
            .put(&url)
            .json(route)
            .send()
            .map_err(transport_error)?;

        let api_resp = parse_response(response)?;
        api_resp.check()?;

        // The result type depends on the request type.
        match route {
            HostnameRoute::Dns(_) => {
                let result: DnsRouteResult = api_resp.parse_result()?;
                Ok(HostnameRouteResult::Dns(result))
            }
            HostnameRoute::Lb(_) => {
                let result: LbRouteResult = api_resp.parse_result()?;
                Ok(HostnameRouteResult::Lb(result))
            }
        }
    }
}

// Convenience URL builder for vnet + ID.
impl ReqwestApiClient {
    fn vnet_url(&self, id: Uuid) -> String {
        format!("{}/{}", self.config.account_vnet_url(), id)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_status_error_200_is_none() {
        assert!(map_status_error(reqwest::StatusCode::OK).is_none());
    }

    #[test]
    fn map_status_error_201_is_none() {
        assert!(map_status_error(reqwest::StatusCode::CREATED).is_none());
    }

    #[test]
    fn map_status_error_401_is_unauthorized() {
        let err = map_status_error(reqwest::StatusCode::UNAUTHORIZED).expect("should be error");
        assert!(matches!(err, ApiClientError::Unauthorized));
    }

    #[test]
    fn map_status_error_403_is_unauthorized() {
        let err = map_status_error(reqwest::StatusCode::FORBIDDEN).expect("should be error");
        assert!(matches!(err, ApiClientError::Unauthorized));
    }

    #[test]
    fn map_status_error_404_is_not_found() {
        let err = map_status_error(reqwest::StatusCode::NOT_FOUND).expect("should be error");
        assert!(matches!(err, ApiClientError::NotFound));
    }

    #[test]
    fn map_status_error_409_is_conflict() {
        let err = map_status_error(reqwest::StatusCode::CONFLICT).expect("should be error");
        assert!(matches!(err, ApiClientError::TunnelNameConflict));
    }

    #[test]
    fn map_status_error_500_is_none() {
        // Unknown codes fall through to response body parsing.
        assert!(map_status_error(reqwest::StatusCode::INTERNAL_SERVER_ERROR).is_none());
    }

    #[test]
    fn client_construction_succeeds_with_valid_config() {
        let config = ApiClientConfig {
            base_url: "https://api.cloudflare.com/client/v4".to_string(),
            account_tag: "abc".to_string(),
            zone_tag: "zone".to_string(),
            auth_token: "token123".to_string(),
            user_agent: "cloudflared/test".to_string(),
        };
        let client = ReqwestApiClient::new(config);
        assert!(client.is_ok());
    }
}
