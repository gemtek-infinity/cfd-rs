use super::super::super::{
    DEFAULT_HTTP_CONNECT_TIMEOUT, DEFAULT_KEEP_ALIVE_TIMEOUT, DEFAULT_PROXY_ADDRESS, DEFAULT_TCP_KEEP_ALIVE,
    DEFAULT_TLS_TIMEOUT, DurationSpec,
};

pub(super) fn default_http_connect_timeout() -> Option<DurationSpec> {
    Some(DurationSpec(DEFAULT_HTTP_CONNECT_TIMEOUT.to_owned()))
}

pub(super) fn default_tls_timeout() -> Option<DurationSpec> {
    Some(DurationSpec(DEFAULT_TLS_TIMEOUT.to_owned()))
}

pub(super) fn default_tcp_keep_alive() -> Option<DurationSpec> {
    Some(DurationSpec(DEFAULT_TCP_KEEP_ALIVE.to_owned()))
}

pub(super) fn default_keep_alive_timeout() -> Option<DurationSpec> {
    Some(DurationSpec(DEFAULT_KEEP_ALIVE_TIMEOUT.to_owned()))
}

pub(super) fn default_proxy_address() -> Option<String> {
    Some(DEFAULT_PROXY_ADDRESS.to_owned())
}
