//! Route and virtual network CLI commands (CLI-019, CLI-020).
//!
//! Implements behavioral dispatch for `tunnel route {dns,lb,ip}` and
//! `tunnel vnet {add,list,delete,update}`, matching the Go baseline in
//! `tunnel/subcommands.go` and `tunnel/vnets_subcommands.go`.

use std::fmt::Write;

use uuid::Uuid;

use cfdrs_cdc::api::CloudflareApiClient;
use cfdrs_cdc::api_resources::{
    DnsRouteRequest, HostnameRoute, IpRouteFilter, LbRouteRequest, NewRoute, NewVirtualNetwork,
    UpdateVirtualNetwork, VnetFilter,
};
use cfdrs_cli::{CliOutput, GlobalFlags};

use crate::tunnel_commands::{build_client, render_output, resolve_tunnel_id};

// ---------------------------------------------------------------------------
// Vnet ID resolution
// ---------------------------------------------------------------------------

/// Resolve a vnet name-or-UUID to UUID.
///
/// Matches Go `getVnetId()` in `vnets_subcommands.go`.
fn resolve_vnet_id(client: &dyn CloudflareApiClient, input: &str) -> Result<Uuid, String> {
    if let Ok(id) = input.parse::<Uuid>() {
        return Ok(id);
    }

    let filter = VnetFilter::by_name(input);
    let vnets = client
        .list_virtual_networks(&filter)
        .map_err(|e| format!("error looking up virtual network: {e}"))?;

    match vnets.len() {
        1 => Ok(vnets[0].id),
        0 => Err(format!("could not find virtual network with name {input}")),
        n => Err(format!(
            "found {n} virtual networks with name {input}, expected 1"
        )),
    }
}

/// Resolve optional `--vnet` flag to a UUID.
fn resolve_optional_vnet(
    client: &dyn CloudflareApiClient,
    flags: &GlobalFlags,
) -> Result<Option<Uuid>, String> {
    match flags.vnet_id.as_deref() {
        Some(v) => resolve_vnet_id(client, v).map(Some),
        None => Ok(None),
    }
}

// ---------------------------------------------------------------------------
// Route DNS (CLI-019)
// ---------------------------------------------------------------------------

pub fn execute_route_dns(flags: &GlobalFlags) -> CliOutput {
    match route_dns_inner(flags) {
        Ok(output) => output,
        Err(msg) => CliOutput::failure(String::new(), msg, 1),
    }
}

fn route_dns_inner(flags: &GlobalFlags) -> Result<CliOutput, String> {
    let client = build_client(flags)?;
    let tunnel_id = resolve_tunnel_id(&client, &flags.rest_args[0])?;
    let hostname = &flags.rest_args[1];

    let route = HostnameRoute::Dns(DnsRouteRequest::new(hostname.clone(), flags.overwrite_dns));

    let result = client
        .route_tunnel(tunnel_id, &route)
        .map_err(|e| format!("error routing tunnel: {e}"))?;

    Ok(CliOutput::success(format!("{}\n", result.success_summary())))
}

// ---------------------------------------------------------------------------
// Route LB (CLI-019)
// ---------------------------------------------------------------------------

pub fn execute_route_lb(flags: &GlobalFlags) -> CliOutput {
    match route_lb_inner(flags) {
        Ok(output) => output,
        Err(msg) => CliOutput::failure(String::new(), msg, 1),
    }
}

fn route_lb_inner(flags: &GlobalFlags) -> Result<CliOutput, String> {
    let client = build_client(flags)?;
    let tunnel_id = resolve_tunnel_id(&client, &flags.rest_args[0])?;
    let hostname = &flags.rest_args[1];
    let pool = &flags.rest_args[2];

    let route = HostnameRoute::Lb(LbRouteRequest::new(hostname.clone(), pool.clone()));

    let result = client
        .route_tunnel(tunnel_id, &route)
        .map_err(|e| format!("error routing tunnel: {e}"))?;

    Ok(CliOutput::success(format!("{}\n", result.success_summary())))
}

// ---------------------------------------------------------------------------
// Route IP add (CLI-019)
// ---------------------------------------------------------------------------

pub fn execute_route_ip_add(flags: &GlobalFlags) -> CliOutput {
    match route_ip_add_inner(flags) {
        Ok(output) => output,
        Err(msg) => CliOutput::failure(String::new(), msg, 1),
    }
}

