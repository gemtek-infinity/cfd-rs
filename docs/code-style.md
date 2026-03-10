# Code Style

This is a human-facing reference document.
For default AI code-edit guidance, start with `.github/instructions/rust.instructions.md` and load this file only when deeper explanation is useful.

## Purpose

This document defines how Rust code in this repository should **look and read**.

This is a **code style** document, not an architecture document. It focuses on:

- naming
- local readability
- control flow
- comments and docs
- tests
- common code-shape preferences

It does **not** define:

- crate boundaries
- module decomposition policy
- dependency admission
- abstraction strategy
- design-pattern policy
- lifecycle/state modeling

Those belong in `docs/engineering-standards.md`.

---

## 1. Style: Explicit and readable

This repository prefers code that is easy to understand in one pass.

Prefer:

- explicit names
- explicit local variables
- straightforward control flow
- small named helpers

Avoid:

- dense one-liners
- clever chaining
- compressed logic that saves lines but costs readability

Example:

Good:

```rust
let hostname = normalize_hostname(raw_hostname)?;
let service = parse_service(raw_service)?;
let rule = self::IngressRule::new(hostname, service);
```

Less preferred:

```rust
let rule = self::IngressRule::new(
    normalize_hostname(raw_hostname)?,
    parse_service(raw_service)?,
);
```

---

## 2. Style: Clear naming

Names should explain purpose immediately.

Prefer names that describe the domain role:

- `config_path`
- `credential_source`
- `origin_service`
- `ingress_rule`
- `normalized_hostname`

Avoid vague names:

- `data`
- `value`
- `thing`
- `helper`
- `util`
- `manager`
- `handler`

Example:

Good:

```rust
let credential_path = find_credential_path()?;
```

Bad:

```rust
let data = find_credential_path()?;
```

---

## 3. Style: Prefer idiomatic Rust when practical

Prefer idiomatic Rust when it improves clarity, correctness, and maintainability.

Prefer:

- standard library and conventional Rust patterns
- `?` for straightforward error propagation
- `Option` and `Result` used directly rather than reinvented status patterns
- `Self`, `Self::`, and local module paths when they improve readability

Avoid:

- forcing habits from other languages when Rust already has a clearer idiom
- verbose ceremony that obscures the actual behavior
- clever patterns that are technically idiomatic but harder to review than a simpler alternative

Example:

Good:

```rust
fn load(path: &Path) -> Result<Self, LoadError> {
    let text = std::fs::read_to_string(path)?;

    Self::parse(text.as_str())
}
```

Less preferred:

```rust
fn load(path: &Path) -> Result<Self, LoadError> {
    match std::fs::read_to_string(path) {
        Ok(text) => Self::parse(text.as_str()),
        Err(err) => Err(LoadError::from(err)),
    }
}
```

---

## 4. Style: Trait names describe capability or role

Trait names should describe behavior, capability, or a clear contract.

Prefer:

- capability-oriented names such as `Resolver`, `Normalizer`, `CredentialSource`, `OriginProvider`
- adjective-style names when the trait reads naturally that way, such as `Reloadable`
- `Ext` suffix only for narrow extension traits, especially when extending external types

Avoid:

- vague trait names such as `Manager`, `Handler`, `Helper`, or `Processor` unless the domain meaning is truly specific
- naming a trait like a concrete implementation
- using `Ext` for primary domain traits

Example:

Good:

```rust
pub trait OriginProvider {
    fn origin_service(&self) -> &self::OriginService;
}
```

Good:

```rust
pub trait RequestExt {
    fn header_str(&self, name: &str) -> Option<&str>;
}
```

Less preferred:

```rust
pub trait OriginManager {
    fn origin_service(&self) -> &self::OriginService;
}
```

---

## 5. Style: Tight vertical spacing

Do not add blank lines just to visually group things.

Prefer:

- one logical block per contiguous chunk
- blank lines only when separating genuinely different steps
- compact functions that do not look artificially stretched

Avoid:

- empty lines between closely related statements
- “air padding” that makes short functions look longer than they are

Example:

Good:

```rust
let hostname = normalize_hostname(raw.hostname)?;
let service = parse_service(raw.service)?;
let rule = self::IngressRule::new(hostname, service);

validate_rule(&rule)?;
register_rule(rule);
```

Less preferred:

```rust
let hostname = normalize_hostname(raw.hostname)?;

let service = parse_service(raw.service)?;

let rule = self::IngressRule::new(hostname, service);

validate_rule(&rule)?;

register_rule(rule);
```

---

