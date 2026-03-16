use super::{
    BaselineSourceMappingRequest, CfdRsMemory, DomainGapsRankedRequest, EmptyRequest,
    NextParityTicketRequest, Parameters, ParityRowDetailsRequest, log, phase5, to_json,
};
use rmcp::{tool, tool_router};

#[tool_router(router = phase5_tools_router, vis = "pub")]
impl CfdRsMemory {
    #[tool(
        description = "Return the current tracked status summary from STATUS.md, including per-domain \
                       parity progress (closed, partial, absent counts for CLI, CDC, HIS)."
    )]
    async fn status_summary(&self, Parameters(EmptyRequest {}): Parameters<EmptyRequest>) -> String {
        let span = log::ToolSpan::start("status_summary");

        match phase5::status_summary(&self.repo_root) {
            Ok(summary) => {
                span.done(&format!("milestone={}", summary.active_milestone));
                to_json(summary)
            }
            Err(error) => {
                span.error(&error);
                to_json(serde_json::json!({ "error": error }))
            }
        }
    }

    #[tool(description = "Return the current Phase 5 priority queue and active milestone detail.")]
    async fn phase5_priority(&self, Parameters(EmptyRequest {}): Parameters<EmptyRequest>) -> String {
        let span = log::ToolSpan::start("phase5_priority");

        match phase5::phase5_priority(&self.repo_root) {
            Ok(priority) => {
                span.done(&format!("milestone={}", priority.active_milestone.name));
                to_json(priority)
            }
            Err(error) => {
                span.error(&error);
                to_json(serde_json::json!({ "error": error }))
            }
        }
    }

    #[tool(
        description = "Return the next parity row-ID work item, using STATUS.md priority rows first and \
                       then blocker-aware gap ranking."
    )]
    async fn next_parity_ticket(
        &self,
        Parameters(NextParityTicketRequest {
            domain,
            include_blocked,
        }): Parameters<NextParityTicketRequest>,
    ) -> String {
        let span = log::ToolSpan::start("next_parity_ticket");
        let include_blocked = include_blocked.unwrap_or(false);

        match phase5::next_parity_ticket(&self.repo_root, domain.as_deref(), include_blocked) {
            Ok(ticket) => {
                span.done(&format!("row_id={} domain={}", ticket.row_id, ticket.domain));
                to_json(ticket)
            }
            Err(error) => {
                span.error(&error);
                to_json(serde_json::json!({
                    "error": error,
                    "domain": domain,
                    "include_blocked": include_blocked,
                }))
            }
        }
    }

    #[tool(description = "Return combined ledger and roadmap detail for one exact parity row ID.")]
    async fn parity_row_details(
        &self,
        Parameters(ParityRowDetailsRequest { row_id }): Parameters<ParityRowDetailsRequest>,
    ) -> String {
        let span = log::ToolSpan::start("parity_row_details");

        match phase5::parity_row_details(&self.repo_root, &row_id) {
            Ok(details) => {
                span.done(&format!(
                    "row_id={} milestone={}",
                    details.row_id, details.roadmap.milestone
                ));
                to_json(details)
            }
            Err(error) => {
                span.error(&error);
                to_json(serde_json::json!({ "error": error, "row_id": row_id }))
            }
        }
    }

    #[tool(
        description = "Return ranked open gaps for one parity domain with partial vs absent breakdown, \
                       without loading all ledgers together."
    )]
    async fn domain_gaps_ranked(
        &self,
        Parameters(DomainGapsRankedRequest { domain, limit }): Parameters<DomainGapsRankedRequest>,
    ) -> String {
        let span = log::ToolSpan::start("domain_gaps_ranked");
        let limit = limit.unwrap_or(10).clamp(1, 50) as usize;

        match phase5::domain_gaps_ranked(&self.repo_root, &domain, limit) {
            Ok(ranked) => {
                span.done(&format!("domain={} rows={}", ranked.domain, ranked.rows.len()));
                to_json(ranked)
            }
            Err(error) => {
                span.error(&error);
                to_json(serde_json::json!({ "error": error, "domain": domain }))
            }
        }
    }

    #[tool(
        description = "Map one parity row ID back to frozen baseline source files, symbol hints, and the \
                       exact parity feature doc."
    )]
    async fn baseline_source_mapping(
        &self,
        Parameters(BaselineSourceMappingRequest { row_id }): Parameters<BaselineSourceMappingRequest>,
    ) -> String {
        let span = log::ToolSpan::start("baseline_source_mapping");

        match phase5::baseline_source_mapping(&self.repo_root, &row_id) {
            Ok(mapping) => {
                span.done(&format!(
                    "row_id={} paths={}",
                    mapping.row_id,
                    mapping.baseline_paths.len()
                ));
                to_json(mapping)
            }
            Err(error) => {
                span.error(&error);
                to_json(serde_json::json!({ "error": error, "row_id": row_id }))
            }
        }
    }
}