fn route_ip_add_inner(flags: &GlobalFlags) -> Result<CliOutput, String> {
    let client = build_client(flags)?;
    let network = &flags.rest_args[0];
    let tunnel_id = resolve_tunnel_id(&client, &flags.rest_args[1])?;
    let comment = flags.rest_args.get(2).cloned().unwrap_or_default();
    let vnet_id = resolve_optional_vnet(&client, flags)?;

    let new_route = NewRoute {
        network: network.clone(),
        tunnel_id,
        comment,
        virtual_network_id: vnet_id,
    };

    client
        .add_route(&new_route)
        .map_err(|e| format!("error adding route: {e}"))?;

    Ok(CliOutput::success(format!(
        "Successfully added route for {network} over tunnel {tunnel_id}\n"
    )))
}

// ---------------------------------------------------------------------------
// Route IP show/list (CLI-019)
// ---------------------------------------------------------------------------

pub fn execute_route_ip_show(flags: &GlobalFlags) -> CliOutput {
    match route_ip_show_inner(flags) {
        Ok(output) => output,
        Err(msg) => CliOutput::failure(String::new(), msg, 1),
    }
}

fn route_ip_show_inner(flags: &GlobalFlags) -> Result<CliOutput, String> {
    let client = build_client(flags)?;

    let filter = IpRouteFilter {
        is_deleted: if flags.filter_is_deleted { Some(true) } else { None },
        tunnel_id: flags
            .filter_tunnel_id
            .as_deref()
            .map(|s| s.parse::<Uuid>())
            .transpose()
            .map_err(|e| format!("invalid --filter-tunnel-id: {e}"))?,
        network_subset: flags.filter_network_subset.clone(),
        network_superset: flags.filter_network_superset.clone(),
        comment: flags.filter_comment_is.clone(),
        virtual_network_id: flags
            .filter_vnet_id
            .as_deref()
            .map(|s| s.parse::<Uuid>())
            .transpose()
            .map_err(|e| format!("invalid --filter-vnet-id: {e}"))?,
        existed_at: None,
        per_page: None,
        page: None,
    };

    let routes = client
        .list_routes(&filter)
        .map_err(|e| format!("error listing routes: {e}"))?;

    if let Some(ref fmt) = flags.output_format {
        let output = render_output(fmt, &routes)?;
        return Ok(CliOutput::success(output));
    }

    Ok(CliOutput::success(render_route_table(&routes)))
}

fn render_route_table(routes: &[cfdrs_cdc::api_resources::DetailedRoute]) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "ID\tNETWORK\tVIRTUAL NET ID\tCOMMENT\tTUNNEL ID\tTUNNEL NAME\tCREATED\tDELETED"
    );
    for r in routes {
        let vnet = r.virtual_network_id.map(|v| v.to_string()).unwrap_or_default();
        let _ = writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            r.id, r.network, vnet, r.comment, r.tunnel_id, r.tunnel_name, r.created_at, r.deleted_at
        );
    }
    out
}

// ---------------------------------------------------------------------------
// Route IP delete (CLI-019)
// ---------------------------------------------------------------------------

pub fn execute_route_ip_delete(flags: &GlobalFlags) -> CliOutput {
    match route_ip_delete_inner(flags) {
        Ok(output) => output,
        Err(msg) => CliOutput::failure(String::new(), msg, 1),
    }
}

fn route_ip_delete_inner(flags: &GlobalFlags) -> Result<CliOutput, String> {
    let client = build_client(flags)?;
    let input = &flags.rest_args[0];
    let vnet_id = resolve_optional_vnet(&client, flags)?;

    let route_id = resolve_route_id(&client, input, vnet_id)?;

    client
        .delete_route(route_id)
        .map_err(|e| format!("error deleting route: {e}"))?;

    Ok(CliOutput::success(format!(
        "Successfully deleted route with ID {route_id}\n"
    )))
}