## 6. Style: Boring control flow

Code should be unsurprising to read.

Prefer:

- early returns for invalid states
- `match` for meaningful branching
- `if let` or `let else` for simple guards

Avoid:

- deep nesting when a guard clause is clearer
- clever control flow that hides the important path

Example:

Good:

```rust
let Some(path) = credential_path else {
    return Err(ConfigLoadError::MissingCredentialFile);
};
```

Good:

```rust
match origin {
    Origin::Local(service) => load_local(service),
    Origin::Remote(url) => load_remote(url),
}
```

---

## 7. Style: Long sequential functions indicate chunking

A long function made of many sequential steps is usually a sign that named helper functions would improve readability.

Prefer:

- small named helpers when a function has many consecutive stages
- a visible top-level flow with implementation details extracted below

Avoid:

- long “do this, then this, then this” functions with no named sub-steps

Example:

Good:

```rust
fn load_ingress(raw: &RawIngress) -> Result<IngressRule, LoadError> {
    let hostname = parse_hostname(raw)?;
    let service = parse_service(raw)?;
    validate_ingress(&hostname, &service)?;

    Ok(self::IngressRule::new(hostname, service))
}
```

Less preferred:

```rust
fn load_ingress(raw: &RawIngress) -> Result<IngressRule, LoadError> {
    let hostname = normalize_hostname(raw.hostname.as_str())?;
    let service = parse_service(raw.service.as_str())?;

    if hostname.is_empty() {
        return Err(LoadError::InvalidHostname);
    }

    if service.is_unsupported() {
        return Err(LoadError::UnsupportedService);
    }

    if raw.origin_request.is_some() {
        // more mapping...
    }

    // more parsing...
    // more normalization...
    // more validation...

    Ok(self::IngressRule::new(hostname, service))
}
```

---

## 8. Style: Separate setup from a multi-line final expression

If a multi-line block ends with a meaningful final expression, add one blank line after preparatory statements before that final expression.

This applies to:

- final `Ok(...)` / `Err(...)`
- struct construction
- a final function call in a block
- a final expression inside a `match` arm or `if` block

Prefer:

- one blank line between setup and the final expression in a multi-line block

Avoid:

- crowding the final expression directly under setup when the block is already doing more than one step

Example:

Good:

```rust
fn build_rule(raw: RawRule) -> Result<self::IngressRule, LoadError> {
    let hostname = parse_hostname(&raw)?;
    let service = parse_service(&raw)?;
    let origin_request = parse_origin_request(&raw)?;

    Ok(self::IngressRule {
        hostname,
        service,
        origin_request,
    })
}
```

Good:

```rust
match source {
    Source::File(path) => {
        let path = validate_path(path)?;

        load_file(path)
    }
    Source::Inline(text) => load_inline(text),
}
```

Less preferred:

```rust
fn build_rule(raw: RawRule) -> Result<self::IngressRule, LoadError> {
    let hostname = parse_hostname(&raw)?;
    let service = parse_service(&raw)?;
    let origin_request = parse_origin_request(&raw)?;
    Ok(self::IngressRule {
        hostname,
        service,
        origin_request,
    })
}
```

---

## 9. Style: Visible intermediate steps

Use intermediate variables when they make the flow easier to scan.

Prefer named steps when:

- each step has meaning
- errors can happen
- values are reused
- the expression becomes visually dense

Example:

Good:

```rust
let hostname = normalize_hostname(raw.hostname)?;
let service = parse_service(raw.service)?;
let rule = self::IngressRule::new(hostname, service);
```

Acceptable when still simple:

```rust
let rule = self::IngressRule::new(hostname, service);
```

---

## 10. Style: Prefer `self::` for sibling items

When referring to types or functions defined in the same module, prefer `self::` to make local ownership obvious.

This reduces ambiguity and makes it easier for reviewers to see that the item is local to the current module.

Prefer:

- `self::IngressRule`
- `self::normalize_hostname`
- `self::LoadError`

Over:

- bare local type or function names when the origin is not immediately obvious

Example:

Good:

```rust
fn build_rule(hostname: Hostname, service: Service) -> self::IngressRule {
    self::IngressRule::new(hostname, service)
}
```

Less preferred:

```rust
fn build_rule(hostname: Hostname, service: Service) -> IngressRule {
    IngressRule::new(hostname, service)
}
```

---

## 11. Style: Prefer `Self` and `Self::` inside `impl`

Inside an `impl`, prefer `Self` and `Self::` when they make the code easier to scan and reduce repeated type names.

Prefer:

