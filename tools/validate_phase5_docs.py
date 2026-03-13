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
STATUS_PATH = REPO_ROOT / "STATUS.md"
GITIGNORE_PATH = REPO_ROOT / ".gitignore"
SELF_PATH = REPO_ROOT / "tools/validate_phase5_docs.py"
MCP_CONFIG_PATH = REPO_ROOT / ".vscode/mcp.json"
AGENTS_PATH = REPO_ROOT / "AGENTS.md"
COPILOT_PATH = REPO_ROOT / ".github/copilot-instructions.md"
ROUTING_PATH = REPO_ROOT / "docs/ai-context-routing.md"
RUST_INSTRUCTIONS_PATH = REPO_ROOT / ".github/instructions/rust.instructions.md"
CONTRIBUTING_PATH = REPO_ROOT / "CONTRIBUTING.md"
MCP_MANIFEST_PATH = REPO_ROOT / "tools/mcp-cfd-rs/Cargo.toml"
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
}
TEXT_FILE_NAMES = {
    "AGENTS.md",
    "CONTRIBUTING.md",
    "README.md",
    "REWRITE_CHARTER.md",
    "STATUS.md",
    "Cargo.toml",
    "Cargo.lock",
}
TEXT_SUFFIXES = {".md", ".rs", ".py", ".go", ".toml", ".yml", ".yaml", ".csv"}
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


def main() -> int:
    errors: list[str] = []
    validate_row_coverage(errors)
    validate_status_contract(errors)
    validate_evidence_vocabulary(errors)
    validate_legacy_cleanup(errors)
    validate_architecture(errors)
    validate_gcfgr_ignored(errors)
    validate_agent_guidance(errors)
    validate_editor_mcp_config(errors)

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

    duplicates = sorted({row_id for row_id in ledger_row_ids if ledger_row_ids.count(row_id) > 1})
    if duplicates:
        errors.append(f"duplicate ledger row IDs found: {', '.join(duplicates)}")

    with ROADMAP_INDEX_PATH.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))

    csv_row_ids = [row["row_id"].strip() for row in rows if row.get("row_id")]
    csv_duplicates = sorted({row_id for row_id in csv_row_ids if csv_row_ids.count(row_id) > 1})
    if csv_duplicates:
        errors.append(f"duplicate roadmap-index row IDs found: {', '.join(csv_duplicates)}")

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


def validate_status_contract(errors: list[str]) -> None:
    text = STATUS_PATH.read_text(encoding="utf-8")
    for heading in REQUIRED_STATUS_HEADINGS:
        if heading not in text:
            errors.append(f"STATUS.md is missing required heading: {heading}")

    active_snapshot = extract_section(text, "## Active Snapshot")
    if active_snapshot is None:
        return

    if len(active_snapshot) > 2000:
        errors.append("STATUS.md Active Snapshot exceeds 2000 characters")

    non_empty_lines = [line for line in active_snapshot.splitlines() if line.strip()]
    if len(non_empty_lines) > 16:
        errors.append("STATUS.md Active Snapshot exceeds 16 non-empty lines")


def validate_evidence_vocabulary(errors: list[str]) -> None:
    for path in LEDGER_PATHS:
        text = path.read_text(encoding="utf-8")
        for pattern in STALE_EVIDENCE_PATTERNS:
            if pattern in text:
                errors.append(f"stale evidence vocabulary '{pattern}' still present in {path.relative_to(REPO_ROOT)}")


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
            errors.append(f"{path.relative_to(REPO_ROOT)} must require cargo +nightly fmt")

    for path in (AGENTS_PATH, COPILOT_PATH, ROUTING_PATH):
        text = path.read_text(encoding="utf-8")
        for snippet in REQUIRED_CORE_TOOL_SNIPPETS:
            if snippet not in text:
                errors.append(f"{path.relative_to(REPO_ROOT)} is missing MCP startup-tool guidance for {snippet}")

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
            errors.append(f"{path.relative_to(REPO_ROOT)} must describe the debtmap-enabled operational MCP surface")


def validate_editor_mcp_config(errors: list[str]) -> None:
    config = json.loads(MCP_CONFIG_PATH.read_text(encoding="utf-8"))
    servers = config.get("servers", {})
    core = servers.get("cfd-rs-memory")
    if not isinstance(core, dict):
        errors.append(".vscode/mcp.json must define the cfd-rs-memory server")
        return

    if core.get("command") != "cargo":
        errors.append(".vscode/mcp.json must launch cfd-rs-memory via cargo")

    args = core.get("args")
    if not isinstance(args, list):
        errors.append(".vscode/mcp.json cfd-rs-memory args must be a list")
        return

    for required_arg in ("run", "--locked", "--quiet", "--release", "--features", "debtmap"):
        if required_arg not in args:
            errors.append(f".vscode/mcp.json cfd-rs-memory args must include {required_arg}")

    if "--no-default-features" in args:
        errors.append(".vscode/mcp.json must not start cfd-rs-memory with --no-default-features")

    manifest_path = "${workspaceFolder}/tools/mcp-cfd-rs/Cargo.toml"
    if "--manifest-path" not in args or manifest_path not in args:
        errors.append(".vscode/mcp.json must point cfd-rs-memory at tools/mcp-cfd-rs/Cargo.toml")

    env = core.get("env", {})
    if env.get("MCP_LOG") != "brief":
        errors.append(".vscode/mcp.json cfd-rs-memory must set MCP_LOG=brief")

    with MCP_MANIFEST_PATH.open("rb") as handle:
        manifest = tomllib.load(handle)

    feature_table = manifest.get("features", {})
    default_features = feature_table.get("default", [])
    if "debtmap" not in default_features:
        errors.append("tools/mcp-cfd-rs/Cargo.toml must keep debtmap in the default feature set")


def parse_ledger_row_ids(path: Path) -> list[str]:
    text = path.read_text(encoding="utf-8")
    return re.findall(r"^\|\s*((?:CLI|CDC|HIS)-\d{3})\s*\|", text, flags=re.MULTILINE)


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