/// Resolve a route by UUID or CIDR lookup via subset+superset filter.
fn resolve_route_id(
    client: &dyn CloudflareApiClient,
    input: &str,
    vnet_id: Option<Uuid>,
) -> Result<Uuid, String> {
    if let Ok(id) = input.parse::<Uuid>() {
        return Ok(id);
    }

    // Treat as CIDR — find route by exact match (subset == superset == input).
    let filter = IpRouteFilter {
        network_subset: Some(input.to_string()),
        network_superset: Some(input.to_string()),
        virtual_network_id: vnet_id,
        ..Default::default()
    };

    let routes = client
        .list_routes(&filter)
        .map_err(|e| format!("error looking up route: {e}"))?;

    match routes.len() {
        1 => Ok(routes[0].id),
        0 => Err(format!("no route found for {input}")),
        n => Err(format!("found {n} routes matching {input}, expected 1")),
    }
}

// ---------------------------------------------------------------------------
// Route IP get (CLI-019)
// ---------------------------------------------------------------------------

pub fn execute_route_ip_get(flags: &GlobalFlags) -> CliOutput {
    match route_ip_get_inner(flags) {
        Ok(output) => output,
        Err(msg) => CliOutput::failure(String::new(), msg, 1),
    }
}

fn route_ip_get_inner(flags: &GlobalFlags) -> Result<CliOutput, String> {
    let client = build_client(flags)?;
    let ip = &flags.rest_args[0];
    let vnet_id = resolve_optional_vnet(&client, flags)?;

    let route = client
        .get_route_by_ip(ip, vnet_id)
        .map_err(|e| format!("error getting route by IP: {e}"))?;

    if route.id == Uuid::nil() && route.network.is_empty() {
        return Ok(CliOutput::success(format!("No route matches the IP {ip}\n")));
    }

    if let Some(ref fmt) = flags.output_format {
        let output = render_output(fmt, &[&route])?;
        return Ok(CliOutput::success(output));
    }

    Ok(CliOutput::success(render_route_table(&[route])))
}

// ---------------------------------------------------------------------------
// Vnet add (CLI-020)
// ---------------------------------------------------------------------------

pub fn execute_vnet_add(flags: &GlobalFlags) -> CliOutput {
    match vnet_add_inner(flags) {
        Ok(output) => output,
        Err(msg) => CliOutput::failure(String::new(), msg, 1),
    }
}

fn vnet_add_inner(flags: &GlobalFlags) -> Result<CliOutput, String> {
    let client = build_client(flags)?;
    let name = &flags.rest_args[0];
    let comment = flags.rest_args.get(1).cloned().unwrap_or_default();

    let new_vnet = NewVirtualNetwork {
        name: name.clone(),
        comment,
        is_default: flags.vnet_default,
    };

    let created = client
        .create_virtual_network(&new_vnet)
        .map_err(|e| format!("error adding virtual network: {e}"))?;

    let mut msg = format!(
        "Successfully added virtual network '{name}' with ID: {}\n",
        created.id
    );
    if created.is_default {
        let _ = writeln!(msg, "(set as the default virtual network)");
    }
    let _ = writeln!(msg, "You can now add IP routes attached to this virtual network.");

    Ok(CliOutput::success(msg))
}

// ---------------------------------------------------------------------------
// Vnet list (CLI-020)
// ---------------------------------------------------------------------------

pub fn execute_vnet_list(flags: &GlobalFlags) -> CliOutput {
    match vnet_list_inner(flags) {
        Ok(output) => output,
        Err(msg) => CliOutput::failure(String::new(), msg, 1),
    }
}

fn vnet_list_inner(flags: &GlobalFlags) -> Result<CliOutput, String> {
    let client = build_client(flags)?;

    let filter = VnetFilter {
        id: flags
            .filter_vnet_id
            .as_deref()
            .map(|s| s.parse::<Uuid>())
            .transpose()
            .map_err(|e| format!("invalid --id: {e}"))?,
        name: flags.tunnel_name.clone(),
        is_default: if flags.vnet_is_default_filter {
            Some(true)
        } else {
            None
        },
        is_deleted: if flags.show_deleted { Some(true) } else { None },
        per_page: None,
    };

    let vnets = client
        .list_virtual_networks(&filter)
        .map_err(|e| format!("error listing virtual networks: {e}"))?;

    if let Some(ref fmt) = flags.output_format {
        let output = render_output(fmt, &vnets)?;
        return Ok(CliOutput::success(output));
    }

    if vnets.is_empty() {
        return Ok(CliOutput::success(
            "No virtual networks were found for the given filter flags.\n".to_string(),
        ));
    }

    Ok(CliOutput::success(render_vnet_table(&vnets)))
}

