mod defaults;
mod merge;

use super::{IngressFlagRequest, OriginRequestConfig};

pub(super) fn materialize_defaults(raw: &OriginRequestConfig) -> OriginRequestConfig {
    self::defaults::materialize_defaults(raw)
}

pub(super) fn merge_overrides(
    base: &OriginRequestConfig,
    overrides: &OriginRequestConfig,
) -> OriginRequestConfig {
    self::merge::merge_overrides(base, overrides)
}

pub(super) fn flag_defaults(request: &IngressFlagRequest) -> OriginRequestConfig {
    self::defaults::flag_defaults(request)
}
