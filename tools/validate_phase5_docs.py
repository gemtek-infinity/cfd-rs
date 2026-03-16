#!/usr/bin/env python3
from __future__ import annotations

import csv
import json
import re
import sys
import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
LEDGER_PATHS = [
    REPO_ROOT / "docs/parity/cli/implementation-checklist.md",
    REPO_ROOT / "docs/parity/cdc/implementation-checklist.md",
    REPO_ROOT / "docs/parity/his/implementation-checklist.md",
]
ROADMAP_INDEX_PATH = REPO_ROOT / "docs/phase-5/roadmap-index.csv"
SOURCE_MAP_PATH = REPO_ROOT / "docs/parity/source-map.csv"
STATUS_PATH = REPO_ROOT / "STATUS.md"
CHARTER_PATH = REPO_ROOT / "REWRITE_CHARTER.md"
LOGGING_DOC_PATH = REPO_ROOT / "docs/parity/logging-compatibility.md"
PARITY_README_PATH = REPO_ROOT / "docs/parity/README.md"
JUSTFILE_PATH = REPO_ROOT / "Justfile"
GITIGNORE_PATH = REPO_ROOT / ".gitignore"
SELF_PATH = REPO_ROOT / "tools/validate_phase5_docs.py"
CONTRACT_LITERAL_VALIDATOR_PATH = REPO_ROOT / "tools/validate_contract_literals.py"
MCP_CONFIG_PATH = REPO_ROOT / ".vscode/mcp.json"
AGENTS_PATH = REPO_ROOT / "AGENTS.md"
COPILOT_PATH = REPO_ROOT / ".github/copilot-instructions.md"
ROUTING_PATH = REPO_ROOT / "docs/ai-context-routing.md"
RUST_INSTRUCTIONS_PATH = REPO_ROOT / ".github/instructions/rust.instructions.md"
CONTRIBUTING_PATH = REPO_ROOT / "CONTRIBUTING.md"
README_PATH = REPO_ROOT / "README.md"
DEPENDENCY_POLICY_PATH = REPO_ROOT / "docs/dependency-policy.md"
ROADMAP_PATH = REPO_ROOT / "docs/phase-5/roadmap.md"
MCP_MANIFEST_PATH = REPO_ROOT / "tools/mcp-cfd-rs/Cargo.toml"
PHASE5_HELPERS_PATH = REPO_ROOT / "tools/mcp-cfd-rs/src/phase5/helpers.rs"
REQUIRED_STATUS_HEADINGS = [
    "## Active Snapshot",
    "## Current Reality",
    "## Active Milestone",
    "## Priority Rows",
    "## Architecture Contract",
    "## Canonical Links",
]
STALE_EVIDENCE_PATTERNS = [
    "first-slice evidence exists",
    "partial local tests only",
    "stage-3.1",
    "Stage 3.1",
]
LEGACY_PATTERNS = {
    "FINAL_PLAN.md": "deleted final-plan document is still referenced",
    "FINAL_PHASE.md": "deleted final-phase document is still referenced",
    "docs/status/": "deleted docs/status tree is still referenced",
    "docs/ACTIVE_CONTEXT.md": "removed active-context file is still referenced",
    "get_active_context": "removed MCP tool is still referenced",
    "tools/first_slice_parity.py": "legacy parity harness path is still referenced",
    "tools/first_slice_go_capture": "legacy Go capture path is still referenced",
    "fixtures/first-slice": "legacy fixture path is still referenced",
    "phase_1": "stage-named test artifact is still referenced",
    "baseline-2026.2.0/design-audit/": "historical audit tree is still referenced",
    "baseline-2026.2.0/old-impl": "stale baseline subdirectory path is still referenced",
    "ADR-0006-standard-format-and-workspace-dependency-admission.md": "stale ADR path is still referenced",
}
TEXT_FILE_NAMES = {
    "AGENTS.md",
    "CONTRIBUTING.md",
    "README.md",
    "REWRITE_CHARTER.md",
    "STATUS.md",
    "Justfile",
    "Cargo.toml",
    "Cargo.lock",
}
TEXT_SUFFIXES = {".md", ".rs", ".py", ".go", ".toml", ".yml", ".yaml", ".csv", ".json"}
CRATE_MANIFESTS = {
    "cfdrs-bin": REPO_ROOT / "crates/cfdrs-bin/Cargo.toml",
    "cfdrs-cli": REPO_ROOT / "crates/cfdrs-cli/Cargo.toml",
    "cfdrs-cdc": REPO_ROOT / "crates/cfdrs-cdc/Cargo.toml",
    "cfdrs-his": REPO_ROOT / "crates/cfdrs-his/Cargo.toml",
    "cfdrs-shared": REPO_ROOT / "crates/cfdrs-shared/Cargo.toml",
}
ALLOWED_DEPS = {
    "cfdrs-bin": {"cfdrs-cli", "cfdrs-cdc", "cfdrs-his", "cfdrs-shared"},
    "cfdrs-cli": {"cfdrs-shared"},
    "cfdrs-cdc": {"cfdrs-shared"},
    "cfdrs-his": {"cfdrs-shared"},
    "cfdrs-shared": set(),
}
REQUIRED_CORE_TOOL_SNIPPETS = [
    "status_summary",
    "phase5_priority",
    "next_parity_ticket",
    "parity_row_details",
    "domain_gaps_ranked",
    "baseline_source_mapping",
    "crate_surface_summary",
    "crate_dependency_graph",
]
REQUIRED_COMPACT_ROUTING_SNIPPETS = [
    "get_context_snapshot",
    "get_context_bundle",
    "get_context_brief",
]
REQUIRED_OPERATIONAL_MCP_GUIDANCE = {
    AGENTS_PATH: "debtmap-enabled MCP target",
    COPILOT_PATH: "debtmap-enabled MCP target",
    CONTRIBUTING_PATH: "operational MCP target is debtmap-enabled",
    ROUTING_PATH: "required debtmap-enabled MCP surface",
}
REQUIRED_PUBLIC_RECIPES = {
    "help:",
    "doctor:",
    "fmt:",
    "fmt-check:",
    "validate-governance:",
    "validate-app:",
    "validate-tools:",
    "validate-debtmap:",
    "validate-pr:",
    "mcp-run:",
    "mcp-run-maintenance:",
    "mcp-smoke:",
    "mcp-smoke-maintenance:",
    "debtmap-report:",
    "shared-behavior-capture:",
    "shared-behavior-compare:",
    "preview-test:",
    "preview-build lane:",
    "preview-smoke lane:",
    "preview-package lane:",
    "preview-all lane:",
}
LOGGING_ROWS = {
    "CLI-003",
    "CLI-023",
    "CLI-024",
    "CDC-023",
    "CDC-024",
    "CDC-026",
    "CDC-038",
    "HIS-036",
    "HIS-050",
    "HIS-063",
    "HIS-064",
    "HIS-065",
    "HIS-067",
    "HIS-068",
}
ALLOWED_RUST_STATUS_VALUES = {
    "not audited",
    "audited, absent",
    "audited, partial",
    "audited, parity-backed",
    "audited, intentional divergence",
    "blocked",
}
ALLOWED_EVIDENCE_STATUS_VALUES = {
    "not present",
    "minimal",
    "weak",
    "partial",
    "parity-backed",
    "baseline-backed tests",
    "compare-backed",
    "local tests",
    "not applicable",
}
ALLOWED_DIVERGENCE_STATUS_VALUES = {
    "none recorded",
    "closed",
    "open gap",
    "intentional divergence",
    "unknown",
    "blocked",
    "not applicable",
}
CLOSED_RUST_STATUS_VALUES = {
    "audited, parity-backed",
    "audited, intentional divergence",
}
PARTIAL_RUST_STATUS_VALUES = {"audited, partial"}
NOT_AUDITED_RUST_STATUS_VALUES = {"not audited"}
NOT_APPLICABLE_ROW_IDS = {"HIS-056", "HIS-057"}


