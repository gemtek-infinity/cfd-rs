#!/usr/bin/env python3
# Shared-behavior parity harness for config, credentials, and ingress evidence.

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
import tempfile
import tomllib


REPO_ROOT = Path(__file__).resolve().parent.parent
FIXTURE_ROOT = (
    REPO_ROOT / "crates" / "cfdrs-shared" / "tests" / "fixtures" / "shared-behavior"
)
INDEX_PATH = FIXTURE_ROOT / "fixture-index.toml"
GO_TRUTH_DIR = FIXTURE_ROOT / "golden" / "go-truth"
RUST_ACTUAL_DIR = FIXTURE_ROOT / "golden" / "rust-actual"
GO_CAPTURE_RUNNER = REPO_ROOT / "tools" / "shared_behavior_go_capture" / "main.go"
LOCAL_GO_BINARY = (
    Path.home() / ".local" / "go-toolchain" / "usr" / "lib" / "go-1.22" / "bin" / "go"
)
SUPPORTED_RUST_ACTUAL_CATEGORIES = {
    "config-discovery",
    "credentials-origin-cert",
    "ingress-normalization",
    "yaml-config",
    "invalid-input",
    "ordering-defaulting",
}
SUPPORTED_GO_TRUTH_CATEGORIES = set(SUPPORTED_RUST_ACTUAL_CATEGORIES)


@dataclass(frozen=True)
class Fixture:
    category: str
    comparison: str
    go_truth: tuple[str, ...]
    fixture_id: str
    input_path: Path

    @property
    def go_truth_path(self) -> Path:
        return GO_TRUTH_DIR / f"{self.fixture_id}.json"

    @property
    def rust_actual_path(self) -> Path:
        return RUST_ACTUAL_DIR / f"{self.fixture_id}.json"


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    fixtures = load_fixtures()
    return args.func(args, fixtures)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Parity harness entrypoint for the shared-behavior surface."
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    inventory = subparsers.add_parser(
        "inventory", help="List shared-behavior fixtures and paths."
    )
    inventory.add_argument(
        "--format",
        choices=("text", "json"),
        default="text",
        help="Output format.",
    )
    inventory.set_defaults(func=cmd_inventory)

    check_go_truth = subparsers.add_parser(
        "check-go-truth",
        help="Fail if any accepted fixture is missing a checked-in Go truth JSON artifact.",
    )
    check_go_truth.set_defaults(func=cmd_check_go_truth)

    capture_go_truth = subparsers.add_parser(
        "capture-go-truth",
        help="Generate checked-in Go truth artifacts for the supported shared-behavior fixtures.",
    )
    capture_go_truth.add_argument(
        "--fixture-id",
        action="append",
        default=[],
        help="Limit capture to one or more fixture IDs.",
    )
    capture_go_truth.add_argument(
        "--output-dir",
        default=str(GO_TRUTH_DIR),
        help="Directory where Go truth JSON files should be written.",
    )
    capture_go_truth.set_defaults(func=cmd_capture_go_truth)

    compare = subparsers.add_parser(
        "compare",
        help="Run real Go-versus-Rust comparison for the selected shared-behavior fixtures.",
    )
    compare.add_argument(
        "--require-go-truth",
        action="store_true",
        help="Fail if any selected fixture is missing a Go truth artifact.",
    )
    compare.add_argument(
        "--require-rust-actual",
        action="store_true",
        help="Fail if any selected fixture is missing a Rust actual artifact.",
    )
    compare.add_argument(
        "--fixture-id",
        action="append",
        default=[],
        help="Limit comparison to one or more fixture IDs.",
    )
    compare.set_defaults(func=cmd_compare)

    emit_rust_actual = subparsers.add_parser(
        "emit-rust-actual",
        help="Generate Rust-side actual artifacts for the targeted shared-behavior fixtures.",
    )
    emit_rust_actual.add_argument(
        "--fixture-id",
        action="append",
        default=[],
        help="Limit emission to one or more fixture IDs.",
    )
    emit_rust_actual.add_argument(
        "--output-dir",
        default=str(RUST_ACTUAL_DIR),
        help="Directory where Rust actual JSON files should be written.",
    )
    emit_rust_actual.set_defaults(func=cmd_emit_rust_actual)

    return parser


