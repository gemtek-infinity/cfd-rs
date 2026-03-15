//! Feature flags advertised during tunnel registration.
//!
//! These strings are sent to the edge in `ClientInfo.features` and control
//! which protocol capabilities the connector advertises.
//!
//! Baseline truth: `baseline-2026.2.0/features/features.go`

// ---------------------------------------------------------------------------
// Always-on features (sent on every registration)
// ---------------------------------------------------------------------------

/// Edge may push configuration changes remotely.
pub const ALLOW_REMOTE_CONFIG: &str = "allow_remote_config";

/// HTTP headers are base64-serialized in metadata rather than sent inline.
pub const SERIALIZED_HEADERS: &str = "serialized_headers";

/// Connector supports datagram V2 for UDP/ICMP.
pub const DATAGRAM_V2: &str = "support_datagram_v2";

/// Connector supports QUIC stream EOF signalling.
pub const QUIC_SUPPORT_EOF: &str = "support_quic_eof";

/// Connector supports management log streaming.
pub const MANAGEMENT_LOGS: &str = "management_logs";

// ---------------------------------------------------------------------------
// Selector-enabled features (set by user flags or edge config)
// ---------------------------------------------------------------------------

/// Connector advertises post-quantum key agreement support.
pub const POST_QUANTUM: &str = "postquantum";

/// Connector supports quick reconnects.
pub const QUICK_RECONNECTS: &str = "quick_reconnects";

/// Connector supports datagram V3.2 (current V3 variant).
pub const DATAGRAM_V3_2: &str = "support_datagram_v3_2";

// ---------------------------------------------------------------------------
// Deprecated features (filtered out before sending)
// ---------------------------------------------------------------------------

/// Deprecated: replaced by `support_datagram_v3_2`. TUN-9291.
pub const DEPRECATED_DATAGRAM_V3: &str = "support_datagram_v3";

/// Deprecated: replaced by `support_datagram_v3_2`. TUN-9883.
pub const DEPRECATED_DATAGRAM_V3_1: &str = "support_datagram_v3_1";

const DEPRECATED_FEATURES: &[&str] = &[DEPRECATED_DATAGRAM_V3, DEPRECATED_DATAGRAM_V3_1];

/// Default feature list sent on every registration.
///
/// Matches Go's `defaultFeatures` in `features/features.go`.
pub fn default_feature_list() -> Vec<String> {
    vec![
        ALLOW_REMOTE_CONFIG.to_owned(),
        SERIALIZED_HEADERS.to_owned(),
        DATAGRAM_V2.to_owned(),
        QUIC_SUPPORT_EOF.to_owned(),
        MANAGEMENT_LOGS.to_owned(),
    ]
}

/// Dedup and remove deprecated features, matching Go's
/// `dedupAndRemoveFeatures`.
pub fn dedup_and_filter(features: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();

    for feature in features {
        if DEPRECATED_FEATURES.contains(&feature.as_str()) {
            continue;
        }

        if seen.insert(feature.clone()) {
            result.push(feature.clone());
        }
    }

    result
}

/// Build a registration feature list based on runtime capabilities.
///
/// Starts from the always-on defaults, adds `support_datagram_v3_2`
/// when V3 datagram support is active, adds `postquantum` when
/// post-quantum mode is requested, and then deduplicates / filters
/// deprecated entries.
///
/// Matches Go's `FeatureSnapshot.FeaturesList` construction in
/// `features/features.go`.
pub fn build_feature_list(datagram_v3: bool, post_quantum: bool) -> Vec<String> {
    let mut features = default_feature_list();

    if datagram_v3 {
        features.push(DATAGRAM_V3_2.to_owned());
    }

    if post_quantum {
        features.push(POST_QUANTUM.to_owned());
    }

    dedup_and_filter(&features)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_features_match_baseline() {
        let defaults = default_feature_list();
        assert_eq!(defaults.len(), 5);
        assert!(defaults.contains(&ALLOW_REMOTE_CONFIG.to_owned()));
        assert!(defaults.contains(&SERIALIZED_HEADERS.to_owned()));
        assert!(defaults.contains(&DATAGRAM_V2.to_owned()));
        assert!(defaults.contains(&QUIC_SUPPORT_EOF.to_owned()));
        assert!(defaults.contains(&MANAGEMENT_LOGS.to_owned()));
    }

    #[test]
    fn dedup_removes_deprecated_features() {
        let input = vec![
            ALLOW_REMOTE_CONFIG.to_owned(),
            DEPRECATED_DATAGRAM_V3.to_owned(),
            DEPRECATED_DATAGRAM_V3_1.to_owned(),
            DATAGRAM_V3_2.to_owned(),
        ];
        let filtered = dedup_and_filter(&input);

        assert_eq!(filtered.len(), 2);
        assert!(filtered.contains(&ALLOW_REMOTE_CONFIG.to_owned()));
        assert!(filtered.contains(&DATAGRAM_V3_2.to_owned()));
    }

    #[test]
    fn dedup_removes_duplicates() {
        let input = vec![
            ALLOW_REMOTE_CONFIG.to_owned(),
            ALLOW_REMOTE_CONFIG.to_owned(),
            SERIALIZED_HEADERS.to_owned(),
        ];
        let filtered = dedup_and_filter(&input);

        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn build_feature_list_defaults_only() {
        let features = build_feature_list(false, false);
        assert_eq!(features.len(), 5);
        assert!(features.contains(&DATAGRAM_V2.to_owned()));
        assert!(!features.contains(&DATAGRAM_V3_2.to_owned()));
        assert!(!features.contains(&POST_QUANTUM.to_owned()));
    }

    #[test]
    fn build_feature_list_with_datagram_v3() {
        let features = build_feature_list(true, false);
        assert!(features.contains(&DATAGRAM_V3_2.to_owned()));
        assert!(features.contains(&DATAGRAM_V2.to_owned()));
        assert_eq!(features.len(), 6);
    }

    #[test]
    fn build_feature_list_with_post_quantum() {
        let features = build_feature_list(false, true);
        assert!(features.contains(&POST_QUANTUM.to_owned()));
        assert!(!features.contains(&DATAGRAM_V3_2.to_owned()));
        assert_eq!(features.len(), 6);
    }

    #[test]
    fn build_feature_list_with_all_selectors() {
        let features = build_feature_list(true, true);
        assert!(features.contains(&DATAGRAM_V3_2.to_owned()));
        assert!(features.contains(&POST_QUANTUM.to_owned()));
        assert_eq!(features.len(), 7);
    }
}