def main() -> int:
    errors: list[str] = []
    validate_row_coverage(errors)
    validate_source_map(errors)
    validate_ledger_vocabulary(errors)
    validate_status_contract(errors)
    validate_status_parity_snapshot(errors)
    validate_status_priority_queue(errors)
    validate_parity_readme_summary(errors)
    validate_charter_contract(errors)
    validate_logging_contract(errors)
    validate_evidence_vocabulary(errors)
    validate_mcp_status_vocab(errors)
    validate_legacy_cleanup(errors)
    validate_architecture(errors)
    validate_gcfgr_ignored(errors)
    validate_agent_guidance(errors)
    validate_editor_mcp_config(errors)
    validate_justfile_contract(errors)
    validate_cloudflare_rs_gate(errors)
    validate_markdown_repo_links(errors)
    validate_markdown_link_targets(errors)

    if errors:
        for error in errors:
            print(f"ERROR: {error}")
        return 1

    print("phase5-doc-validation: ok")
    return 0


def validate_row_coverage(errors: list[str]) -> None:
    ledger_row_ids: list[str] = []
    for path in LEDGER_PATHS:
        ledger_row_ids.extend(parse_ledger_row_ids(path))

    duplicates = sorted(
        {row_id for row_id in ledger_row_ids if ledger_row_ids.count(row_id) > 1}
    )
    if duplicates:
        errors.append(f"duplicate ledger row IDs found: {', '.join(duplicates)}")

    with ROADMAP_INDEX_PATH.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))

    csv_row_ids = [row["row_id"].strip() for row in rows if row.get("row_id")]
    csv_duplicates = sorted(
        {row_id for row_id in csv_row_ids if csv_row_ids.count(row_id) > 1}
    )
    if csv_duplicates:
        errors.append(
            f"duplicate roadmap-index row IDs found: {', '.join(csv_duplicates)}"
        )

    missing = sorted(set(ledger_row_ids) - set(csv_row_ids))
    extra = sorted(set(csv_row_ids) - set(ledger_row_ids))

    if missing:
        errors.append(f"roadmap-index missing ledger rows: {', '.join(missing)}")
    if extra:
        errors.append(f"roadmap-index has unknown extra rows: {', '.join(extra)}")

    if len(csv_row_ids) != len(ledger_row_ids):
        errors.append(
            f"roadmap-index row count {len(csv_row_ids)} does not match ledger row count {len(ledger_row_ids)}"
        )