- `Self`
- `Self::new(...)`
- `Self::DEFAULT_RETRY_LIMIT`

Over:

- repeating the full type name inside its own implementation

Example:

Good:

```rust
impl IngressRule {
    fn new(hostname: Hostname, service: Service) -> Self {
        Self { hostname, service }
    }
}
```

Less preferred:

```rust
impl IngressRule {
    fn new(hostname: Hostname, service: Service) -> IngressRule {
        IngressRule { hostname, service }
    }
}
```

---

## 12. Style: Put related constants in the owning `impl`

If a constant exists only to support a specific type, prefer an associated `const` inside that type’s `impl` instead of a file-scope constant.

Prefer:

- `impl Foo { const DEFAULT_TIMEOUT_SECS: u64 = 30; }`

Avoid:

- file-scope constants when the value is only meaningful for one type

Example:

Good:

```rust
impl TunnelConfig {
    const DEFAULT_RETRY_LIMIT: usize = 3;

    fn retry_limit(&self) -> usize {
        Self::DEFAULT_RETRY_LIMIT
    }
}
```

Less preferred:

```rust
const DEFAULT_RETRY_LIMIT: usize = 3;

impl TunnelConfig {
    fn retry_limit(&self) -> usize {
        DEFAULT_RETRY_LIMIT
    }
}
```

---

## 13. Style: Avoid magic numbers in function bodies

Do not hardcode numeric values in function bodies when the value has meaning.

Prefer:

- named constants for meaningful values
- inline literals only for obvious trivial values such as `0`, `1`, or similar universally understood values

Example:

Good:

```rust
impl RetryPolicy {
    const MAX_RETRIES: usize = 3;
    const BASE_DELAY_MS: u64 = 250;

    fn backoff_ms(attempt: usize) -> u64 {
        attempt.min(Self::MAX_RETRIES) as u64 * Self::BASE_DELAY_MS
    }
}
```

Less preferred:

```rust
fn backoff_ms(attempt: usize) -> u64 {
    attempt.min(3) as u64 * 250
}
```

---

## 14. Style: Put parse and conversion types at the operation site

When parsing or converting, prefer making the target type explicit at the operation site rather than on the binding.

This makes the conversion easier to spot and reduces “where did that type come from?” scanning.

Prefer:

```rust
let hostname = raw.parse::<self::Hostname>()?;
```

Over:

```rust
let hostname: self::Hostname = raw.parse()?;
```

Example:

Good:

```rust
let service = raw_service.parse::<self::OriginService>()?;
```

Less preferred:

```rust
let service: self::OriginService = raw_service.parse()?;
```

---

## 15. Style: Avoid nested `match` when a flatter shape is possible

Nested `match` blocks should be rare.

If pattern branching becomes nested, prefer:

- helper extraction
- `if let`
- `let else`
- a flatter outer `match`
- precomputing an intermediate value

Example:

Good:

```rust
match source {
    Source::File(path) => {
        let path = validate_path(path)?;

        load_file(path)
    }
    Source::Inline(text) => load_inline(text),
}
```

Less preferred:

```rust
match source {
    Source::File(path) => match validate_path(path) {
        Ok(path) => load_file(path),
        Err(err) => Err(err),
    },
    Source::Inline(text) => load_inline(text),
}
```

---

## 16. Style: Comments explain why

Comments should explain:

- why something exists
- compatibility requirements
- invariants
- non-obvious behavior
- tradeoffs

Do not comment obvious syntax.

Bad:

```rust
// Increment retry count.
retry_count += 1;
```

Good:

```rust
// Retries are counted before backoff selection so metrics match the caller-visible attempt number.
retry_count += 1;
```

---

## 17. Style: Explain quirks explicitly

If code exists because of a compatibility quirk, parser oddity, protocol edge case, or other non-obvious reason, add a comment that explains it.

Prefer comments that explain:

- what the quirk is
- why the code exists
- what would break if it were “cleaned up” without understanding the reason

Example:

Good:

```rust
// The legacy config format treats an empty hostname as a catch-all rule.
// Preserve that behavior here for compatibility with the accepted parity baseline.
let hostname = if raw.hostname.is_empty() {
    self::Hostname::catch_all()
} else {
    parse_hostname(raw.hostname.as_str())?
};
```

---

## 18. Style: Practical doc comments

Public doc comments should explain:

- what the item does
- what assumptions it makes
- what the caller must preserve
- how it should be used

Prefer plain language.

Good:

```rust
/// Normalizes a hostname into the repository's canonical ingress form.
///
/// This does not perform DNS validation.
fn normalize_hostname(input: &str) -> Result<Hostname, NormalizeError> {
```

