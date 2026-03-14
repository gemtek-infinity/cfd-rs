#!/usr/bin/env python3
from __future__ import annotations

import csv
import re
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
BASELINE_ROOT = REPO_ROOT / "baseline-2026.2.0"
OUTPUT_PATH = REPO_ROOT / "docs" / "parity" / "source-map.csv"

LEDGERS = {
    "CLI": REPO_ROOT / "docs" / "parity" / "cli" / "implementation-checklist.md",
    "CDC": REPO_ROOT / "docs" / "parity" / "cdc" / "implementation-checklist.md",
    "HIS": REPO_ROOT / "docs" / "parity" / "his" / "implementation-checklist.md",
}

FEATURE_DOCS = {
    "CLI": {
        "Root And Global Surface": "docs/parity/cli/root-and-global-flags.md",
        "Tunnel Command Surface": "docs/parity/cli/tunnel-subtree.md",
        "Access, Tail, And Management Surface": "docs/parity/cli/access-subtree.md",
        "Compatibility, Formatting, And Error Behavior": "docs/parity/cli/implementation-checklist.md",
        "Transitional Rust-Only Commands": "docs/parity/cli/implementation-checklist.md",
    },
    "CDC": {
        "Registration RPC": "docs/parity/cdc/registration-rpc.md",
        "Stream Contracts": "docs/parity/cdc/stream-contracts.md",
        "Control Stream And Lifecycle": "docs/parity/cdc/stream-contracts.md",
        "Management And Log Streaming": "docs/parity/cdc/management-and-diagnostics.md",
        "Metrics And Readiness": "docs/parity/cdc/metrics-readiness-and-api.md",
        "Cloudflare REST API": "docs/parity/cdc/metrics-readiness-and-api.md",
        "Datagram And UDP": "docs/parity/cdc/stream-contracts.md",
        "Token And Credential Encoding": "docs/parity/cdc/implementation-checklist.md",
        "QUIC Transport Wire Contract": "docs/parity/cdc/stream-contracts.md",
    },
    "HIS": {
        "Config Discovery and Loading": "docs/parity/his/filesystem-and-layout.md",
        "Credentials and Lookup": "docs/parity/his/filesystem-and-layout.md",
        "Service Installation and Uninstall": "docs/parity/his/service-installation.md",
        "Systemd and Init System": "docs/parity/his/service-installation.md",
        "Local HTTP Endpoints": "docs/parity/his/diagnostics-and-collection.md",
        "Diagnostics Collection": "docs/parity/his/diagnostics-and-collection.md",
        "Watcher and Config Reload": "docs/parity/his/reload-and-watcher.md",
        "Updater": "docs/parity/his/implementation-checklist.md",
        "Environment and Privilege": "docs/parity/his/implementation-checklist.md",
        "Deployment Evidence": "docs/parity/his/implementation-checklist.md",
        "Package Manager Scripts": "docs/parity/his/implementation-checklist.md",
        "Signal Handling and Graceful Shutdown": "docs/parity/his/reload-and-watcher.md",
        "Logging and File Artifacts": "docs/parity/logging-compatibility.md",
        "ICMP and Raw Sockets": "docs/parity/his/implementation-checklist.md",
        "Local Test Server": "docs/parity/his/implementation-checklist.md",
        "Process Restart": "docs/parity/his/implementation-checklist.md",
    },
}

ROW_FEATURE_DOC_OVERRIDES = {
    "CLI-003": "docs/parity/logging-compatibility.md",
    "CLI-023": "docs/parity/logging-compatibility.md",
    "CLI-024": "docs/parity/logging-compatibility.md",
    "CDC-023": "docs/parity/logging-compatibility.md",
    "CDC-024": "docs/parity/logging-compatibility.md",
    "CDC-026": "docs/parity/logging-compatibility.md",
    "CDC-038": "docs/parity/logging-compatibility.md",
    "HIS-036": "docs/parity/logging-compatibility.md",
    "HIS-050": "docs/parity/logging-compatibility.md",
    "HIS-063": "docs/parity/logging-compatibility.md",
    "HIS-064": "docs/parity/logging-compatibility.md",
    "HIS-065": "docs/parity/logging-compatibility.md",
    "HIS-067": "docs/parity/logging-compatibility.md",
    "HIS-068": "docs/parity/logging-compatibility.md",
}

