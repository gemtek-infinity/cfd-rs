# CLI Implementation Checklist

## Purpose

This document is the live parity ledger for the cloudflared CLI surface.

Parity in this document means parity against the frozen Go baseline for:

- visible command structure
- hidden and compatibility command structure
- help and usage text
- flag and environment-variable binding behavior
- exit-code behavior
- stdout and stderr behavior
- formatting details that are blackbox-visible

This document does not claim parity from Rust code shape alone.

It records:

- the frozen CLI behavior or contract that must be matched
- the current Rust owner, if any
- the current Rust implementation state
- the current evidence maturity
- whether a gap or divergence is open
- the tests required before parity can be claimed

## Checklist Field Vocabulary

The table uses three different status fields.

### Rust status now

Use only these values:

- not audited
- audited, absent
- audited, partial
- audited, parity-backed
- audited, intentional divergence
- blocked

### Parity evidence status

Preferred values:

- not present
- minimal
- weak
- partial
- parity-backed
- first-slice evidence exists
- partial local tests only

If a new value is needed later, add it deliberately and keep it short.

### Divergence status

Preferred values:

- none recorded
- open gap
- intentional divergence
- unknown
- blocked

## Seeded Checklist

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CLI-001 | root invocation | frozen root app behavior | empty invocation is not pure help; it follows the frozen root action and service-mode behavior | current binary plus current CLI surface | audited, absent | not present | open gap | blackbox empty invocation capture, stdout and stderr capture, exit-code compare | critical | current Rust surface is narrow and help-centric |
| CLI-002 | root help text | root app help output | root help exposes the frozen top-level command families, wording, ordering, and visible help structure | current CLI surface | audited, absent | not present | open gap | exact help snapshot compare, top-level command inventory capture | critical | current help only exposes `validate`, `run`, `help`, and `version` |
| CLI-003 | root global flags | root app flags and shared flags | global flags, aliases, environment bindings, defaults, and hidden flags must match the frozen baseline where lane-relevant | current CLI surface | not audited | not present | unknown | help crawl, source inventory, flag binding capture, env-behavior tests | critical | include hidden flags and aliases, not just visible help rows |
| CLI-004 | help command behavior | root help command and command-local help behavior | explicit help command and help routing behavior must match the frozen baseline for root and subcommands | current CLI surface | audited, partial | minimal | open gap | help-command snapshot tests, subcommand help-routing tests, exit-code tests | high | current Rust exposes help behavior, but upstream help-command parity is not audited |
| CLI-005 | version command | version command contract | version output, short mode, and related formatting must match the frozen baseline where supported | current CLI surface | audited, partial | minimal | open gap | exact stdout snapshot tests, flag-mode tests, exit-code tests | high | current Rust has basic version output, but no audited short-mode parity |
| CLI-006 | update command | root command tree and update command behavior | update command, flags, messaging, and exit behavior must match the frozen baseline where lane-relevant | none in current Rust | audited, absent | not present | open gap | help capture, command invocation tests, exit-code tests | high | includes behavior when update is attempted, not just command presence |
| CLI-007 | service command | Linux service command surface | Linux service install and uninstall commands, flags, and command-level help must match the frozen baseline on the declared lane | none in current Rust | audited, absent | not present | open gap | help capture, Linux command-behavior tests, exit-code tests | critical | overlaps HIS for host effects, but command-surface parity is still CLI-owned |
| CLI-008 | tunnel root behavior | tunnel command contract | `tunnel` is both a command namespace and a runnable decision surface with frozen invocation behavior | none in current Rust | audited, absent | not present | open gap | blackbox tunnel invocation matrix, stdout and stderr capture, exit-code tests | critical | not just a static command tree |
| CLI-009 | tunnel subcommands | tunnel command tree | `tunnel` subcommands, help text, routing, and command availability must match the frozen baseline | none in current Rust | audited, absent | not present | open gap | per-subcommand help crawl, invocation matrix, subtree feature-group docs | critical | expected surface includes `login`, `create`, `route`, `vnet`, `run`, `list`, `ready`, `info`, `ingress`, `delete`, `cleanup`, `token`, and `diag` where baseline exposes them |
| CLI-010 | access subtree | access command tree | `access` subcommands, aliases, parsing quirks, and help behavior must match the frozen baseline | none in current Rust | audited, absent | not present | open gap | subtree help crawl, alias tests, targeted behavior tests | high | include `login`, `curl`, `token`, TCP aliases, `ssh-config`, and `ssh-gen` where baseline exposes them |
| CLI-011 | tail subtree | tail command tree | `tail` command, hidden token path, filters, outputs, and token sourcing behavior must match the frozen baseline | none in current Rust | audited, absent | not present | open gap | help crawl, filter-behavior tests, output-format tests, hidden-path invocation tests | high | overlaps CDC log-streaming behavior, but CLI entry semantics are still CLI-owned |
| CLI-012 | management subtree | hidden management command tree | hidden management command paths and token-related command behavior must match the frozen baseline where present | none in current Rust | audited, absent | not present | open gap | hidden-command help capture, invocation tests, failure-text tests | medium | hidden paths must be audited explicitly rather than ignored |
| CLI-013 | compatibility placeholders | removed-command compatibility contract | removed or transitional commands remain present where the frozen baseline preserves them to fail explicitly or redirect users | none in current Rust | audited, absent | not present | open gap | placeholder-command failure tests, stderr snapshot tests, exit-code tests | high | includes both top-level and nested compatibility placeholders |
| CLI-014 | help formatting contract | blackbox output contract | spacing, wrapping, headings, ordering, and wording are visible contract and must match the frozen baseline where parity is claimed | current CLI surface | audited, partial | minimal | open gap | exact text snapshots, width-sensitive capture tests, no substring-only proofs | critical | do not reduce this surface to substring assertions |
| CLI-015 | usage failure behavior | blackbox error behavior | unknown commands, bad flags, and usage failures must match the frozen baseline for error text, stream placement, and exit code | current CLI surface | audited, partial | minimal | open gap | stderr and stdout capture, exit-code tests, unknown-command and bad-flag matrix | high | current Rust has usage failure logic, but it is not upstream-parity-backed |
| CLI-016 | validate command | current Rust transitional surface | current Rust `validate` path exists as a transitional command and is not a frozen top-level parity target | current CLI surface | audited, intentional divergence | partial local tests only | intentional divergence | divergence note, transitional command tests, retirement or rename decision tracking | medium | may become internal, renamed, or retired during the final phase |
| CLI-017 | run command | current Rust runtime handoff surface versus frozen tunnel-run behavior | current Rust `run` path only partially overlaps the frozen `tunnel` runnable surface and must not be treated as full CLI parity | current runtime plus current CLI surface | audited, partial | partial local tests only | open gap | command contract tests, runtime invocation tests, compare against frozen `tunnel` runnable behavior | critical | must be reconciled against the upstream `tunnel` root and `tunnel run` contract, not treated as equivalent by name alone |

## Immediate Work Queue

1. capture root invocation and root help from the frozen Go binary
2. capture the full top-level command list, including hidden and compatibility paths where callable
3. inventory root global flags, aliases, defaults, and environment-variable bindings
4. crawl `tunnel --help` and each tunnel subcommand deeply
5. crawl `access --help` and its subcommands, including alias behavior
6. crawl `tail --help`, hidden token paths, and hidden management command behavior
7. record compatibility placeholder commands and their failure text and exit behavior
8. replace substring-only Rust CLI tests with snapshot-grade parity tests where a surface is implemented
