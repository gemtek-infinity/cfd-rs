/// Per-subcommand help rendering matching Go baseline `commandHelpTemplate()`.
///
/// Go baseline: every tunnel subcommand has a `CustomHelpTemplate` that renders
/// NAME, USAGE, DESCRIPTION, TUNNEL COMMAND OPTIONS (12 parent flags), and
/// SUBCOMMAND OPTIONS (per-subcommand flags).
use crate::types::HelpTarget;

// --- Help data types ---

struct SubcommandFlagEntry {
    /// Flag names as shown in help, e.g. `"--output value, -o value"`.
    names: &'static str,
    /// Usage description, e.g. `"Render output using given FORMAT."`.
    usage: &'static str,
}

struct SubcommandHelpSpec {
    /// Go `HelpName`, e.g. `"cloudflared tunnel create"`.
    help_name: &'static str,
    /// Go `Usage`, e.g. `"Create a new tunnel with given name"`.
    usage: &'static str,
    /// Go `UsageText`, e.g. `"cloudflared tunnel [...] create [...] NAME"`.
    usage_text: &'static str,
    /// Go `Description` — may be multi-line.
    description: &'static str,
    /// Per-subcommand flags (the SUBCOMMAND OPTIONS section).
    flags: &'static [SubcommandFlagEntry],
}

// --- Tunnel command options (12 parent flags from Go configureCloudflaredFlags
// + ConfigureLoggingFlags) ---

const TUNNEL_COMMAND_OPTIONS: &[SubcommandFlagEntry] = &[
    SubcommandFlagEntry {
        names: "--config value",
        usage: "Specifies a config file in YAML format.",
    },
    SubcommandFlagEntry {
        names: "--origincert value",
        usage: "Path to the certificate generated for your origin when you run cloudflared login. \
                [$TUNNEL_ORIGIN_CERT]",
    },
    SubcommandFlagEntry {
        names: "--autoupdate-freq value",
        usage: "Autoupdate frequency. Default is 24h0m0s. (default: 24h0m0s)",
    },
    SubcommandFlagEntry {
        names: "--no-autoupdate",
        usage: "Disable periodic check for updates, restarting the server with the new version. (default: \
                false) [$NO_AUTOUPDATE]",
    },
    SubcommandFlagEntry {
        names: "--metrics value",
        usage: "Listen address for metrics reporting. [$TUNNEL_METRICS]",
    },
    SubcommandFlagEntry {
        names: "--pidfile value",
        usage: "Write the application's PID to this file after first successful connection. \
                [$TUNNEL_PIDFILE]",
    },
    SubcommandFlagEntry {
        names: "--loglevel value",
        usage: "Application logging level {debug, info, warn, error, fatal}. (default: \"info\") \
                [$TUNNEL_LOGLEVEL]",
    },
    SubcommandFlagEntry {
        names: "--transport-loglevel value, --proto-loglevel value",
        usage: "Transport logging level {debug, info, warn, error, fatal} (default: \"info\") \
                [$TUNNEL_PROTO_LOGLEVEL, $TUNNEL_TRANSPORT_LOGLEVEL]",
    },
    SubcommandFlagEntry {
        names: "--logfile value",
        usage: "Save application log to this file for reporting issues. [$TUNNEL_LOGFILE]",
    },
    SubcommandFlagEntry {
        names: "--log-directory value",
        usage: "Save application log to this directory for reporting issues. [$TUNNEL_LOGDIRECTORY]",
    },
    SubcommandFlagEntry {
        names: "--trace-output value",
        usage: "Name of trace output file, generated when cloudflared stops. [$TUNNEL_TRACE_OUTPUT]",
    },
    SubcommandFlagEntry {
        names: "--output value",
        usage: "Output format for the logs (default, json) (default: \"default\") \
                [$TUNNEL_MANAGEMENT_OUTPUT, $TUNNEL_LOG_OUTPUT]",
    },
];