SECTION_FALLBACKS = {
    "CLI": {
        "Root And Global Surface": [
            "cmd/cloudflared/main.go",
            "cmd/cloudflared/tunnel/cmd.go",
            "logger/configuration.go",
        ],
        "Tunnel Command Surface": [
            "cmd/cloudflared/tunnel/cmd.go",
            "cmd/cloudflared/tunnel/subcommands.go",
            "cmd/cloudflared/tunnel/vnets_subcommands.go",
            "cmd/cloudflared/tunnel/ingress_subcommands.go",
            "cmd/cloudflared/tunnel/login.go",
        ],
        "Access, Tail, And Management Surface": [
            "cmd/cloudflared/access/cmd.go",
            "cmd/cloudflared/tail/cmd.go",
            "cmd/cloudflared/management/cmd.go",
        ],
        "Compatibility, Formatting, And Error Behavior": [
            "cmd/cloudflared/main.go",
            "cmd/cloudflared/proxydns/cmd.go",
            "cmd/cloudflared/tunnel/cmd.go",
        ],
        "Transitional Rust-Only Commands": [
            "cmd/cloudflared/main.go",
            "cmd/cloudflared/tunnel/subcommands.go",
        ],
    },
    "CDC": {
        "Registration RPC": [
            "tunnelrpc/proto/tunnelrpc.capnp",
            "tunnelrpc/registration_client.go",
            "tunnelrpc/pogs/registration_server.go",
        ],
        "Stream Contracts": [
            "tunnelrpc/proto/quic_metadata_protocol.capnp",
            "tunnelrpc/pogs/quic_metadata_protocol.go",
            "connection/header.go",
        ],
        "Control Stream And Lifecycle": [
            "connection/control.go",
            "connection/event.go",
            "connection/protocol.go",
            "edgediscovery/allregions/discovery.go",
            "edgediscovery/allregions/region.go",
        ],
        "Management And Log Streaming": [
            "management/service.go",
            "management/middleware.go",
            "management/events.go",
            "management/session.go",
            "management/token.go",
        ],
        "Metrics And Readiness": [
            "metrics/metrics.go",
            "metrics/readiness.go",
        ],
        "Cloudflare REST API": [
            "cfapi/base_client.go",
            "cfapi/client.go",
            "cfapi/tunnel.go",
            "cfapi/ip_route.go",
            "cfapi/virtual_network.go",
            "cfapi/hostname.go",
        ],
        "Datagram And UDP": [
            "datagramsession/manager.go",
            "connection/quic_datagram_v2.go",
            "connection/quic_datagram_v3.go",
            "quic/v3/manager.go",
        ],
        "Token And Credential Encoding": [
            "connection/connection.go",
            "credentials/origin_cert.go",
        ],
        "QUIC Transport Wire Contract": [
            "connection/protocol.go",
            "connection/quic.go",
            "quic/constants.go",
        ],
    },
    "HIS": {
        "Config Discovery and Loading": [
            "config/configuration.go",
            "config/manager.go",
        ],
        "Credentials and Lookup": [
            "credentials/credentials.go",
            "credentials/origin_cert.go",
            "connection/connection.go",
            "cmd/cloudflared/tunnel/credential_finder.go",
        ],
        "Service Installation and Uninstall": [
            "cmd/cloudflared/linux_service.go",
            "cmd/cloudflared/common_service.go",
            "cmd/cloudflared/service_template.go",
        ],
        "Systemd and Init System": [
            "cmd/cloudflared/linux_service.go",
            "cmd/cloudflared/service_template.go",
        ],
        "Local HTTP Endpoints": [
            "metrics/metrics.go",
            "metrics/readiness.go",
            "diagnostic/handlers.go",
        ],
        "Diagnostics Collection": [
            "diagnostic/diagnostic.go",
            "diagnostic/handlers.go",
            "diagnostic/log_collector_host.go",
            "diagnostic/system_collector_linux.go",
            "diagnostic/network/collector_unix.go",
        ],
        "Watcher and Config Reload": [
            "watcher/file.go",
            "cmd/cloudflared/app_service.go",
            "overwatch/app_manager.go",
            "orchestration/orchestrator.go",
        ],
        "Updater": [
            "cmd/cloudflared/updater/update.go",
        ],
        "Environment and Privilege": [
            "diagnostic/handlers.go",
            "ingress/icmp_linux.go",
            "cmd/cloudflared/updater/update.go",
        ],
        "Deployment Evidence": [
            "cmd/cloudflared/linux_service.go",
            "metrics/metrics.go",
        ],
        "Package Manager Scripts": [
            "postinst.sh",
            "postrm.sh",
        ],
        "Signal Handling and Graceful Shutdown": [
            "signal/safe_signal.go",
            "cmd/cloudflared/tunnel/signal.go",
            "token/token.go",
        ],
        "Logging and File Artifacts": [
            "logger/configuration.go",
            "logger/create.go",
            "diagnostic/log_collector_host.go",
            "management/service.go",
            "management/events.go",
            "management/token.go",
        ],
        "ICMP and Raw Sockets": [
            "ingress/icmp_linux.go",
        ],
        "Local Test Server": [
            "hello/hello.go",
            "ingress/origin_service.go",
        ],
        "Process Restart": [
            "metrics/metrics.go",
            "cmd/cloudflared/updater/update.go",
            "vendor/github.com/facebookgo/grace/gracenet/net.go",
        ],
    },
}