def validate_source_map(errors: list[str]) -> None:
    ledger_row_ids: list[str] = []
    for path in LEDGER_PATHS:
        ledger_row_ids.extend(parse_ledger_row_ids(path))

    with SOURCE_MAP_PATH.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))

    source_row_ids = [row["row_id"].strip() for row in rows if row.get("row_id")]
    duplicates = sorted(
        {row_id for row_id in source_row_ids if source_row_ids.count(row_id) > 1}
    )
    if duplicates:
        errors.append(f"duplicate source-map row IDs found: {', '.join(duplicates)}")

    missing = sorted(set(ledger_row_ids) - set(source_row_ids))
    extra = sorted(set(source_row_ids) - set(ledger_row_ids))
    if missing:
        errors.append(f"source-map missing ledger rows: {', '.join(missing)}")
    if extra:
        errors.append(f"source-map has unknown extra rows: {', '.join(extra)}")

    for row in rows:
        row_id = row["row_id"].strip()
        feature_doc = row["feature_doc"].strip()
        baseline_paths = [
            value.strip() for value in row["baseline_paths"].split(";") if value.strip()
        ]
        symbol_hints = [
            value.strip() for value in row["symbol_hints"].split(";") if value.strip()
        ]

        if not feature_doc:
            errors.append(f"source-map row {row_id} is missing feature_doc")
        elif not (REPO_ROOT / feature_doc).exists():
            errors.append(
                f"source-map row {row_id} points to missing feature_doc: {feature_doc}"
            )

        if not baseline_paths:
            errors.append(f"source-map row {row_id} has no baseline_paths")
        for baseline_path in baseline_paths:
            resolved = REPO_ROOT / baseline_path
            if not baseline_path.startswith("baseline-2026.2.0/"):
                errors.append(
                    f"source-map row {row_id} has non-baseline path: {baseline_path}"
                )
            elif not resolved.exists() or not resolved.is_file():
                errors.append(
                    f"source-map row {row_id} points to missing baseline file: {baseline_path}"
                )

        if not symbol_hints:
            errors.append(f"source-map row {row_id} has no symbol_hints")


def validate_ledger_vocabulary(errors: list[str]) -> None:
    for path in LEDGER_PATHS:
        text = path.read_text(encoding="utf-8")
        rows = parse_ledger_rows(path)

        if any(
            row["parity_evidence_status"] == "not applicable"
            or row["divergence_status"] == "not applicable"
            for row in rows
        ):
            if text.count("- not applicable") < 2:
                errors.append(
                    f"{path.relative_to(REPO_ROOT)} must document 'not applicable' in the evidence and divergence vocab sections"
                )

        for row in rows:
            row_id = row["row_id"]
            rust_status = row["rust_status_now"]
            evidence_status = row["parity_evidence_status"]
            divergence_status = row["divergence_status"]

            if rust_status not in ALLOWED_RUST_STATUS_VALUES:
                errors.append(
                    f"{path.relative_to(REPO_ROOT)} row {row_id} has undeclared Rust status now value: {rust_status}"
                )

            if evidence_status not in ALLOWED_EVIDENCE_STATUS_VALUES:
                errors.append(
                    f"{path.relative_to(REPO_ROOT)} row {row_id} has undeclared Parity evidence status value: {evidence_status}"
                )

            if divergence_status not in ALLOWED_DIVERGENCE_STATUS_VALUES:
                errors.append(
                    f"{path.relative_to(REPO_ROOT)} row {row_id} has undeclared Divergence status value: {divergence_status}"
                )

            if (
                evidence_status == "not applicable"
                or divergence_status == "not applicable"
            ) and row_id not in NOT_APPLICABLE_ROW_IDS:
                errors.append(
                    f"{path.relative_to(REPO_ROOT)} row {row_id} uses 'not applicable' outside the allowed packaging-only rows"
                )