// --- Per-subcommand help specs ---

const CREATE_SPEC: SubcommandHelpSpec = SubcommandHelpSpec {
    help_name: "cloudflared tunnel create",
    usage: "Create a new tunnel with given name",
    usage_text: "cloudflared tunnel [tunnel command options] create [subcommand options] NAME",
    description: "Creates a tunnel, registers it with Cloudflare edge and generates a credential file at \
                  the default credential file path. Use \"cloudflared tunnel route\" subcommand to map a \
                  DNS name to the tunnel.",
    flags: &[
        SubcommandFlagEntry {
            names: "--output value, -o value",
            usage: "Render output using given FORMAT. Valid options are 'json' or 'yaml'",
        },
        SubcommandFlagEntry {
            names: "--credentials-file value, --cred-file value",
            usage: "Filepath at which to read/write the tunnel credentials [$TUNNEL_CRED_FILE]",
        },
        SubcommandFlagEntry {
            names: "--secret value, -s value",
            usage: "Base64 encoded secret to set for the tunnel. The decoded secret must be at least 32 \
                    bytes long. If not specified, a random secret will be generated. [$TUNNEL_CREATE_SECRET]",
        },
    ],
};

const LIST_SPEC: SubcommandHelpSpec = SubcommandHelpSpec {
    help_name: "cloudflared tunnel list",
    usage: "List existing tunnels",
    usage_text: "cloudflared tunnel [tunnel command options] list [subcommand options]",
    description: "List all tunnels available. Use filters to narrow down the list.",
    flags: &[
        SubcommandFlagEntry {
            names: "--output value, -o value",
            usage: "Render output using given FORMAT. Valid options are 'json' or 'yaml'",
        },
        SubcommandFlagEntry {
            names: "--show-deleted, -d",
            usage: "Include deleted tunnels in the list (default: false)",
        },
        SubcommandFlagEntry {
            names: "--name value, -n value",
            usage: "List tunnels with the given NAME",
        },
        SubcommandFlagEntry {
            names: "--name-prefix value, -np value",
            usage: "List tunnels that start with the given NAME prefix",
        },
        SubcommandFlagEntry {
            names: "--exclude-name-prefix value, -enp value",
            usage: "List tunnels whose NAME does not start with the given prefix",
        },
        SubcommandFlagEntry {
            names: "--when value, -w value",
            usage: "List tunnels that are active at the given TIME in RFC3339 format",
        },
        SubcommandFlagEntry {
            names: "--id value, -i value",
            usage: "List tunnel by ID",
        },
        SubcommandFlagEntry {
            names: "--show-recently-disconnected, -rd",
            usage: "Include connections that have recently disconnected in the list (default: false)",
        },
        SubcommandFlagEntry {
            names: "--sort-by value",
            usage: "Sorts the list of tunnels by the given field. Valid options are {name, id, createdAt, \
                    deletedAt, numConnections} (default: \"name\") [$TUNNEL_LIST_SORT_BY]",
        },
        SubcommandFlagEntry {
            names: "--invert-sort",
            usage: "Inverts the sort order of the tunnel list. (default: false) [$TUNNEL_LIST_INVERT_SORT]",
        },
    ],
};

