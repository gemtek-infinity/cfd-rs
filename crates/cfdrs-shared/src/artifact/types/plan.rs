use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct EmissionPlan {
    pub repo_root: PathBuf,
    pub fixture_root: PathBuf,
    pub output_dir: PathBuf,
    pub fixtures: Vec<FixtureSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FixtureSpec {
    pub fixture_id: String,
    pub category: String,
    pub comparison: String,
    pub input: String,
    pub source_refs: Vec<String>,
    #[serde(default)]
    pub discovery_case: Option<DiscoveryCase>,
    #[serde(default)]
    pub origin_cert_source: Option<String>,
    #[serde(default)]
    pub ordering_case: Option<OrderingCase>,
    #[serde(default)]
    pub flag_ingress_case: Option<FlagIngressCase>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiscoveryCase {
    pub explicit_config: bool,
    pub present: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrderingCase {
    pub input: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FlagIngressCase {
    pub flags: Vec<String>,
}