def validate_status_parity_snapshot(errors: list[str]) -> None:
    section = extract_section(STATUS_PATH.read_text(encoding="utf-8"), "## Parity Snapshot")
    if section is None:
        return

    table = parse_markdown_table(section)
    if not table:
        errors.append("STATUS.md Parity Snapshot table is missing or malformed")
        return

    expected = expected_status_parity_snapshot()
    actual = {normalize_table_domain_name(row.get("Domain", "")): row for row in table}

    for domain, summary in expected.items():
        row = actual.get(domain)
        if row is None:
            errors.append(f"STATUS.md Parity Snapshot is missing row for {domain}")
            continue

        expected_values = {
            "Total": str(summary["total"]),
            "Closed": str(summary["closed"]),
            "Partial": str(summary["partial"]),
            "Not audited": str(summary["not_audited"]),
            "% Closed": f"{summary['percent_closed']}%",
        }
        for column, expected_value in expected_values.items():
            actual_value = row.get(column, "").replace("*", "").strip()
            if actual_value != expected_value:
                errors.append(
                    f"STATUS.md Parity Snapshot {domain} {column} expected {expected_value} but found {row.get(column)}"
                )


def validate_status_priority_queue(errors: list[str]) -> None:
    section = extract_section(STATUS_PATH.read_text(encoding="utf-8"), "## Priority Rows")
    if section is None:
        return

    ledger_rows = {row["row_id"]: row for row in parse_all_ledger_rows()}
    seen: set[str] = set()

    for line_number, entry in parse_status_priority_entries(section):
        row_source, _summary = split_priority_line(entry)
        row_ids = re.findall(r"(?:CLI|CDC|HIS)-\d{3}", row_source)
        if not row_ids:
            errors.append(
                f"STATUS.md Priority Rows entry at relative line {line_number} must contain at least one row ID"
            )
            continue

        for row_id in row_ids:
            if row_id in seen:
                errors.append(f"STATUS.md Priority Rows repeats row ID: {row_id}")
                continue
            seen.add(row_id)

            row = ledger_rows.get(row_id)
            if row is None:
                errors.append(f"STATUS.md Priority Rows references unknown row ID: {row_id}")
                continue

            if row["rust_status_now"] in CLOSED_RUST_STATUS_VALUES:
                errors.append(f"STATUS.md Priority Rows references already closed row: {row_id}")


def validate_parity_readme_summary(errors: list[str]) -> None:
    section = extract_section(PARITY_README_PATH.read_text(encoding="utf-8"), "## Cross-Domain Summary")
    if section is None:
        return

    table = parse_markdown_table(section)
    if not table:
        errors.append("docs/parity/README.md Cross-Domain Summary table is missing or malformed")
        return

    expected = expected_parity_readme_summary()
    actual = {normalize_table_domain_name(row.get("Domain", "")): row for row in table}

    for domain, summary in expected.items():
        row = actual.get(domain)
        if row is None:
            errors.append(f"docs/parity/README.md Cross-Domain Summary is missing row for {domain}")
            continue

        expected_values = {
            "Rows": str(summary["rows"]),
            "Critical": str(summary["critical"]),
            "High": str(summary["high"]),
        }
        for column, expected_value in expected_values.items():
            if row.get(column) != expected_value:
                errors.append(
                    f"docs/parity/README.md Cross-Domain Summary {domain} {column} expected {expected_value} but found {row.get(column)}"
                )


def validate_mcp_status_vocab(errors: list[str]) -> None:
    text = PHASE5_HELPERS_PATH.read_text(encoding="utf-8")
    for required_symbol in (
        "CLOSED_RUST_STATUSES",
        "PARTIAL_RUST_STATUSES",
        "NOT_AUDITED_RUST_STATUSES",
    ):
        if required_symbol not in text:
            errors.append(
                f"tools/mcp-cfd-rs/src/phase5/helpers.rs must define {required_symbol}"
            )

    for value in sorted(CLOSED_RUST_STATUS_VALUES | PARTIAL_RUST_STATUS_VALUES | NOT_AUDITED_RUST_STATUS_VALUES):
        if value not in text:
            errors.append(
                f"tools/mcp-cfd-rs/src/phase5/helpers.rs must keep canonical status literal: {value}"
            )