const RUN_SPEC: SubcommandHelpSpec = SubcommandHelpSpec {
    help_name: "cloudflared tunnel run",
    usage: "Proxy a local web server by running the given tunnel",
    usage_text: "cloudflared tunnel [tunnel command options] run [subcommand options] [TUNNEL]",
    description: "Run a tunnel by providing a tunnel name or tunnel UUID. If the tunnel is a remotely \
                  managed tunnel, the configuration for it will be pulled from the Cloudflare API. If it is \
                  a locally managed tunnel, the configuration will be read from the config file.\n\n   The \
                  tunnel can be identified by name or UUID. If both --token and a tunnel name/UUID are \
                  provided, the --token takes precedence.",
    flags: &[
        SubcommandFlagEntry {
            names: "--credentials-file value, --cred-file value",
            usage: "Filepath at which to read/write the tunnel credentials [$TUNNEL_CRED_FILE]",
        },
        SubcommandFlagEntry {
            names: "--credentials-contents value",
            usage: "Contents of the tunnel credentials JSON file to use [$TUNNEL_CRED_CONTENTS]",
        },
        SubcommandFlagEntry {
            names: "--token value",
            usage: "The Tunnel token [$TUNNEL_TOKEN]",
        },
        SubcommandFlagEntry {
            names: "--token-file value",
            usage: "Filepath at which to read the tunnel token [$TUNNEL_TOKEN_FILE]",
        },
        SubcommandFlagEntry {
            names: "--post-quantum, --pq",
            usage: "Create an experimental post-quantum secure tunnel (default: false)",
        },
        SubcommandFlagEntry {
            names: "--features value, -F value",
            usage: "Opt into various features that are still being developed or tested",
        },
        SubcommandFlagEntry {
            names: "--icmpv4-src value",
            usage: "Source address to send/receive ICMPv4 messages [$TUNNEL_ICMPV4_SRC]",
        },
        SubcommandFlagEntry {
            names: "--icmpv6-src value",
            usage: "Source address and interface name to send/receive ICMPv6 messages [$TUNNEL_ICMPV6_SRC]",
        },
        SubcommandFlagEntry {
            names: "--max-active-flows value",
            usage: "Overrides the remote configuration for max active private network flows \
                    [$TUNNEL_MAX_ACTIVE_FLOWS]",
        },
        SubcommandFlagEntry {
            names: "--url value",
            usage: "Connect to the local webserver at URL (default: \"http://localhost:8080\") [$TUNNEL_URL]",
        },
        SubcommandFlagEntry {
            names: "--hello-world",
            usage: "Run Hello World Server (default: false) [$TUNNEL_HELLO_WORLD]",
        },
    ],
};

const DELETE_SPEC: SubcommandHelpSpec = SubcommandHelpSpec {
    help_name: "cloudflared tunnel delete",
    usage: "Delete existing tunnel by UUID or name",
    usage_text: "cloudflared tunnel [tunnel command options] delete [subcommand options] TUNNEL",
    description: "Delete a tunnel by name or UUID. A tunnel must not have any active connections to be \
                  deleted unless the -f flag is used.",
    flags: &[
        SubcommandFlagEntry {
            names: "--credentials-file value, --cred-file value",
            usage: "Filepath at which to read/write the tunnel credentials [$TUNNEL_CRED_FILE]",
        },
        SubcommandFlagEntry {
            names: "--force, -f",
            usage: "Deletes a tunnel even if tunnel is connected and it has dependencies associated to it \
                    (default: false) [$TUNNEL_RUN_FORCE_OVERWRITE]",
        },
    ],
};

const CLEANUP_SPEC: SubcommandHelpSpec = SubcommandHelpSpec {
    help_name: "cloudflared tunnel cleanup",
    usage: "Cleanup tunnel connections",
    usage_text: "cloudflared tunnel [tunnel command options] cleanup [subcommand options] TUNNEL",
    description: "Delete connections for tunnels with the given tunnel UUIDs or names.",
    flags: &[SubcommandFlagEntry {
        names: "--connector-id value, -c value",
        usage: "Constraints the cleanup to stop the connections of a single Connector (by its ID) \
                [$TUNNEL_CLEANUP_CONNECTOR]",
    }],
};

const TOKEN_SPEC: SubcommandHelpSpec = SubcommandHelpSpec {
    help_name: "cloudflared tunnel token",
    usage: "Fetch the credentials token for an existing tunnel (by name or UUID) that allows to run it",
    usage_text: "cloudflared tunnel [tunnel command options] token [subcommand options] TUNNEL",
    description: "The token is base64-encoded and can be used with --token flag. Use --cred-file to specify \
                  the credential file path.",
    flags: &[SubcommandFlagEntry {
        names: "--credentials-file value, --cred-file value",
        usage: "Filepath at which to read/write the tunnel credentials [$TUNNEL_CRED_FILE]",
    }],
};

