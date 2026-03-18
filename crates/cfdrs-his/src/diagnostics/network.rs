use std::collections::BTreeMap;
use std::process::Command;

use serde::{Deserialize, Serialize};

use super::DIAGNOSTIC_REGIONS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkHop {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hop: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rtts: Option<Vec<u64>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkTrace {
    pub hops: Vec<NetworkHop>,
    pub raw: String,
    pub error: Option<String>,
}

pub fn collect_network_traces() -> BTreeMap<String, NetworkTrace> {
    collect_network_traces_with(run_traceroute)
}

fn collect_network_traces_with<F>(runner: F) -> BTreeMap<String, NetworkTrace>
where
    F: Fn(&str, bool) -> NetworkTrace,
{
    let mut traces = BTreeMap::new();

    for region in DIAGNOSTIC_REGIONS {
        traces.insert(format!("{region}-v4"), runner(region, true));
        traces.insert(format!("{region}-v6"), runner(region, false));
    }

    traces
}

fn run_traceroute(hostname: &str, use_v4: bool) -> NetworkTrace {
    let command = if use_v4 { "traceroute" } else { "traceroute6" };
    let args = ["-I", "-w", "5", "-m", "5", hostname];

    match Command::new(command).args(args).output() {
        Ok(output) => decode_trace_output(
            String::from_utf8_lossy(&output.stdout).to_string(),
            output.status.success(),
            output.status.to_string(),
        ),
        Err(error) => NetworkTrace {
            hops: Vec::new(),
            raw: String::new(),
            error: Some(format!(
                "error retrieving output from command '{command}': {error}"
            )),
        },
    }
}

fn decode_trace_output(raw: String, success: bool, status: String) -> NetworkTrace {
    let hops = raw.lines().filter_map(decode_line).collect::<Vec<_>>();
    let error = if success {
        None
    } else {
        Some(format!("traceroute command exited with status {status}"))
    };

    NetworkTrace { hops, raw, error }
}

fn decode_line(text: &str) -> Option<NetworkHop> {
    let fields: Vec<&str> = text.split_whitespace().collect();
    let hop = fields.first()?.parse::<u8>().ok()?;

    let filtered = fields
        .iter()
        .skip(1)
        .copied()
        .filter(|field| *field != "ms" && *field != "*")
        .collect::<Vec<_>>();

    if filtered.is_empty() {
        return Some(NetworkHop {
            hop: Some(hop),
            domain: Some("*".to_owned()),
            rtts: None,
        });
    }

    let mut domain_parts = Vec::new();
    let mut rtts = Vec::new();

    for field in filtered {
        match field.parse::<f64>() {
            Ok(rtt) => rtts.push((rtt * 1000.0) as u64),
            Err(_) => domain_parts.push(field),
        }
    }

    if domain_parts.is_empty() {
        return None;
    }

    Some(NetworkHop {
        hop: Some(hop),
        domain: Some(domain_parts.join(" ")),
        rtts: if rtts.is_empty() { None } else { Some(rtts) },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_timeout_hop_matches_go_shape() {
        let hop = decode_line("3 * * *").expect("timeout hop");
        assert_eq!(hop.hop, Some(3));
        assert_eq!(hop.domain.as_deref(), Some("*"));
        assert_eq!(hop.rtts, None);
    }

    #[test]
    fn decode_normal_hop_matches_go_shape() {
        let hop = decode_line("1 192.0.2.1 1.234 ms 2.345 ms").expect("hop");
        assert_eq!(hop.hop, Some(1));
        assert_eq!(hop.domain.as_deref(), Some("192.0.2.1"));
        assert_eq!(hop.rtts, Some(vec![1234, 2345]));
    }

    #[test]
    fn collect_network_traces_emits_four_named_targets() {
        let traces = collect_network_traces_with(|host, use_v4| NetworkTrace {
            hops: vec![NetworkHop {
                hop: Some(1),
                domain: Some(host.to_owned()),
                rtts: if use_v4 {
                    Some(vec![1000])
                } else {
                    Some(vec![2000])
                },
            }],
            raw: format!("{host} raw"),
            error: None,
        });
        assert_eq!(traces.len(), 4);
        assert!(traces.contains_key("region1.v2.argotunnel.com-v4"));
        assert!(traces.contains_key("region1.v2.argotunnel.com-v6"));
        assert!(traces.contains_key("region2.v2.argotunnel.com-v4"));
        assert!(traces.contains_key("region2.v2.argotunnel.com-v6"));
    }
}
