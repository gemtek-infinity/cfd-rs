#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path
import tomllib


REPO_ROOT = Path(__file__).resolve().parent.parent
FIXTURE_ROOT = (
    REPO_ROOT / "crates" / "cloudflared-config" / "tests" / "fixtures" / "first-slice"
)
INDEX_PATH = FIXTURE_ROOT / "fixture-index.toml"
GO_TRUTH_DIR = FIXTURE_ROOT / "golden" / "go-truth"
RUST_ACTUAL_DIR = FIXTURE_ROOT / "golden" / "rust-actual"
SUPPORTED_RUST_ACTUAL_CATEGORIES = {
    "config-discovery",
    "yaml-config",
    "invalid-input",
    "ordering-defaulting",
}


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
        description="Phase 1A parity harness entrypoint for the accepted first slice."
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    inventory = subparsers.add_parser(
        "inventory", help="List first-slice fixtures and paths."
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

    compare = subparsers.add_parser(
        "compare",
        help="Describe or execute the Go-versus-Rust comparison contract.",
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
        help="Generate Rust-side actual artifacts for the targeted Phase 1B.2 fixtures.",
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

    print("Phase 1A fixture inventory")
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
        "missing Go truth artifacts for accepted first-slice fixtures:", file=sys.stderr
    )
    for fixture in missing:
        print(
            f"- {fixture.fixture_id}: expected {fixture.go_truth_path.relative_to(REPO_ROOT).as_posix()}",
            file=sys.stderr,
        )
    print(
        "capture Go outputs before claiming executable first-slice parity.",
        file=sys.stderr,
    )
    return 1


def cmd_compare(args: argparse.Namespace, fixtures: list[Fixture]) -> int:
    selected = select_fixtures(fixtures, args.fixture_id)
    missing_go_truth = [
        fixture for fixture in selected if not fixture.go_truth_path.exists()
    ]
    missing_rust_actual = [
        fixture for fixture in selected if not fixture.rust_actual_path.exists()
    ]
    comparable = [
        fixture
        for fixture in selected
        if fixture.go_truth_path.exists() and fixture.rust_actual_path.exists()
    ]

    print("Phase 1A comparison contract")
    print(f"selected fixtures: {len(selected)}")
    print(f"comparable today: {len(comparable)}")
    print(f"missing Go truth: {len(missing_go_truth)}")
    print(f"missing Rust actual: {len(missing_rust_actual)}")

    for fixture in selected:
        status = comparison_status(fixture)
        print(f"- {fixture.fixture_id}: {status}")

    if args.require_go_truth and missing_go_truth:
        print(
            "compare failed because Go truth artifacts are still missing.",
            file=sys.stderr,
        )
        return 1

    if args.require_rust_actual and missing_rust_actual:
        print(
            "compare failed because Rust actual artifacts are still missing.",
            file=sys.stderr,
        )
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
        print("no Phase 1B.2-targeted fixtures were selected", file=sys.stderr)
        return 1

    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    plan = {
        "fixture_root": str(FIXTURE_ROOT),
        "output_dir": str(output_dir),
        "fixtures": [build_emission_fixture(fixture) for fixture in targeted],
    }

    import subprocess

    completed = subprocess.run(
        [
            "cargo",
            "run",
            "-q",
            "-p",
            "cloudflared-config",
            "--example",
            "first_slice_emit",
        ],
        cwd=REPO_ROOT,
        input=json.dumps(plan),
        text=True,
        capture_output=True,
    )

    if skipped:
        for fixture in skipped:
            print(
                f"skipping unsupported Phase 1B.2 category for {fixture.fixture_id}: {fixture.category}",
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
    return payload


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


def display_repo_relative(path: Path) -> str:
    try:
        return path.relative_to(REPO_ROOT).as_posix()
    except ValueError:
        return str(path)


if __name__ == "__main__":
    raise SystemExit(main())