const INFO_SPEC: SubcommandHelpSpec = SubcommandHelpSpec {
    help_name: "cloudflared tunnel info",
    usage: "List details about the active connectors for a tunnel",
    usage_text: "cloudflared tunnel [tunnel command options] info [subcommand options] [TUNNEL]",
    description: "List details about the active connectors for a tunnel.",
    flags: &[
        SubcommandFlagEntry {
            names: "--output value, -o value",
            usage: "Render output using given FORMAT. Valid options are 'json' or 'yaml'",
        },
        SubcommandFlagEntry {
            names: "--show-recently-disconnected, -rd",
            usage: "Include connections that have recently disconnected in the list (default: false)",
        },
        SubcommandFlagEntry {
            names: "--sort-by value",
            usage: "Sorts the list of connections by the given field. Valid options are {id, startedAt, \
                    numConnections, version} (default: \"createdAt\")",
        },
        SubcommandFlagEntry {
            names: "--invert-sort",
            usage: "Inverts the sort order of the tunnel info. (default: false)",
        },
    ],
};

const READY_SPEC: SubcommandHelpSpec = SubcommandHelpSpec {
    help_name: "cloudflared tunnel ready",
    usage: "Call /ready endpoint and return proper exit code",
    usage_text: "cloudflared tunnel [tunnel command options] ready [subcommand options]",
    description: "Call the /ready endpoint of the metrics server and return proper exit code.",
    flags: &[],
};

const DIAG_SPEC: SubcommandHelpSpec = SubcommandHelpSpec {
    help_name: "cloudflared tunnel diag",
    usage: "Creates a diagnostic report from a local cloudflared instance",
    usage_text: "cloudflared tunnel [tunnel command options] diag [subcommand options]",
    description: "Creates a diagnostic zip file from a local cloudflared instance.",
    flags: &[
        SubcommandFlagEntry {
            names: "--metrics value",
            usage: "The metrics server address i.e.: 127.0.0.1:12345",
        },
        SubcommandFlagEntry {
            names: "--diag-container-id value",
            usage: "Container ID or Name to collect logs from",
        },
        SubcommandFlagEntry {
            names: "--diag-pod-id value",
            usage: "Kubernetes POD to collect logs from",
        },
        SubcommandFlagEntry {
            names: "--no-diag-logs",
            usage: "Log collection will not be performed (default: false)",
        },
        SubcommandFlagEntry {
            names: "--no-diag-metrics",
            usage: "Metric collection will not be performed (default: false)",
        },
        SubcommandFlagEntry {
            names: "--no-diag-system",
            usage: "System information collection will not be performed (default: false)",
        },
        SubcommandFlagEntry {
            names: "--no-diag-runtime",
            usage: "Runtime information collection will not be performed (default: false)",
        },
        SubcommandFlagEntry {
            names: "--no-diag-network",
            usage: "Network diagnostics won't be performed (default: false)",
        },
    ],
};

const LOGIN_SPEC: SubcommandHelpSpec = SubcommandHelpSpec {
    help_name: "cloudflared tunnel login",
    usage: "Generate a configuration file with your login details",
    usage_text: "cloudflared tunnel [tunnel command options] login [subcommand options]",
    description: "Creates a certificate file, cert.pem, that contains login details for the given \
                  Cloudflare account.",
    flags: &[SubcommandFlagEntry {
        names: "--fedramp",
        usage: "Use FedRAMP-compliant endpoint (default: false)",
    }],
};

