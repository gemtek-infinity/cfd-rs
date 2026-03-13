pub(crate) const HTTP_METHOD_KEY: &str = "HttpMethod";
pub(crate) const HTTP_HOST_KEY: &str = "HttpHost";
pub(crate) const HTTP_HEADER_KEY: &str = "HttpHeader";
pub(crate) const HTTP_STATUS_KEY: &str = "HttpStatus";
pub(crate) const FLOW_ID_KEY: &str = "FlowID";
pub(crate) const CF_TRACE_ID_KEY: &str = "cf-trace-id";
#[cfg(test)]
pub(crate) const CONTENT_LENGTH_KEY: &str = "HttpHeader:Content-Length";
pub(crate) const TRACE_CONTEXT_KEY: &str = "cf-trace-context";
pub(crate) const DEFAULT_HTTP_METHOD: &str = "GET";
pub(crate) const HTTP_LABEL: &str = "HTTP";
pub(crate) const WEBSOCKET_LABEL: &str = "WebSocket";
pub(crate) const TCP_LABEL: &str = "TCP";
const HEADER_SEPARATOR: &str = ":";

pub(crate) fn header_metadata_key(name: &str) -> String {
    format!("{HTTP_HEADER_KEY}{HEADER_SEPARATOR}{name}")
}

pub(crate) fn header_metadata_prefix() -> String {
    header_metadata_key("")
}