def validate_status_contract(errors: list[str]) -> None:
    text = STATUS_PATH.read_text(encoding="utf-8")
    for heading in REQUIRED_STATUS_HEADINGS:
        if heading not in text:
            errors.append(f"STATUS.md is missing required heading: {heading}")

    active_snapshot = extract_section(text, "## Active Snapshot")
    if active_snapshot is None:
        return

    if len(active_snapshot) > 2500:
        errors.append("STATUS.md Active Snapshot exceeds 2500 characters")

    non_empty_lines = [line for line in active_snapshot.splitlines() if line.strip()]
    if len(non_empty_lines) > 18:
        errors.append("STATUS.md Active Snapshot exceeds 18 non-empty lines")

    if "production-alpha logging blocker set" not in active_snapshot:
        errors.append(
            "STATUS.md Active Snapshot must call out the production-alpha logging blocker set"
        )


def validate_charter_contract(errors: list[str]) -> None:
    text = CHARTER_PATH.read_text(encoding="utf-8")

    if "Source-of-truth routing" in text:
        errors.append("REWRITE_CHARTER.md must not contain a routing/index section")
    if "REWRITE_CHARTER.md" in text:
        errors.append("REWRITE_CHARTER.md must not self-reference")
    if "design-audit" in text:
        errors.append("REWRITE_CHARTER.md must not reference design-audit")


def validate_logging_contract(errors: list[str]) -> None:
    text = LOGGING_DOC_PATH.read_text(encoding="utf-8")
    for row_id in sorted(LOGGING_ROWS):
        if row_id not in text:
            errors.append(f"docs/parity/logging-compatibility.md must mention {row_id}")

    for owner in ("cfdrs-cli", "cfdrs-his", "cfdrs-cdc"):
        if owner not in text:
            errors.append(
                f"docs/parity/logging-compatibility.md must mention owner {owner}"
            )

    if "production-alpha blocker" not in text:
        errors.append(
            "docs/parity/logging-compatibility.md must say logging is a production-alpha blocker"
        )

    roadmap_text = ROADMAP_PATH.read_text(encoding="utf-8")
    status_text = STATUS_PATH.read_text(encoding="utf-8")
    if "logging blocker" not in roadmap_text.lower():
        errors.append(
            "docs/phase-5/roadmap.md must keep the logging blocker set explicit"
        )
    if "production-alpha logging blocker set" not in status_text:
        errors.append("STATUS.md must keep the logging blocker set explicit")


def validate_evidence_vocabulary(errors: list[str]) -> None:
    for path in LEDGER_PATHS:
        text = path.read_text(encoding="utf-8")
        for pattern in STALE_EVIDENCE_PATTERNS:
            if pattern in text:
                errors.append(
                    f"stale evidence vocabulary '{pattern}' still present in {path.relative_to(REPO_ROOT)}"
                )


def validate_legacy_cleanup(errors: list[str]) -> None:
    for path in iter_repo_text_files():
        if path == SELF_PATH:
            continue

        text = path.read_text(encoding="utf-8")
        for pattern, message in LEGACY_PATTERNS.items():
            if pattern in text:
                errors.append(f"{message}: {path.relative_to(REPO_ROOT)}")


def validate_architecture(errors: list[str]) -> None:
    for crate_name, manifest_path in CRATE_MANIFESTS.items():
        with manifest_path.open("rb") as handle:
            manifest = tomllib.load(handle)

        for section_name in ("dependencies", "build-dependencies"):
            section = manifest.get(section_name, {})
            for dependency_name in section:
                if dependency_name not in CRATE_MANIFESTS:
                    continue
                if dependency_name not in ALLOWED_DEPS[crate_name]:
                    errors.append(
                        f"forbidden workspace dependency {crate_name} -> {dependency_name} in [{section_name}]"
                    )


def validate_gcfgr_ignored(errors: list[str]) -> None:
    gitignore = GITIGNORE_PATH.read_text(encoding="utf-8")
    if "GCFGR.md" not in gitignore.splitlines():
        errors.append(".gitignore must ignore GCFGR.md")