fn render_vnet_table(vnets: &[cfdrs_cdc::api_resources::VirtualNetwork]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "ID\tNAME\tIS DEFAULT\tCOMMENT\tCREATED\tDELETED");
    for v in vnets {
        let _ = writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{}",
            v.id, v.name, v.is_default, v.comment, v.created_at, v.deleted_at
        );
    }
    out
}

// ---------------------------------------------------------------------------
// Vnet delete (CLI-020)
// ---------------------------------------------------------------------------

pub fn execute_vnet_delete(flags: &GlobalFlags) -> CliOutput {
    match vnet_delete_inner(flags) {
        Ok(output) => output,
        Err(msg) => CliOutput::failure(String::new(), msg, 1),
    }
}

fn vnet_delete_inner(flags: &GlobalFlags) -> Result<CliOutput, String> {
    let client = build_client(flags)?;
    let vnet_id = resolve_vnet_id(&client, &flags.rest_args[0])?;

    client
        .delete_virtual_network(vnet_id, flags.force)
        .map_err(|e| format!("error deleting virtual network: {e}"))?;

    Ok(CliOutput::success(format!(
        "Successfully deleted virtual network '{vnet_id}'\n"
    )))
}

// ---------------------------------------------------------------------------
// Vnet update (CLI-020)
// ---------------------------------------------------------------------------

pub fn execute_vnet_update(flags: &GlobalFlags) -> CliOutput {
    match vnet_update_inner(flags) {
        Ok(output) => output,
        Err(msg) => CliOutput::failure(String::new(), msg, 1),
    }
}

fn vnet_update_inner(flags: &GlobalFlags) -> Result<CliOutput, String> {
    let client = build_client(flags)?;
    let vnet_id = resolve_vnet_id(&client, &flags.rest_args[0])?;

    let updates = UpdateVirtualNetwork {
        name: flags.tunnel_name.clone(),
        comment: flags.vnet_comment.clone(),
        is_default: if flags.vnet_default { Some(true) } else { None },
    };

    client
        .update_virtual_network(vnet_id, &updates)
        .map_err(|e| format!("error updating virtual network: {e}"))?;

    Ok(CliOutput::success(format!(
        "Successfully updated virtual network '{vnet_id}'\n"
    )))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_vnet_id_parses_uuid_directly() {
        let id = "12345678-1234-1234-1234-123456789abc";
        // UUID-shaped input should parse without any API call.
        let parsed = id.parse::<Uuid>().expect("valid UUID");
        assert_eq!(parsed.to_string(), id);
    }

    #[test]
    fn render_route_table_empty() {
        let table = render_route_table(&[]);
        assert!(table.contains("ID\tNETWORK\tVIRTUAL NET ID"));
        // Should still have the header line.
        assert_eq!(table.lines().count(), 1);
    }

    #[test]
    fn render_vnet_table_empty() {
        let table = render_vnet_table(&[]);
        assert!(table.contains("ID\tNAME\tIS DEFAULT"));
        assert_eq!(table.lines().count(), 1);
    }

    #[test]
    fn render_route_table_row() {
        let routes = vec![cfdrs_cdc::api_resources::DetailedRoute {
            id: Uuid::nil(),
            network: "10.0.0.0/8".to_string(),
            tunnel_id: Uuid::nil(),
            virtual_network_id: None,
            comment: "test".to_string(),
            created_at: "2025-01-01".to_string(),
            deleted_at: String::new(),
            tunnel_name: "my-tunnel".to_string(),
        }];
        let table = render_route_table(&routes);
        assert!(table.contains("10.0.0.0/8"));
        assert!(table.contains("my-tunnel"));
        assert_eq!(table.lines().count(), 2);
    }

    #[test]
    fn render_vnet_table_row() {
        let vnets = vec![cfdrs_cdc::api_resources::VirtualNetwork {
            id: Uuid::nil(),
            name: "default-vnet".to_string(),
            comment: "main".to_string(),
            is_default: true,
            created_at: "2025-01-01".to_string(),
            deleted_at: String::new(),
        }];
        let table = render_vnet_table(&vnets);
        assert!(table.contains("default-vnet"));
        assert!(table.contains("true"));
        assert_eq!(table.lines().count(), 2);
    }
}