def load_fixtures() -> list[Fixture]:
    with INDEX_PATH.open("rb") as handle:
        raw = tomllib.load(handle)

    fixtures = []
    for entry in raw.get("fixture", []):
        input_path = FIXTURE_ROOT / entry["input"]
        fixtures.append(
            Fixture(
                category=entry["category"],
                comparison=entry["comparison"],
                go_truth=tuple(entry["go_truth"]),
                fixture_id=entry["id"],
                input_path=input_path,
            )
        )

    if not fixtures:
        raise SystemExit(f"no fixtures found in {INDEX_PATH}")

    return fixtures


def cmd_inventory(args: argparse.Namespace, fixtures: list[Fixture]) -> int:
    payload = [
        {
            "fixture_id": fixture.fixture_id,
            "category": fixture.category,
            "comparison": fixture.comparison,
            "input": fixture.input_path.relative_to(FIXTURE_ROOT).as_posix(),
            "go_truth": fixture.go_truth_path.relative_to(REPO_ROOT).as_posix(),
            "rust_actual": fixture.rust_actual_path.relative_to(REPO_ROOT).as_posix(),
        }
        for fixture in fixtures
    ]

    if args.format == "json":
        print(json.dumps(payload, indent=2, sort_keys=True))
        return 0

    print("Shared-behavior fixture inventory")
    print(f"fixture root: {FIXTURE_ROOT.relative_to(REPO_ROOT).as_posix()}")
    print(f"fixtures: {len(fixtures)}")
    for fixture in fixtures:
        print(
            f"- {fixture.fixture_id} | category={fixture.category} | comparison={fixture.comparison}"
        )
        print(f"  input: {fixture.input_path.relative_to(REPO_ROOT).as_posix()}")
        print(f"  go truth: {fixture.go_truth_path.relative_to(REPO_ROOT).as_posix()}")
        print(
            f"  rust actual: {fixture.rust_actual_path.relative_to(REPO_ROOT).as_posix()}"
        )
    return 0


def cmd_check_go_truth(_args: argparse.Namespace, fixtures: list[Fixture]) -> int:
    missing = [fixture for fixture in fixtures if not fixture.go_truth_path.exists()]
    if not missing:
        print(f"all {len(fixtures)} fixtures have checked-in Go truth artifacts")
        return 0

    print(
        "missing Go truth artifacts for shared-behavior fixtures:", file=sys.stderr
    )
    for fixture in missing:
        print(
            f"- {fixture.fixture_id}: expected {fixture.go_truth_path.relative_to(REPO_ROOT).as_posix()}",
            file=sys.stderr,
        )
    print(
        "capture Go outputs before claiming executable shared-behavior parity.",
        file=sys.stderr,
    )
    return 1


def cmd_capture_go_truth(args: argparse.Namespace, fixtures: list[Fixture]) -> int:
    selected = select_fixtures(fixtures, args.fixture_id)
    targeted = [
        fixture
        for fixture in selected
        if fixture.category in SUPPORTED_GO_TRUTH_CATEGORIES
    ]
    skipped = [
        fixture
        for fixture in selected
        if fixture.category not in SUPPORTED_GO_TRUTH_CATEGORIES
    ]

    if not targeted:
        print(
            "no supported shared-behavior fixtures were selected for Go truth capture",
            file=sys.stderr,
        )
        return 1

    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    completed = run_go_capture(targeted, output_dir)

    if skipped:
        for fixture in skipped:
            print(
                f"skipping unsupported Go truth category for {fixture.fixture_id}: {fixture.category}",
                file=sys.stderr,
            )

    if completed.returncode != 0:
        if completed.stderr:
            print(completed.stderr, file=sys.stderr, end="")
        if completed.stdout:
            print(completed.stdout, file=sys.stderr, end="")
        return completed.returncode

    print(
        f"captured {len(targeted)} Go truth artifacts into {display_repo_relative(output_dir)}"
    )
    for fixture in targeted:
        print(f"- {fixture.fixture_id}")
    return 0