def validate_agent_guidance(errors: list[str]) -> None:
    for path in (AGENTS_PATH, COPILOT_PATH, CONTRIBUTING_PATH, RUST_INSTRUCTIONS_PATH):
        text = path.read_text(encoding="utf-8")
        if "cargo +nightly fmt" not in text:
            errors.append(
                f"{path.relative_to(REPO_ROOT)} must require cargo +nightly fmt"
            )

    for path in (
        AGENTS_PATH,
        COPILOT_PATH,
        CONTRIBUTING_PATH,
        ROUTING_PATH,
        README_PATH,
    ):
        text = path.read_text(encoding="utf-8")
        if "Justfile" not in text and "just " not in text:
            errors.append(
                f"{path.relative_to(REPO_ROOT)} must route normal execution through Justfile"
            )

    for path in (AGENTS_PATH, COPILOT_PATH, ROUTING_PATH):
        text = path.read_text(encoding="utf-8")
        for snippet in REQUIRED_CORE_TOOL_SNIPPETS:
            if snippet not in text:
                errors.append(
                    f"{path.relative_to(REPO_ROOT)} is missing MCP startup-tool guidance for {snippet}"
                )

    for path in (AGENTS_PATH, COPILOT_PATH):
        text = path.read_text(encoding="utf-8")
        for snippet in REQUIRED_COMPACT_ROUTING_SNIPPETS:
            if snippet not in text:
                errors.append(
                    f"{path.relative_to(REPO_ROOT)} is missing compact-routing guidance for {snippet}"
                )

    for path, snippet in REQUIRED_OPERATIONAL_MCP_GUIDANCE.items():
        text = path.read_text(encoding="utf-8")
        if snippet not in text:
            errors.append(
                f"{path.relative_to(REPO_ROOT)} must describe the debtmap-enabled operational MCP surface"
            )


def validate_editor_mcp_config(errors: list[str]) -> None:
    config = json.loads(MCP_CONFIG_PATH.read_text(encoding="utf-8"))
    servers = config.get("servers", {})
    core = servers.get("cfd-rs-memory")
    if not isinstance(core, dict):
        errors.append(".vscode/mcp.json must define the cfd-rs-memory server")
        return

    if core.get("command") != "just":
        errors.append(".vscode/mcp.json must launch cfd-rs-memory via just")

    args = core.get("args")
    if args != ["mcp-run"]:
        errors.append(".vscode/mcp.json cfd-rs-memory args must be ['mcp-run']")

    env = core.get("env", {})
    if env.get("MCP_LOG") != "brief":
        errors.append(".vscode/mcp.json cfd-rs-memory must set MCP_LOG=brief")

    with MCP_MANIFEST_PATH.open("rb") as handle:
        manifest = tomllib.load(handle)

    feature_table = manifest.get("features", {})
    default_features = feature_table.get("default", [])
    if "debtmap" not in default_features:
        errors.append(
            "tools/mcp-cfd-rs/Cargo.toml must keep debtmap in the default feature set"
        )


def validate_justfile_contract(errors: list[str]) -> None:
    if not JUSTFILE_PATH.exists():
        errors.append("Justfile is missing")
        return

    text = JUSTFILE_PATH.read_text(encoding="utf-8")
    if "cargo +nightly fmt --all" not in text:
        errors.append("Justfile fmt recipe must run cargo +nightly fmt --all")
    if "cargo +nightly fmt --all --check" not in text:
        errors.append(
            "Justfile fmt-check recipe must run cargo +nightly fmt --all --check"
        )

    public_headers = set()
    recipe_header = re.compile(r"^([A-Za-z0-9_-]+(?:\s+[A-Za-z0-9_-]+)?)\s*:")
    for line in text.splitlines():
        if line.startswith((" ", "\t", "#")) or not line.strip():
            continue
        if line.startswith(("set ", "alias ", "export ")):
            continue
        match = recipe_header.match(line)
        if not match:
            continue
        header = f"{match.group(1)}:"
        if header.startswith("_"):
            continue
        public_headers.add(header)

    if public_headers != REQUIRED_PUBLIC_RECIPES:
        missing = sorted(REQUIRED_PUBLIC_RECIPES - public_headers)
        extra = sorted(public_headers - REQUIRED_PUBLIC_RECIPES)
        if missing:
            errors.append(f"Justfile is missing public recipes: {', '.join(missing)}")
        if extra:
            errors.append(f"Justfile has unexpected public recipes: {', '.join(extra)}")

    if not CONTRACT_LITERAL_VALIDATOR_PATH.exists():
        errors.append("tools/validate_contract_literals.py is missing")


def validate_cloudflare_rs_gate(errors: list[str]) -> None:
    text = DEPENDENCY_POLICY_PATH.read_text(encoding="utf-8")
    for snippet in ("cloudflare-rs", "CDC-033", "CDC-034", "CDC-038", "no admission"):
        if snippet not in text:
            errors.append(
                f"docs/dependency-policy.md must keep the cloudflare-rs gate snippet: {snippet}"
            )


def validate_markdown_repo_links(errors: list[str]) -> None:
    for path in markdown_paths():
        for line_number, line in enumerate(
            iter_markdown_non_fence_lines(path), start=1
        ):
            for match in re.finditer(r"`([^`]+)`", line):
                start, end = match.span()
                if start > 0 and line[start - 1] == "[" and line[end : end + 2] == "](":
                    continue
                candidate = match.group(1)
                target = resolve_linkable_repo_target(path, candidate)
                if target is None:
                    continue
                errors.append(
                    f"{path.relative_to(REPO_ROOT)}:{line_number} has repo path in code span without markdown link: {candidate}"
                )


