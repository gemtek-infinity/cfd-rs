use std::collections::BTreeMap;
use std::io::Write;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::ACCEPT;

use super::{
    AddressableTunnelState, ConfigDiagnostics, DiagnosticRunError, TunnelState, find_metrics_server,
};

const DEFAULT_HTTP_TIMEOUT: Duration = Duration::from_secs(15);
const ACCEPT_HEADER_VALUE: &str = "application/json;version=1";

#[derive(Debug, Clone)]
pub struct DiagnosticHttpClient {
    client: Client,
    base_url: Option<String>,
}

pub fn diagnostics_http_client() -> DiagnosticHttpClient {
    let client = Client::builder()
        .timeout(DEFAULT_HTTP_TIMEOUT)
        .build()
        .expect("diagnostic http client should build");

    DiagnosticHttpClient {
        client,
        base_url: None,
    }
}

impl DiagnosticHttpClient {
    pub fn set_base_url(&mut self, base_url: String) {
        self.base_url = Some(base_url);
    }

    pub fn base_url(&self) -> Option<&str> {
        self.base_url.as_deref()
    }

    pub fn get_tunnel_state(&self) -> Result<TunnelState, String> {
        self.get_json("/diag/tunnel")
    }

    pub fn get_log_configuration(&self) -> Result<ConfigDiagnostics, String> {
        let data: BTreeMap<String, String> = self.get_json("/diag/configuration")?;
        Ok(ConfigDiagnostics {
            uid: data.get("uid").cloned().unwrap_or_default(),
            log_file: data.get("logfile").cloned(),
            log_directory: data.get("log-directory").cloned(),
        })
    }

    pub fn copy_bytes(&self, endpoint: &str, writer: &mut dyn Write) -> Result<(), String> {
        let response = self.get(endpoint)?;
        let bytes = response
            .bytes()
            .map_err(|error| format!("diagnostic client error whilst reading response: {error}"))?;
        writer
            .write_all(&bytes)
            .map_err(|error| format!("error writing response: {error}"))
    }

    pub fn copy_pretty_json(&self, endpoint: &str, writer: &mut dyn Write) -> Result<(), String> {
        let response = self.get(endpoint)?;
        let value = response
            .json::<serde_json::Value>()
            .map_err(|error| format!("diagnostic client error whilst reading response: {error}"))?;
        serde_json::to_writer_pretty(&mut *writer, &value)
            .map_err(|error| format!("diagnostic client error whilst writing json: {error}"))?;
        writer
            .write_all(b"\n")
            .map_err(|error| format!("diagnostic client error whilst writing json: {error}"))
    }

    fn get_json<T>(&self, endpoint: &str) -> Result<T, String>
    where
        T: serde::de::DeserializeOwned,
    {
        self.get(endpoint)?
            .json::<T>()
            .map_err(|error| format!("failed to decode body: {error}"))
    }

    fn get(&self, endpoint: &str) -> Result<reqwest::blocking::Response, String> {
        let base_url = self.base_url.as_deref().ok_or_else(|| "no base url".to_owned())?;
        let url = build_url(base_url, endpoint);
        self.client
            .get(url)
            .header(ACCEPT, ACCEPT_HEADER_VALUE)
            .send()
            .map_err(|error| format!("error GET request: {error}"))
    }
}

pub fn probe_metrics_server_tunnel_state(address: &str) -> Option<TunnelState> {
    let mut client = diagnostics_http_client();
    let base_url = normalize_base_url(address).ok()?;
    client.set_base_url(base_url);
    client.get_tunnel_state().ok()
}

pub fn find_metrics_server_http(
    addresses: &[String],
) -> Result<AddressableTunnelState, super::DiscoveryError> {
    find_metrics_server(addresses, probe_metrics_server_tunnel_state)
}

pub(crate) fn normalize_base_url(address: &str) -> Result<String, DiagnosticRunError> {
    if address.starts_with("http://") || address.starts_with("https://") {
        return Ok(address.trim_end_matches('/').to_owned());
    }

    let candidate = format!("http://{address}");
    reqwest::Url::parse(&candidate)
        .map(|_| candidate)
        .map_err(|error| DiagnosticRunError::InvalidAddress(error.to_string()))
}

fn build_url(base_url: &str, endpoint: &str) -> String {
    if endpoint.starts_with('/') {
        format!("{base_url}{endpoint}")
    } else {
        format!("{base_url}/{endpoint}")
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    use super::*;

    fn serve_once(body: &str, content_type: &str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let address = listener.local_addr().expect("addr");
        let body = body.to_owned();
        let content_type = content_type.to_owned();

        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buffer = [0_u8; 2048];
            let _ = stream.read(&mut buffer);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: {content_type}\r\nConnection: \
                 close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).expect("write");
        });

        format!("http://{address}")
    }

    #[test]
    fn normalize_base_url_adds_http_scheme() {
        assert_eq!(
            normalize_base_url("127.0.0.1:20241").expect("normalized"),
            "http://127.0.0.1:20241"
        );
    }

    #[test]
    fn get_tunnel_state_decodes_json() {
        let base_url = serve_once(
            "{\"tunnelID\":\"00000000-0000-0000-0000-000000000000\"}",
            "application/json",
        );
        let mut client = diagnostics_http_client();
        client.set_base_url(base_url);
        let state = client.get_tunnel_state().expect("decode");
        assert_eq!(
            state.tunnel_id.as_deref(),
            Some("00000000-0000-0000-0000-000000000000")
        );
    }

    #[test]
    fn copy_pretty_json_reformats_payload() {
        let base_url = serve_once(
            "{\"uid\":\"1000\",\"logfile\":\"/tmp/test.log\"}",
            "application/json",
        );
        let mut client = diagnostics_http_client();
        client.set_base_url(base_url);
        let mut output = Vec::new();
        client
            .copy_pretty_json("/diag/configuration", &mut output)
            .expect("copy");
        let text = String::from_utf8(output).expect("utf8");
        assert!(text.contains("  \"uid\": \"1000\""));
    }

    #[test]
    fn probe_metrics_server_tunnel_state_returns_none_on_errors() {
        assert!(probe_metrics_server_tunnel_state("127.0.0.1:9").is_none());
    }
}