def cmd_compare(args: argparse.Namespace, fixtures: list[Fixture]) -> int:
    selected = select_fixtures(fixtures, args.fixture_id)
    missing_go_truth = [
        fixture for fixture in selected if not fixture.go_truth_path.exists()
    ]

    print("Shared-behavior Rust-vs-Go comparison")
    print(f"selected fixtures: {len(selected)}")
    print(f"missing Go truth: {len(missing_go_truth)}")

    if args.require_go_truth and missing_go_truth:
        print(
            "compare failed because Go truth artifacts are still missing.",
            file=sys.stderr,
        )
        for fixture in missing_go_truth:
            print(
                f"- {fixture.fixture_id}: expected {fixture.go_truth_path.relative_to(REPO_ROOT).as_posix()}",
                file=sys.stderr,
            )
        return 1

    with tempfile.TemporaryDirectory(prefix="cloudflared-rust-actual-") as temp_dir:
        rust_actual_dir = Path(temp_dir)
        completed = run_rust_emitter(selected, rust_actual_dir)
        if completed.returncode != 0:
            if completed.stderr:
                print(completed.stderr, file=sys.stderr, end="")
            if completed.stdout:
                print(completed.stdout, file=sys.stderr, end="")
            return completed.returncode

        missing_rust_actual = [
            fixture
            for fixture in selected
            if not rust_actual_path_for_dir(rust_actual_dir, fixture).exists()
        ]
        compared = 0
        matched = 0
        mismatches: list[tuple[Fixture, list[str]]] = []

        for fixture in selected:
            if not fixture.go_truth_path.exists():
                print(f"- {fixture.fixture_id}: missing-go-truth")
                continue

            rust_actual_path = rust_actual_path_for_dir(rust_actual_dir, fixture)
            if not rust_actual_path.exists():
                print(f"- {fixture.fixture_id}: missing-rust-actual")
                continue

            compared += 1
            go_truth = load_json_artifact(fixture.go_truth_path)
            rust_actual = load_json_artifact(rust_actual_path)
            differences = compare_artifacts(fixture, go_truth, rust_actual)
            if differences:
                mismatches.append((fixture, differences))
                print(f"- {fixture.fixture_id}: mismatch")
                for difference in differences:
                    print(f"  {difference}")
            else:
                matched += 1
                print(f"- {fixture.fixture_id}: match")

        print(f"compared: {compared}")
        print(f"matched: {matched}")
        print(f"mismatched: {len(mismatches)}")
        print(f"missing Rust actual: {len(missing_rust_actual)}")

        if args.require_rust_actual and missing_rust_actual:
            print(
                "compare failed because Rust actual artifacts are still missing.",
                file=sys.stderr,
            )
            return 1
        if mismatches:
            print(
                "compare failed because one or more fixture artifacts differ.",
                file=sys.stderr,
            )
            return 1
        if args.require_go_truth and missing_go_truth:
            return 1
        if args.require_rust_actual and missing_rust_actual:
            return 1
        return 0


def cmd_emit_rust_actual(args: argparse.Namespace, fixtures: list[Fixture]) -> int:
    selected = select_fixtures(fixtures, args.fixture_id)
    targeted = [
        fixture
        for fixture in selected
        if fixture.category in SUPPORTED_RUST_ACTUAL_CATEGORIES
    ]
    skipped = [
        fixture
        for fixture in selected
        if fixture.category not in SUPPORTED_RUST_ACTUAL_CATEGORIES
    ]

    if not targeted:
        print("no targeted shared-behavior fixtures were selected", file=sys.stderr)
        return 1

    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    plan = {
        "repo_root": str(REPO_ROOT),
        "fixture_root": str(FIXTURE_ROOT),
        "output_dir": str(output_dir),
        "fixtures": [build_emission_fixture(fixture) for fixture in targeted],
    }

    completed = run_rust_emitter(targeted, output_dir)

    if skipped:
        for fixture in skipped:
            print(
                f"skipping unsupported shared-behavior category for {fixture.fixture_id}: {fixture.category}",
                file=sys.stderr,
            )

    if completed.returncode != 0:
        if completed.stderr:
            print(completed.stderr, file=sys.stderr, end="")
        if completed.stdout:
            print(completed.stdout, file=sys.stderr, end="")
        return completed.returncode

    print(
        f"emitted {len(targeted)} Rust actual artifacts into {display_repo_relative(output_dir)}"
    )
    for fixture in targeted:
        print(f"- {fixture.fixture_id}")
    return 0