ROW_FALLBACKS = {
    "CLI-025": ["cmd/cloudflared/proxydns/cmd.go", "cmd/cloudflared/tunnel/cmd.go"],
    "CLI-026": ["cmd/cloudflared/tunnel/cmd.go"],
    "CLI-028": ["cmd/cloudflared/main.go", "cmd/cloudflared/tunnel/login.go"],
    "CLI-029": ["cmd/cloudflared/main.go", "cmd/cloudflared/tunnel/cmd.go"],
    "CLI-030": ["cmd/cloudflared/main.go"],
    "CLI-031": ["cmd/cloudflared/main.go"],
    "CLI-032": ["cmd/cloudflared/tunnel/cmd.go", "cmd/cloudflared/tunnel/subcommands.go"],
    "CDC-042": ["connection/connection.go"],
    "CDC-043": ["credentials/origin_cert.go"],
    "HIS-036": ["diagnostic/log_collector_host.go", "logger/create.go"],
    "HIS-046": ["cmd/cloudflared/updater/update.go"],
    "HIS-047": ["cmd/cloudflared/updater/update.go"],
    "HIS-048": ["cmd/cloudflared/updater/update.go"],
    "HIS-049": ["cmd/cloudflared/updater/update.go"],
    "HIS-053": ["cmd/cloudflared/linux_service.go", "metrics/metrics.go"],
    "HIS-054": ["cmd/cloudflared/main.go"],
    "HIS-055": ["cmd/cloudflared/linux_service.go"],
    "HIS-058": ["signal/safe_signal.go"],
    "HIS-059": ["cmd/cloudflared/tunnel/cmd.go", "signal/safe_signal.go"],
    "HIS-060": ["cmd/cloudflared/tunnel/signal.go"],
    "HIS-061": ["cmd/cloudflared/tunnel/cmd.go"],
    "HIS-062": ["token/token.go", "token/path.go"],
    "HIS-063": ["logger/create.go"],
    "HIS-064": ["logger/create.go", "config/configuration.go"],
    "HIS-065": ["logger/create.go"],
    "HIS-066": ["logger/create.go"],
    "HIS-067": ["logger/configuration.go"],
    "HIS-068": ["logger/configuration.go", "management/events.go"],
    "HIS-069": ["ingress/icmp_linux.go"],
    "HIS-070": ["ingress/icmp_linux.go"],
    "HIS-071": ["cmd/cloudflared/tunnel/configuration.go", "ingress/icmp_linux.go"],
    "HIS-072": ["hello/hello.go", "ingress/origin_service.go"],
    "HIS-073": ["metrics/metrics.go", "vendor/github.com/facebookgo/grace/gracenet/net.go"],
    "HIS-074": ["cmd/cloudflared/updater/update.go", "vendor/github.com/facebookgo/grace/gracenet/net.go"],
}

PATH_TOKEN_RE = re.compile(r"[A-Za-z0-9_./-]+(?:\.(?:go|capnp|sh|pem))")
FLAG_RE = re.compile(r"--[a-z0-9-]+")
ENV_RE = re.compile(r"\bTUNNEL_[A-Z0-9_]+\b")
ROUTE_RE = re.compile(r"/(?:[A-Za-z][A-Za-z0-9_{}.*-]+/?)+")
IDENT_RE = re.compile(r"\b[A-Za-z][A-Za-z0-9_]*(?:\(\))?\b")


@dataclass
class LedgerRow:
    row_id: str
    domain: str
    section: str
    feature_group: str
    baseline_source: str
    behavior: str
    notes: str


def main() -> int:
    rows = []
    for domain, path in LEDGERS.items():
        rows.extend(parse_ledger(path, domain))

    OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    with OUTPUT_PATH.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(
            handle,
            fieldnames=["row_id", "domain", "feature_doc", "baseline_paths", "symbol_hints"],
        )
        writer.writeheader()
        for row in rows:
            feature_doc = ROW_FEATURE_DOC_OVERRIDES.get(
                row.row_id,
                FEATURE_DOCS[row.domain].get(row.section, f"docs/parity/{row.domain.lower()}/implementation-checklist.md"),
            )
            baseline_paths = resolve_baseline_paths(row)
            symbol_hints = extract_symbol_hints(row)
            writer.writerow(
                {
                    "row_id": row.row_id,
                    "domain": row.domain,
                    "feature_doc": feature_doc,
                    "baseline_paths": ";".join(baseline_paths),
                    "symbol_hints": ";".join(symbol_hints),
                }
            )
    print(OUTPUT_PATH.relative_to(REPO_ROOT))
    return 0


