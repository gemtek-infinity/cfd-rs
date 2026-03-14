use super::{CfdRsMemory, ContextBundleRequest, ContextSnapshotRequest, Parameters, log, profile, to_json};
use rmcp::{tool, tool_router};

#[tool_router(router = context_tools_router, vis = "pub")]
impl CfdRsMemory {
    #[tool(description = "Return a curated narrow context bundle for a common repository question type.")]
    async fn get_context_bundle(
        &self,
        Parameters(ContextBundleRequest { bundle }): Parameters<ContextBundleRequest>,
    ) -> String {
        let span = log::ToolSpan::start("get_context_bundle");
        match profile::bundle(bundle.trim()) {
            Some(bundle) => {
                span.done(&format!(
                    "bundle={} entries={}",
                    bundle.bundle,
                    bundle.entries.len()
                ));
                to_json(bundle)
            }
            None => {
                span.error("unknown bundle");
                to_json(serde_json::json!({
                    "error": "unknown bundle",
                    "supported_bundles": profile::supported_bundle_names()
                }))
            }
        }
    }

    #[tool(description = "Return a compact first-read brief for a curated repository context bundle.")]
    async fn get_context_brief(
        &self,
        Parameters(ContextBundleRequest { bundle }): Parameters<ContextBundleRequest>,
    ) -> String {
        let span = log::ToolSpan::start("get_context_brief");
        match profile::brief(bundle.trim()) {
            Some(brief) => {
                span.done(&format!("bundle={}", brief.bundle));
                to_json(brief)
            }
            None => {
                span.error("unknown bundle");
                to_json(serde_json::json!({
                    "error": "unknown bundle",
                    "supported_bundles": profile::supported_bundle_names()
                }))
            }
        }
    }

    #[tool(description = "Return a compact source-backed snapshot for a core rewrite routing question.")]
    async fn get_context_snapshot(
        &self,
        Parameters(ContextSnapshotRequest { snapshot }): Parameters<ContextSnapshotRequest>,
    ) -> String {
        let span = log::ToolSpan::start("get_context_snapshot");
        match profile::snapshot(snapshot.trim()) {
            Some(snapshot) => {
                span.done(&format!(
                    "snapshot={} facts={}",
                    snapshot.snapshot,
                    snapshot.facts.len()
                ));
                to_json(snapshot)
            }
            None => {
                span.error("unknown snapshot");
                to_json(serde_json::json!({
                    "error": "unknown snapshot",
                    "supported_snapshots": profile::supported_snapshot_names()
                }))
            }
        }
    }
}