def select_fixtures(fixtures: list[Fixture], fixture_ids: list[str]) -> list[Fixture]:
    if not fixture_ids:
        return fixtures

    allowed = set(fixture_ids)
    selected = [fixture for fixture in fixtures if fixture.fixture_id in allowed]
    missing = sorted(allowed.difference({fixture.fixture_id for fixture in selected}))
    if missing:
        raise SystemExit(f"unknown fixture ids: {', '.join(missing)}")
    return selected


def comparison_status(fixture: Fixture) -> str:
    go_truth_exists = fixture.go_truth_path.exists()
    rust_actual_exists = fixture.rust_actual_path.exists()
    if go_truth_exists and rust_actual_exists:
        return "ready-for-json-compare"
    if not go_truth_exists:
        return "blocked-missing-go-truth"
    return "waiting-for-rust-actual"


def build_emission_fixture(fixture: Fixture) -> dict[str, object]:
    payload: dict[str, object] = {
        "fixture_id": fixture.fixture_id,
        "category": fixture.category,
        "comparison": fixture.comparison,
        "input": fixture.input_path.relative_to(FIXTURE_ROOT).as_posix(),
        "source_refs": list(fixture.go_truth),
    }
    if fixture.category == "config-discovery":
        payload["discovery_case"] = load_discovery_case(fixture.fixture_id)
    if fixture.category == "credentials-origin-cert":
        payload["origin_cert_source"] = load_origin_cert_source(fixture.fixture_id)
    if (
        fixture.category == "ordering-defaulting"
        and fixture.input_path.name == "cases.toml"
    ):
        payload["ordering_case"] = load_ordering_case(fixture.fixture_id)
    if fixture.category == "ingress-normalization":
        payload["flag_ingress_case"] = load_flag_ingress_case(fixture.fixture_id)
    return payload


def run_rust_emitter(
    fixtures: list[Fixture], output_dir: Path
) -> subprocess.CompletedProcess[str]:
    plan = {
        "repo_root": str(REPO_ROOT),
        "fixture_root": str(FIXTURE_ROOT),
        "output_dir": str(output_dir),
        "fixtures": [build_emission_fixture(fixture) for fixture in fixtures],
    }

    return subprocess.run(
        [
            "cargo",
            "run",
            "-q",
            "-p",
            "cfdrs-shared",
            "--example",
            "shared_behavior_emit",
        ],
        cwd=REPO_ROOT,
        input=json.dumps(plan),
        text=True,
        capture_output=True,
    )


def run_go_capture(
    fixtures: list[Fixture], output_dir: Path
) -> subprocess.CompletedProcess[str]:
    plan = {
        "repo_root": str(REPO_ROOT),
        "fixture_root": str(FIXTURE_ROOT),
        "output_dir": str(output_dir),
        "fixtures": [build_emission_fixture(fixture) for fixture in fixtures],
    }

    with tempfile.TemporaryDirectory(prefix="cloudflared-go-truth-") as temp_dir:
        temp_root = Path(temp_dir)
        write_go_capture_module(temp_root)
        try:
            go_binary = str(go_executable())
            tidy = subprocess.run(
                [go_binary, "mod", "tidy"],
                cwd=temp_root,
                text=True,
                capture_output=True,
            )
            if tidy.returncode != 0:
                return tidy
            return subprocess.run(
                [go_binary, "run", "."],
                cwd=temp_root,
                input=json.dumps(plan),
                text=True,
                capture_output=True,
            )
        except FileNotFoundError as error:
            raise SystemExit(
                "go toolchain not found on PATH; install Go to run capture-go-truth"
            ) from error


def write_go_capture_module(temp_root: Path) -> None:
    shutil.copy2(GO_CAPTURE_RUNNER, temp_root / "main.go")
    go_mod = f"""module sharedbehaviorcapture

go 1.24.0

require (
    github.com/cloudflare/cloudflared v0.0.0
    github.com/rs/zerolog v1.20.0
    github.com/urfave/cli/v2 v2.3.0
    golang.org/x/net v0.40.0
    gopkg.in/yaml.v3 v3.0.1
)

replace github.com/cloudflare/cloudflared => {REPO_ROOT / 'baseline-2026.2.0'}
"""
    (temp_root / "go.mod").write_text(go_mod)