const ROUTE_SPEC: SubcommandHelpSpec = SubcommandHelpSpec {
    help_name: "cloudflared tunnel route",
    usage: "Define which traffic routed from Cloudflare edge to this tunnel: requests to a DNS hostname, to \
            a Cloudflare Load Balancer, or traffic originating from Cloudflare WARP clients",
    usage_text: "cloudflared tunnel [tunnel command options] route command [command options] [arguments...]",
    description: "Routes traffic from Cloudflare edge to this tunnel.\n\n   Subcommands:\n     dns     \
                  Route a hostname by creating a DNS CNAME record to a tunnel\n     lb      Use this tunnel \
                  as a load balancer origin\n     ip      Configure and query Cloudflare WARP routing to \
                  private networks",
    flags: &[],
};

const ROUTE_DNS_SPEC: SubcommandHelpSpec = SubcommandHelpSpec {
    help_name: "cloudflared tunnel route dns",
    usage: "Route a hostname by creating a DNS CNAME record to a tunnel",
    usage_text: "cloudflared tunnel route dns [TUNNEL] [HOSTNAME]",
    description: "Creates a DNS CNAME record hostname that points to the tunnel.",
    flags: &[SubcommandFlagEntry {
        names: "--overwrite-dns",
        usage: "Overwrites existing DNS records with this hostname (default: false)",
    }],
};

const ROUTE_LB_SPEC: SubcommandHelpSpec = SubcommandHelpSpec {
    help_name: "cloudflared tunnel route lb",
    usage: "Use this tunnel as a load balancer origin, using a hostname and a load balancer pool name",
    usage_text: "cloudflared tunnel route lb [TUNNEL] [HOSTNAME] [LB-POOL-NAME]",
    description: "Uses this tunnel as a load balancer origin.",
    flags: &[],
};

const ROUTE_IP_SPEC: SubcommandHelpSpec = SubcommandHelpSpec {
    help_name: "cloudflared tunnel route ip",
    usage: "Configure and query Cloudflare WARP routing to private networks on this tunnel",
    usage_text: "cloudflared tunnel route ip command [command options] [arguments...]",
    description: "Manage the private routing table for Cloudflare WARP.\n\n   Subcommands:\n     add     \
                  Add a new network to the routing table\n     show    Show the routing table (alias: \
                  list)\n     delete  Delete a row from the routing table\n     get     Check which route \
                  matches a given IP",
    flags: &[],
};

const VNET_SPEC: SubcommandHelpSpec = SubcommandHelpSpec {
    help_name: "cloudflared tunnel vnet",
    usage: "Configure and query virtual networks to manage private IP routes with overlapping IPs.",
    usage_text: "cloudflared tunnel [tunnel command options] vnet command [command options] [arguments...]",
    description: "Virtual networks are a way to logically separate IP routes to handle overlapping private \
                  IPs.\n\n   Subcommands:\n     add     Add a new virtual network\n     list    Lists the \
                  virtual networks\n     delete  Delete a virtual network\n     update  Update a virtual \
                  network",
    flags: &[],
};

const INGRESS_SPEC: SubcommandHelpSpec = SubcommandHelpSpec {
    help_name: "cloudflared tunnel ingress",
    usage: "Validate and test cloudflared tunnel's ingress configuration",
    usage_text: "cloudflared tunnel [tunnel command options] ingress command [command options] \
                 [arguments...]",
    description: "Test and validate the ingress configuration for a cloudflared tunnel.\n\n   \
                  Subcommands:\n     validate  Validate the ingress configuration\n     rule      Check \
                  which ingress rule matches a given request URL",
    flags: &[],
};

// --- Rendering ---