def parse_ledger(path: Path, domain: str) -> list[LedgerRow]:
    text = path.read_text(encoding="utf-8")
    rows: list[LedgerRow] = []
    section = ""

    for line in text.splitlines():
        if line.startswith("### "):
            section = line[4:].strip()
            continue

        if not re.match(rf"^\|\s*{domain}-\d{{3}}\s*\|", line):
            continue

        columns = [column.strip() for column in line.strip().strip("|").split("|")]
        if len(columns) != 11:
            raise ValueError(f"unexpected column count in {path}: {line}")

        rows.append(
            LedgerRow(
                row_id=columns[0],
                domain=domain,
                section=section,
                feature_group=columns[1],
                baseline_source=columns[2],
                behavior=columns[3],
                notes=columns[10],
            )
        )

    return rows


def resolve_baseline_paths(row: LedgerRow) -> list[str]:
    candidates: list[str] = []
    last_dir: str | None = None

    for token in find_path_tokens(row.baseline_source):
        resolved = resolve_token(token, last_dir)
        if resolved:
            candidates.append(resolved)
            last_dir = str(Path(resolved).parent)

    if not candidates:
        for fallback in ROW_FALLBACKS.get(row.row_id, SECTION_FALLBACKS[row.domain].get(row.section, [])):
            if (BASELINE_ROOT / fallback).exists():
                candidates.append(fallback)

    unique = []
    for candidate in candidates:
        if candidate not in unique:
            unique.append(candidate)

    if not unique:
        raise ValueError(f"no baseline paths resolved for {row.row_id}")

    return [f"baseline-2026.2.0/{candidate}" for candidate in unique]


def find_path_tokens(text: str) -> list[str]:
    tokens: list[str] = []
    tokens.extend(extract_backticks(text))
    tokens.extend(PATH_TOKEN_RE.findall(text))
    seen = []
    for token in tokens:
        if token not in seen:
            seen.append(token)
    return seen


def resolve_token(token: str, last_dir: str | None) -> str | None:
    cleaned = token.strip().strip("`").strip(".,;:()")
    cleaned = cleaned.strip('"\'')
    if not cleaned or cleaned.startswith("http://") or cleaned.startswith("https://"):
        return None

    if cleaned.startswith("baseline-2026.2.0/"):
        cleaned = cleaned.removeprefix("baseline-2026.2.0/")

    direct = BASELINE_ROOT / cleaned
    if direct.exists() and direct.is_file():
        return cleaned

    if last_dir and "/" not in cleaned:
        sibling = Path(last_dir) / cleaned
        sibling_path = BASELINE_ROOT / sibling
        if sibling_path.exists() and sibling_path.is_file():
            return sibling.as_posix()

    if "/" not in cleaned:
        matches = sorted(
            path.relative_to(BASELINE_ROOT).as_posix()
            for path in BASELINE_ROOT.rglob(cleaned)
            if path.is_file()
        )
        if len(matches) == 1:
            return matches[0]

    return None


def extract_symbol_hints(row: LedgerRow) -> list[str]:
    values: list[str] = []
    texts = [row.feature_group, row.baseline_source, row.behavior, row.notes]

    for text in texts:
        for token in extract_backticks(text):
            if should_keep_symbol(token):
                values.append(token)

        for pattern in (FLAG_RE, ENV_RE, ROUTE_RE):
            for token in pattern.findall(text):
                values.append(token)

        for token in IDENT_RE.findall(text):
            if token.endswith("()") or token in {"Type", "notify", "RestartSec", "ReadTimeout", "WriteTimeout"}:
                values.append(token)

    unique = []
    for value in values:
        normalized = value.strip()
        if not normalized or normalized in unique:
            continue
        unique.append(normalized)

    if not unique:
        unique.append(row.feature_group)

    return unique[:10]


def should_keep_symbol(token: str) -> bool:
    cleaned = token.strip()
    if not cleaned or len(cleaned) > 96:
        return False
    if cleaned.startswith("baseline-"):
        return False
    if PATH_TOKEN_RE.fullmatch(cleaned):
        return False
    return True


def extract_backticks(text: str) -> list[str]:
    return re.findall(r"`([^`]+)`", text)


if __name__ == "__main__":
    raise SystemExit(main())