def go_executable() -> Path:
    configured = os.environ.get("GO_BINARY") or shutil.which("go")
    if configured:
        return Path(configured)
    if LOCAL_GO_BINARY.exists():
        return LOCAL_GO_BINARY
    raise FileNotFoundError("go")


def load_json_artifact(path: Path) -> dict[str, object]:
    return json.loads(path.read_text())


def rust_actual_path_for_dir(output_dir: Path, fixture: Fixture) -> Path:
    return output_dir / f"{fixture.fixture_id}.json"


def compare_artifacts(
    fixture: Fixture,
    go_truth: dict[str, object],
    rust_actual: dict[str, object],
) -> list[str]:
    differences: list[str] = []
    for field in [
        "schema_version",
        "fixture_id",
        "report_kind",
        "comparison",
        "source_refs",
    ]:
        if go_truth.get(field) != rust_actual.get(field):
            differences.append(
                f"envelope.{field}: go={render_value(go_truth.get(field))} rust={render_value(rust_actual.get(field))}"
            )

    comparison = fixture.comparison
    if comparison in {"exact", "exact-json"}:
        differences.extend(
            diff_json(
                go_truth.get("payload"), rust_actual.get("payload"), path="payload"
            )
        )
    elif comparison == "error-category":
        differences.extend(compare_error_category(go_truth, rust_actual))
    elif comparison == "structural":
        differences.extend(compare_structural(go_truth, rust_actual))
    elif comparison == "semantic":
        differences.extend(compare_semantic(go_truth, rust_actual))
    elif comparison == "warning-or-report":
        differences.extend(compare_warning_or_report(go_truth, rust_actual))
    else:
        differences.append(f"unsupported comparison mode: {comparison}")

    return differences


def compare_error_category(
    go_truth: dict[str, object], rust_actual: dict[str, object]
) -> list[str]:
    differences: list[str] = []
    if go_truth.get("report_kind") != "error-report.v1":
        differences.append(
            f"go report_kind must be error-report.v1, found {go_truth.get('report_kind')!r}"
        )
    if rust_actual.get("report_kind") != "error-report.v1":
        differences.append(
            f"rust report_kind must be error-report.v1, found {rust_actual.get('report_kind')!r}"
        )
    go_payload = as_dict(go_truth.get("payload"))
    rust_payload = as_dict(rust_actual.get("payload"))
    if go_payload.get("category") != rust_payload.get("category"):
        differences.append(
            f"payload.category: go={render_value(go_payload.get('category'))} rust={render_value(rust_payload.get('category'))}"
        )
    return differences


def compare_structural(
    go_truth: dict[str, object], rust_actual: dict[str, object]
) -> list[str]:
    differences: list[str] = []
    go_payload = as_dict(go_truth.get("payload"))
    rust_payload = as_dict(rust_actual.get("payload"))
    for key in ["action", "source_kind", "resolved_path", "created_paths"]:
        if go_payload.get(key) != rust_payload.get(key):
            differences.append(
                f"payload.{key}: go={render_value(go_payload.get(key))} rust={render_value(rust_payload.get(key))}"
            )
    return differences


def compare_semantic(
    go_truth: dict[str, object], rust_actual: dict[str, object]
) -> list[str]:
    go_contract = extract_no_ingress_contract(go_truth)
    rust_contract = extract_no_ingress_contract(rust_actual)
    if go_contract == rust_contract:
        return []
    return [
        f"semantic no-ingress contract: go={render_value(go_contract)} rust={render_value(rust_contract)}"
    ]


def extract_no_ingress_contract(artifact: dict[str, object]) -> dict[str, object]:
    payload = as_dict(artifact.get("payload"))
    ingress_rules = payload.get("ingress")
    if not isinstance(ingress_rules, list) or not ingress_rules:
        return {"report_kind": artifact.get("report_kind"), "ingress": None}
    last_rule = as_dict(ingress_rules[-1])
    service = as_dict(last_rule.get("service"))
    return {
        "report_kind": artifact.get("report_kind"),
        "ingress_count": len(ingress_rules),
        "last_service_kind": service.get("kind"),
        "last_status_code": service.get("status_code"),
    }


