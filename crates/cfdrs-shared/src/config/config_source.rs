use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Where a resolved configuration came from.
///
/// This is a cross-domain type used by both HIS (discovery workflow) and
/// shared config normalization, so it lives in cfdrs-shared rather than
/// cfdrs-his.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum ConfigSource {
    ExplicitPath(PathBuf),
    DiscoveredPath(PathBuf),
    AutoCreatedPath(PathBuf),
}

impl fmt::Display for ConfigSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExplicitPath(_) => f.write_str("explicit-path"),
            Self::DiscoveredPath(_) => f.write_str("discovered-path"),
            Self::AutoCreatedPath(_) => f.write_str("auto-created-path"),
        }
    }
}