fn spec_for_target(target: &HelpTarget) -> &'static SubcommandHelpSpec {
    match target {
        HelpTarget::TunnelCreate => &CREATE_SPEC,
        HelpTarget::TunnelList => &LIST_SPEC,
        HelpTarget::TunnelRun => &RUN_SPEC,
        HelpTarget::TunnelDelete => &DELETE_SPEC,
        HelpTarget::TunnelCleanup => &CLEANUP_SPEC,
        HelpTarget::TunnelToken => &TOKEN_SPEC,
        HelpTarget::TunnelInfo => &INFO_SPEC,
        HelpTarget::TunnelReady => &READY_SPEC,
        HelpTarget::TunnelDiag => &DIAG_SPEC,
        HelpTarget::TunnelLogin => &LOGIN_SPEC,
        HelpTarget::TunnelRoute => &ROUTE_SPEC,
        HelpTarget::TunnelRouteDns => &ROUTE_DNS_SPEC,
        HelpTarget::TunnelRouteLb => &ROUTE_LB_SPEC,
        HelpTarget::TunnelRouteIp => &ROUTE_IP_SPEC,
        HelpTarget::TunnelVnet => &VNET_SPEC,
        HelpTarget::TunnelIngress => &INGRESS_SPEC,
        // Root, Tunnel, Access are handled by the caller — never reach here.
        _ => &READY_SPEC,
    }
}

/// Render a flag section with tabwriter-style column alignment.
fn render_flag_section(text: &mut String, heading: &str, flags: &[SubcommandFlagEntry]) {
    if flags.is_empty() {
        return;
    }

    text.push_str(heading);
    text.push('\n');

    let max_name_with_indent = flags.iter().map(|f| f.names.len() + 3).max().unwrap_or(3);
    let column = max_name_with_indent + 2;
    let pad_width = column - 3;

    for flag in flags {
        text.push_str(&format!("   {:<pad_width$}{}\n", flag.names, flag.usage));
    }

    text.push('\n');
}