Avoid:

- long tutorial prose
- decorative language
- repeating the function name in sentence form without useful content

---

## 19. Style: Quiet imports

Imports should be tidy and predictable.

Prefer:

- specific imports when they improve readability
- stable grouping
- minimal aliasing

Avoid:

- glob imports unless clearly justified
- aliases that make code harder to follow
- noisy import walls

Good:

```rust
use std::path::PathBuf;

use crate::config::TunnelConfig;
use crate::error::ConfigLoadError;
```

Less preferred:

```rust
use crate::*;
use std::*;
```

---

## 20. Style: Meaningful errors

Error names and messages should help the reader understand what failed.

Prefer:

- `InvalidHostname`
- `MissingCredentialFile`
- `UnsupportedProtocol`
- `ConfigLoadError`

Avoid:

- `Failure`
- `BadInput`
- `UnknownError`
- `InternalError`

Example:

Good:

```rust
return Err(ConfigLoadError::MissingCredentialFile);
```

Less preferred:

```rust
return Err(ConfigLoadError::Failure);
```

---

## 21. Style: Do not use `unwrap` in production code

Do not use `unwrap` in runtime, library, or production-path code.

If a failure must panic intentionally, prefer `expect` with a message that explains the invariant or assumption.

`unwrap` is acceptable in:

- tests
- temporary development tooling
- short-lived local experiments that are not production paths

Prefer:

```rust
let config = raw.parse::<self::TunnelConfig>()
    .expect("embedded test fixture must parse as TunnelConfig");
```

Over:

```rust
let config = raw.parse::<self::TunnelConfig>().unwrap();
```

In production code, prefer proper error propagation when possible:

```rust
let config = raw.parse::<self::TunnelConfig>()?;
```

---

## 22. Style: Tests read like behavior

Test names should describe behavior, not implementation trivia.

Prefer:

- `loads_default_origin_when_not_configured`
- `rejects_invalid_service_url`
- `preserves_rule_order_during_normalization`

Avoid:

- `test_1`
- `basic_case`
- `works`

Good:

```rust
#[test]
fn rejects_invalid_service_url() {
```

---

## 23. Style: Keep boolean names readable

Boolean names should read clearly at the call site.

Prefer:

- `is_enabled`
- `has_credentials`
- `should_reload`
- `was_loaded_from_disk`

Avoid:

- `flag`
- `check`
- `status`

Example:

Good:

```rust
if should_reload {
    reload_config()?;
}
```

Less preferred:

```rust
if flag {
    reload_config()?;
}
```

---

## 24. Style: Prefer positive conditions when practical

Prefer conditions that read directly.

Good:

```rust
if has_credentials {
    load_credentials()?;
}
```

Less preferred:

```rust
if !missing_credentials {
    load_credentials()?;
}
```

Avoid double negatives unless they are truly the clearest form.

---

## 25. Style: Keep method chains short when they carry meaning

Short chains are fine. Once a chain starts carrying multiple semantic steps, break it into named variables.

Good:

```rust
let hostname = raw.hostname.trim();
let hostname = normalize_hostname(hostname)?;
```

Less preferred:

```rust
let hostname = raw.hostname.trim().to_ascii_lowercase().parse::<self::Hostname>()?;
```

---

## 26. Style: Keep field order stable in struct construction

When constructing a struct, prefer the same field order as the type definition unless there is a strong readability reason not to.

This makes review easier because readers can compare faster.

Good:

```rust
Self {
    hostname,
    service,
    origin_request,
}
```

---

## 27. Style: One obvious local pattern is better than many equivalent ones

If the repository has a preferred local pattern, use it consistently.

Examples:

- prefer `Self` over repeating the type name inside its own `impl`
- prefer `self::` for sibling items
- prefer explicit parse types at the operation site
- prefer early guards over deep nesting

Consistency lowers review cost more than micro-optimizing every line.

---

## 28. Style: AI-generated code must be normalized

AI-generated code is only acceptable after it reads like repository-owned code.

Common AI drift signs:

- vague naming
- repetitive helper structure
- too many obvious comments
- mechanically correct but semantically weak naming
- dense combinators where direct control flow is clearer

Rule:

- valid Rust is not enough
- merged Rust must read like intentional repository code

---

## Quick rule of thumb

When choosing between two valid versions, prefer the one that is:

1. easier to understand in one pass
2. easier to review
3. more explicit about local intent
4. more consistent with surrounding code

Consistency is a feature.
