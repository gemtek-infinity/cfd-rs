#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
STRING_LITERAL_RE = re.compile(r'"(?:\\.|[^"\\])*"')

TARGETS = {
    REPO_ROOT / "crates/cfdrs-cli/src/parse.rs": {
        "exact": {
            "--config",
            "--help",
            "-h",
            "help",
            "--version",
            "-v",
            "-V",
            "version",
            "validate",
            "run",
            "missing value for --config",
            "--config may only be provided once",
            "unknown flag: {flag}",
            "unknown command or argument: {value}",
            "multiple commands were provided: {existing} and {next}",
        },
        "prefix": set(),
    },
    REPO_ROOT / "crates/cfdrs-cli/src/help.rs": {
        "exact": {
            "cloudflared",
            "Usage:",
            "Admitted commands:",
        },
        "prefix": set(),
    },
    REPO_ROOT / "crates/cfdrs-cli/src/error.rs": {
        "exact": {
            "error: {message}\nRun `cloudflared help` for the admitted command surface.\n",
            "error: startup validation failed [{category}]: {error}\n",
        },
        "prefix": set(),
    },
    REPO_ROOT / "crates/cfdrs-cli/src/types.rs": {
        "exact": {"help", "version", "validate", "run"},
        "prefix": set(),
    },
    REPO_ROOT / "crates/cfdrs-bin/src/main.rs": {
        "exact": {"cloudflared"},
        "prefix": set(),
    },
    REPO_ROOT / "crates/cfdrs-bin/src/runtime/state/deployment_evidence.rs": {
        "exact": {"no-systemd-unit", "no-installer", "alpha-only", "no-config-reload"},
        "prefix": {"deploy-"},
    },
    REPO_ROOT / "crates/cfdrs-cdc/src/stream.rs": {
        "exact": {
            "HttpMethod",
            "HttpHost",
            "HttpHeader",
            "HttpStatus",
            "FlowID",
            "cf-trace-id",
            "HttpHeader:Content-Length",
            "cf-trace-context",
            "GET",
            "HTTP",
            "WebSocket",
            "TCP",
        },
        "prefix": {"HttpHeader:"},
    },
}


def main() -> int:
    errors: list[str] = []

    for path, rules in TARGETS.items():
        literals = list(string_literals(path.read_text(encoding="utf-8")))
        exact_hits = sorted({literal for literal in literals if literal in rules["exact"]})
        prefix_hits = sorted(
            {
                literal
                for literal in literals
                for prefix in rules["prefix"]
                if literal.startswith(prefix)
            }
        )

        for hit in exact_hits + prefix_hits:
            errors.append(
                f"contract literal '{hit}' must live in a dedicated contract module, not {path.relative_to(REPO_ROOT)}"
            )

    if errors:
        for error in errors:
            print(f"ERROR: {error}")
        return 1

    print("contract-literal-validation: ok")
    return 0


def string_literals(text: str):
    for match in STRING_LITERAL_RE.finditer(text):
        yield bytes(match.group(0)[1:-1], "utf-8").decode("unicode_escape")


if __name__ == "__main__":
    sys.exit(main())