/// Render per-subcommand help text matching Go baseline
/// `commandHelpTemplate()`.
///
/// Format:
/// ```text
/// NAME:
///    cloudflared tunnel <subcmd> - <Usage>
///
/// USAGE:
///    <UsageText>
///
/// DESCRIPTION:
///    <Description>
///
/// TUNNEL COMMAND OPTIONS:
///    <12 parent flags>
///
/// SUBCOMMAND OPTIONS:
///    <per-subcommand flags>
/// ```
pub fn render_subcommand_help_text(target: &HelpTarget) -> String {
    let spec = spec_for_target(target);
    let mut text = String::with_capacity(2048);

    // NAME
    text.push_str("NAME:\n");
    text.push_str(&format!("   {} - {}\n\n", spec.help_name, spec.usage));

    // USAGE
    text.push_str("USAGE:\n");
    text.push_str(&format!("   {}\n\n", spec.usage_text));

    // DESCRIPTION
    text.push_str("DESCRIPTION:\n");
    text.push_str(&format!("   {}\n\n", spec.description));

    // TUNNEL COMMAND OPTIONS
    render_flag_section(&mut text, "TUNNEL COMMAND OPTIONS:", TUNNEL_COMMAND_OPTIONS);

    // SUBCOMMAND OPTIONS
    render_flag_section(&mut text, "SUBCOMMAND OPTIONS:", spec.flags);

    text
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Structure tests ---

    #[test]
    fn all_subcommand_help_targets_have_specs() {
        let targets = [
            HelpTarget::TunnelCreate,
            HelpTarget::TunnelList,
            HelpTarget::TunnelRun,
            HelpTarget::TunnelDelete,
            HelpTarget::TunnelCleanup,
            HelpTarget::TunnelToken,
            HelpTarget::TunnelInfo,
            HelpTarget::TunnelReady,
            HelpTarget::TunnelDiag,
            HelpTarget::TunnelLogin,
            HelpTarget::TunnelRoute,
            HelpTarget::TunnelRouteDns,
            HelpTarget::TunnelRouteLb,
            HelpTarget::TunnelRouteIp,
            HelpTarget::TunnelVnet,
            HelpTarget::TunnelIngress,
        ];

        for target in &targets {
            let text = render_subcommand_help_text(target);
            assert!(text.contains("NAME:"), "missing NAME section for {target:?}");
            assert!(text.contains("USAGE:"), "missing USAGE section for {target:?}");
            assert!(
                text.contains("DESCRIPTION:"),
                "missing DESCRIPTION section for {target:?}"
            );
            assert!(
                text.contains("TUNNEL COMMAND OPTIONS:"),
                "missing TUNNEL COMMAND OPTIONS section for {target:?}"
            );
        }
    }

    #[test]
    fn tunnel_command_options_has_12_parent_flags() {
        assert_eq!(TUNNEL_COMMAND_OPTIONS.len(), 12);
    }

    #[test]
    fn tunnel_command_options_are_rendered_in_all_subcommands() {
        let text = render_subcommand_help_text(&HelpTarget::TunnelCreate);

        assert!(text.contains("--config value"), "parent flag --config missing");
        assert!(
            text.contains("--origincert value"),
            "parent flag --origincert missing"
        );
        assert!(
            text.contains("--loglevel value"),
            "parent flag --loglevel missing"
        );
        assert!(text.contains("--logfile value"), "parent flag --logfile missing");
    }

    // --- Per-subcommand content tests ---

    #[test]
    fn create_help_has_own_flags() {
        let text = render_subcommand_help_text(&HelpTarget::TunnelCreate);

        assert!(text.contains("SUBCOMMAND OPTIONS:"));
        assert!(text.contains("--output value, -o value"));
        assert!(text.contains("--secret value, -s value"));
        assert!(text.contains("--credentials-file value, --cred-file value"));
    }

    #[test]
    fn create_help_has_correct_name() {
        let text = render_subcommand_help_text(&HelpTarget::TunnelCreate);

        assert!(text.contains("cloudflared tunnel create - Create a new tunnel with given name"));
    }

    #[test]
    fn list_help_has_10_subcommand_flags() {
        let spec = spec_for_target(&HelpTarget::TunnelList);
        assert_eq!(spec.flags.len(), 10);
    }

    #[test]
    fn list_help_has_all_filter_flags() {
        let text = render_subcommand_help_text(&HelpTarget::TunnelList);

        assert!(text.contains("--show-deleted, -d"));
        assert!(text.contains("--name-prefix value, -np value"));
        assert!(text.contains("--exclude-name-prefix value, -enp value"));
        assert!(text.contains("--when value, -w value"));
        assert!(text.contains("--sort-by value"));
        assert!(text.contains("--invert-sort"));
        assert!(text.contains("--show-recently-disconnected, -rd"));
    }

    #[test]
    fn run_help_has_token_flags() {
        let text = render_subcommand_help_text(&HelpTarget::TunnelRun);

        assert!(text.contains("--token value"));
        assert!(text.contains("--token-file value"));
        assert!(text.contains("--credentials-file value, --cred-file value"));
        assert!(text.contains("--credentials-contents value"));
    }

    #[test]
    fn delete_help_has_force_flag() {
        let text = render_subcommand_help_text(&HelpTarget::TunnelDelete);
        assert!(text.contains("--force, -f"));
    }

    #[test]
    fn cleanup_help_has_connector_id_flag() {
        let text = render_subcommand_help_text(&HelpTarget::TunnelCleanup);
        assert!(text.contains("--connector-id value, -c value"));
    }

    #[test]
    fn info_help_has_sort_flags() {
        let text = render_subcommand_help_text(&HelpTarget::TunnelInfo);

        assert!(text.contains("--sort-by value"));
        assert!(text.contains("--invert-sort"));
        assert!(text.contains("--show-recently-disconnected, -rd"));
    }

    #[test]
    fn ready_help_has_no_subcommand_flags() {
        let text = render_subcommand_help_text(&HelpTarget::TunnelReady);
        assert!(!text.contains("SUBCOMMAND OPTIONS:"));
    }

    #[test]
    fn diag_help_has_exclusion_flags() {
        let text = render_subcommand_help_text(&HelpTarget::TunnelDiag);

        assert!(text.contains("--no-diag-logs"));
        assert!(text.contains("--no-diag-metrics"));
        assert!(text.contains("--no-diag-system"));
        assert!(text.contains("--no-diag-runtime"));
        assert!(text.contains("--no-diag-network"));
        assert!(text.contains("--diag-container-id value"));
        assert!(text.contains("--diag-pod-id value"));
    }

    #[test]
    fn login_help_has_fedramp_flag() {
        let text = render_subcommand_help_text(&HelpTarget::TunnelLogin);
        assert!(text.contains("--fedramp"));
    }

    #[test]
    fn route_help_lists_subcommands_in_description() {
        let text = render_subcommand_help_text(&HelpTarget::TunnelRoute);

        assert!(text.contains("dns"));
        assert!(text.contains("lb"));
        assert!(text.contains("ip"));
    }

    #[test]
    fn route_dns_help_has_overwrite_flag() {
        let text = render_subcommand_help_text(&HelpTarget::TunnelRouteDns);
        assert!(text.contains("--overwrite-dns"));
    }

    #[test]
    fn vnet_help_lists_subcommands_in_description() {
        let text = render_subcommand_help_text(&HelpTarget::TunnelVnet);

        assert!(text.contains("add"));
        assert!(text.contains("list"));
        assert!(text.contains("delete"));
        assert!(text.contains("update"));
    }

    #[test]
    fn ingress_help_lists_subcommands_in_description() {
        let text = render_subcommand_help_text(&HelpTarget::TunnelIngress);

        assert!(text.contains("validate"));
        assert!(text.contains("rule"));
    }

    // --- Column alignment tests ---

    #[test]
    fn tunnel_command_options_column_alignment() {
        let text = render_subcommand_help_text(&HelpTarget::TunnelCreate);

        let options_start = text.find("TUNNEL COMMAND OPTIONS:").expect("section missing");
        let section = &text[options_start..];
        let lines: Vec<&str> = section
            .lines()
            .skip(1) // header
            .take_while(|line| !line.is_empty())
            .collect();

        assert!(!lines.is_empty());

        // All description columns should start at the same position.
        let max_name = TUNNEL_COMMAND_OPTIONS
            .iter()
            .map(|f| f.names.len() + 3)
            .max()
            .unwrap_or(3);
        let _expected_col = max_name + 2;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            let indent = line.len() - trimmed.len();

            assert_eq!(
                indent, 3,
                "flag {i} should have 3-space indent, got {indent}: {line:?}"
            );
        }

        // Verify consistent padding: every flag usage text starts at the same column.
        let first_usage_start = lines[0]
            .find(TUNNEL_COMMAND_OPTIONS[0].usage)
            .expect("usage not found");

        for (i, line) in lines.iter().enumerate() {
            let usage_start = line
                .find(TUNNEL_COMMAND_OPTIONS[i].usage)
                .expect("usage not found in line");

            assert_eq!(
                usage_start, first_usage_start,
                "usage column mismatch at flag {i}: expected {first_usage_start}, got {usage_start}"
            );
        }
    }

    #[test]
    fn subcommand_options_column_alignment_for_list() {
        let text = render_subcommand_help_text(&HelpTarget::TunnelList);

        let options_start = text.find("SUBCOMMAND OPTIONS:").expect("section missing");
        let section = &text[options_start..];
        let lines: Vec<&str> = section
            .lines()
            .skip(1)
            .take_while(|line| !line.is_empty())
            .collect();

        assert_eq!(lines.len(), 10);

        // All usage text should start at the same column.
        let first_usage_start = lines[0].find(LIST_SPEC.flags[0].usage).expect("usage not found");

        for (i, line) in lines.iter().enumerate() {
            let usage_start = line.find(LIST_SPEC.flags[i].usage).expect("usage not found");

            assert_eq!(
                usage_start, first_usage_start,
                "usage column mismatch at flag {i}: expected {first_usage_start}, got {usage_start}"
            );
        }
    }
}