def compare_warning_or_report(
    go_truth: dict[str, object], rust_actual: dict[str, object]
) -> list[str]:
    if go_truth.get("report_kind") != rust_actual.get("report_kind"):
        return [
            f"report_kind: go={render_value(go_truth.get('report_kind'))} rust={render_value(rust_actual.get('report_kind'))}"
        ]

    if go_truth.get("report_kind") == "error-report.v1":
        return compare_error_category(go_truth, rust_actual)

    go_payload = as_dict(go_truth.get("payload"))
    rust_payload = as_dict(rust_actual.get("payload"))
    if go_payload.get("warnings") == rust_payload.get("warnings"):
        return []
    return [
        f"payload.warnings: go={render_value(go_payload.get('warnings'))} rust={render_value(rust_payload.get('warnings'))}"
    ]


def diff_json(
    go_value: object,
    rust_value: object,
    *,
    path: str,
    limit: int = 20,
) -> list[str]:
    differences: list[str] = []

    def walk(left: object, right: object, current_path: str) -> None:
        if len(differences) >= limit:
            return
        if type(left) is not type(right):
            differences.append(
                f"{current_path}: go={render_value(left)} rust={render_value(right)}"
            )
            return
        if isinstance(left, dict):
            assert isinstance(right, dict)
            keys = sorted(set(left) | set(right))
            for key in keys:
                if len(differences) >= limit:
                    return
                if key not in left:
                    differences.append(
                        f"{current_path}.{key}: missing in go, rust={render_value(right[key])}"
                    )
                    continue
                if key not in right:
                    differences.append(
                        f"{current_path}.{key}: go={render_value(left[key])}, missing in rust"
                    )
                    continue
                walk(left[key], right[key], f"{current_path}.{key}")
            return
        if isinstance(left, list):
            assert isinstance(right, list)
            if len(left) != len(right):
                differences.append(
                    f"{current_path}.length: go={len(left)} rust={len(right)}"
                )
                return
            for index, (left_item, right_item) in enumerate(zip(left, right)):
                walk(left_item, right_item, f"{current_path}[{index}]")
            return
        if left != right:
            differences.append(
                f"{current_path}: go={render_value(left)} rust={render_value(right)}"
            )

    walk(go_value, rust_value, path)
    return differences


def as_dict(value: object) -> dict[str, object]:
    if isinstance(value, dict):
        return value
    return {}


def render_value(value: object) -> str:
    return json.dumps(value, sort_keys=True)


def load_discovery_case(fixture_id: str) -> dict[str, object]:
    cases_path = FIXTURE_ROOT / "config-discovery" / "cases.toml"
    with cases_path.open("rb") as handle:
        raw = tomllib.load(handle)

    for case in raw.get("case", []):
        if case.get("id") == fixture_id:
            return {
                "explicit_config": bool(case.get("explicit_config", False)),
                "present": list(case.get("present", [])),
            }

    raise SystemExit(f"missing config discovery case for fixture {fixture_id}")


def load_origin_cert_source(fixture_id: str) -> str:
    sources_path = FIXTURE_ROOT / "credentials-origin-cert" / "sources.toml"
    with sources_path.open("rb") as handle:
        raw = tomllib.load(handle)

    for source in raw.get("source", []):
        if source.get("id") == fixture_id:
            return str(source["path"])

    raise SystemExit(f"missing credentials source for fixture {fixture_id}")


def load_ordering_case(fixture_id: str) -> dict[str, object]:
    cases_path = FIXTURE_ROOT / "ordering-defaulting" / "cases.toml"
    with cases_path.open("rb") as handle:
        raw = tomllib.load(handle)

    for case in raw.get("case", []):
        if case.get("id") == fixture_id:
            return {
                "input": str(case["input"]),
            }

    raise SystemExit(f"missing ordering/defaulting case for fixture {fixture_id}")


def load_flag_ingress_case(fixture_id: str) -> dict[str, object]:
    cases_path = FIXTURE_ROOT / "ingress-normalization" / "cases.toml"
    with cases_path.open("rb") as handle:
        raw = tomllib.load(handle)

    for case in raw.get("case", []):
        if case.get("id") == fixture_id:
            return {
                "flags": list(case.get("flags", [])),
            }

    raise SystemExit(f"missing ingress-normalization case for fixture {fixture_id}")


def display_repo_relative(path: Path) -> str:
    try:
        return path.relative_to(REPO_ROOT).as_posix()
    except ValueError:
        return str(path)


if __name__ == "__main__":
    raise SystemExit(main())