def validate_markdown_link_targets(errors: list[str]) -> None:
    link_pattern = re.compile(r"\[[^\]]+\]\(([^)]+)\)")
    for path in markdown_paths():
        for line_number, line in enumerate(
            iter_markdown_non_fence_lines(path), start=1
        ):
            for match in link_pattern.finditer(line):
                href = match.group(1)
                if "://" in href or href.startswith("#"):
                    continue
                href_path = href.split("#", 1)[0]
                target = (path.parent / href_path).resolve()
                if is_repo_path(target):
                    continue
                errors.append(
                    f"{path.relative_to(REPO_ROOT)}:{line_number} links to non-repo path: {href}"
                )


def parse_ledger_row_ids(path: Path) -> list[str]:
    text = path.read_text(encoding="utf-8")
    return re.findall(r"^\|\s*((?:CLI|CDC|HIS)-\d{3})\s*\|", text, flags=re.MULTILINE)


def parse_all_ledger_rows() -> list[dict[str, str]]:
    rows: list[dict[str, str]] = []
    for path in LEDGER_PATHS:
        rows.extend(parse_ledger_rows(path))
    return rows


def parse_ledger_rows(path: Path) -> list[dict[str, str]]:
    rows: list[dict[str, str]] = []
    current_section = ""
    lines = path.read_text(encoding="utf-8").splitlines()
    index = 0

    while index < len(lines):
        line = lines[index].strip()

        if line.startswith("### "):
            current_section = line.removeprefix("### ").strip()
            index += 1
            continue

        if line.startswith("|"):
            header = split_markdown_row(line)
            has_divider = (
                index + 1 < len(lines)
                and is_markdown_divider(split_markdown_row(lines[index + 1].strip()))
            )
            if header and header[0] == "ID" and has_divider:
                index += 2
                while index < len(lines):
                    row_line = lines[index].strip()
                    if not row_line.startswith("|"):
                        break
                    cells = split_markdown_row(row_line)
                    if len(cells) != len(header):
                        break
                    row = dict(zip(header, cells))
                    row_id = row.get("ID", "").strip()
                    if re.fullmatch(r"(?:CLI|CDC|HIS)-\d{3}", row_id):
                        rows.append(
                            {
                                "row_id": row_id,
                                "domain": row_id.split("-", 1)[0],
                                "section": current_section,
                                "rust_status_now": row.get("Rust status now", "").strip(),
                                "parity_evidence_status": row.get("Parity evidence status", "").strip(),
                                "divergence_status": row.get("Divergence status", "").strip(),
                                "priority": row.get("Priority", "").strip(),
                            }
                        )
                    index += 1
                continue

        index += 1

    return rows


def split_markdown_row(line: str) -> list[str]:
    return [cell.strip() for cell in line.strip().strip("|").split("|")]


def is_markdown_divider(cells: list[str]) -> bool:
    return bool(cells) and all(
        cell and all(ch in "-: " for ch in cell) for cell in cells
    )


def parse_markdown_table(section: str) -> list[dict[str, str]]:
    lines = [line.strip() for line in section.splitlines() if line.strip().startswith("|")]
    if len(lines) < 2:
        return []

    header = split_markdown_row(lines[0])
    divider = split_markdown_row(lines[1])
    if not is_markdown_divider(divider):
        return []

    rows: list[dict[str, str]] = []
    for line in lines[2:]:
        cells = split_markdown_row(line)
        if len(cells) != len(header):
            continue
        rows.append(dict(zip(header, cells)))
    return rows


def parse_status_priority_entries(section: str) -> list[tuple[int, str]]:
    entries: list[tuple[int, str]] = []
    current_line: int | None = None
    current_parts: list[str] = []

    for line_number, line in enumerate(section.splitlines(), start=1):
        stripped = line.strip()
        if not stripped:
            continue

        match = re.match(r"^(\d+)\.\s+(.*)$", stripped)
        if match:
            if current_line is not None:
                entries.append((current_line, " ".join(current_parts)))
            current_line = line_number
            current_parts = [match.group(2).strip()]
            continue

        if current_line is not None:
            current_parts.append(stripped)

    if current_line is not None:
        entries.append((current_line, " ".join(current_parts)))

    return entries


def split_priority_line(text: str) -> tuple[str, str]:
    if " — " in text:
        return tuple(text.split(" — ", 1))
    if " - " in text:
        return tuple(text.split(" - ", 1))
    return text, ""


def normalize_table_domain_name(value: str) -> str:
    return value.replace("*", "").strip()


def expected_status_parity_snapshot() -> dict[str, dict[str, int]]:
    rows = parse_all_ledger_rows()
    summaries: dict[str, dict[str, int]] = {}

    for domain in ("CLI", "CDC", "HIS"):
        domain_rows = [row for row in rows if row["domain"] == domain]
        closed = sum(
            1 for row in domain_rows if row["rust_status_now"] in CLOSED_RUST_STATUS_VALUES
        )
        partial = sum(
            1 for row in domain_rows if row["rust_status_now"] in PARTIAL_RUST_STATUS_VALUES
        )
        not_audited = sum(
            1
            for row in domain_rows
            if row["rust_status_now"] in NOT_AUDITED_RUST_STATUS_VALUES
        )
        total = len(domain_rows)
        summaries[domain] = {
            "total": total,
            "closed": closed,
            "partial": partial,
            "not_audited": not_audited,
            "percent_closed": round((closed / total) * 100) if total else 0,
        }

    total_rows = sum(summary["total"] for summary in summaries.values())
    total_closed = sum(summary["closed"] for summary in summaries.values())
    total_partial = sum(summary["partial"] for summary in summaries.values())
    total_not_audited = sum(summary["not_audited"] for summary in summaries.values())
    summaries["Total"] = {
        "total": total_rows,
        "closed": total_closed,
        "partial": total_partial,
        "not_audited": total_not_audited,
        "percent_closed": round((total_closed / total_rows) * 100) if total_rows else 0,
    }
    return summaries


def expected_parity_readme_summary() -> dict[str, dict[str, int]]:
    rows = parse_all_ledger_rows()
    summaries: dict[str, dict[str, int]] = {}

    for domain in ("CLI", "CDC", "HIS"):
        domain_rows = [row for row in rows if row["domain"] == domain]
        summaries[domain] = {
            "rows": len(domain_rows),
            "critical": sum(1 for row in domain_rows if row["priority"] == "critical"),
            "high": sum(1 for row in domain_rows if row["priority"] == "high"),
        }

    summaries["Total"] = {
        "rows": sum(summary["rows"] for summary in summaries.values()),
        "critical": sum(summary["critical"] for summary in summaries.values()),
        "high": sum(summary["high"] for summary in summaries.values()),
    }
    return summaries


def extract_section(text: str, heading: str) -> str | None:
    lines = text.splitlines()
    capture = False
    captured: list[str] = []

    for line in lines:
        if not capture:
            if line.strip() == heading:
                capture = True
            continue

        if line.startswith("## "):
            break
        captured.append(line)

    if not capture:
        return None
    return "\n".join(captured).strip()


def markdown_paths() -> list[Path]:
    return sorted(
        path for path in REPO_ROOT.rglob("*.md") if "baseline-2026.2.0" not in str(path)
    )


def iter_markdown_non_fence_lines(path: Path) -> list[str]:
    lines = path.read_text(encoding="utf-8").splitlines()
    in_fence = False
    visible_lines: list[str] = []
    for line in lines:
        if line.lstrip().startswith("```"):
            in_fence = not in_fence
            visible_lines.append("")
            continue
        visible_lines.append("" if in_fence else line)
    return visible_lines


def resolve_linkable_repo_target(
    path: Path,
    candidate: str,
) -> Path | None:
    if not is_linkable_repo_candidate(candidate):
        return None
    for target in (
        (path.parent / candidate).resolve(),
        (REPO_ROOT / candidate).resolve(),
    ):
        if is_repo_path(target):
            return target
    return None


def is_repo_path(target: Path) -> bool:
    try:
        target.relative_to(REPO_ROOT)
    except ValueError:
        return False

    if any(part in {".git", "target"} for part in target.parts):
        return False

    return target.exists()


def is_linkable_repo_candidate(candidate: str) -> bool:
    if "://" in candidate or candidate.startswith(("/", "--")):
        return False
    if " " in candidate or any(char in candidate for char in "*{}"):
        return False
    if candidate.startswith(("CLI-", "CDC-", "HIS-")):
        return False
    # GCFGR.md is gitignored and must not be linked.
    if candidate == "GCFGR.md":
        return False
    return (
        "/" in candidate
        or candidate.endswith(
            (".md", ".csv", ".toml", ".json", ".yml", ".yaml", ".rs", ".go")
        )
        or candidate in {"Justfile", "Cargo.toml", "Cargo.lock"}
    )


def iter_repo_text_files() -> list[Path]:
    files: list[Path] = []
    for path in REPO_ROOT.rglob("*"):
        if not path.is_file():
            continue
        if any(part in {".git", "target", "baseline-2026.2.0"} for part in path.parts):
            continue
        if path.name in TEXT_FILE_NAMES or path.suffix in TEXT_SUFFIXES:
            files.append(path)
    return files


if __name__ == "__main__":
    sys.exit(main())
