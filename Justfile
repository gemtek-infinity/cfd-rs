set shell := ["bash", "-euo", "pipefail", "-c"]
set positional-arguments := true


help:
    @printf '\033[1;36mcfd-rs command surface\033[0m\n\n'
    @printf '\033[1;33mCore\033[0m\n'
    @printf '  %-28s %s\n' 'help' 'Show this command map.'
    @printf '  %-28s %s\n' 'doctor' 'Check local toolchain and command prerequisites.'
    @printf '  %-28s %s\n' 'fmt' 'Run cargo +nightly fmt --all.'
    @printf '  %-28s %s\n' 'fmt-check' 'Run cargo +nightly fmt --all --check.'
    @printf '\n\033[1;33mValidation\033[0m\n'
    @printf '  %-28s %s\n' 'validate-governance' 'Validate docs, source-map drift, and contract-literal policy.'
    @printf '  %-28s %s\n' 'validate-app' 'Fmt-check plus app crate check/clippy/test.'
    @printf '  %-28s %s\n' 'validate-tools' 'Fmt-check plus MCP check/clippy/test/smoke.'
    @printf '  %-28s %s\n' 'validate-pr' 'Run governance, app, and tool validation.'
    @printf '\n\033[1;33mMCP\033[0m\n'
    @printf '  %-28s %s\n' 'mcp-run' 'Run the debtmap-enabled MCP server.'
    @printf '  %-28s %s\n' 'mcp-run-maintenance' 'Run the maintenance MCP surface without default features.'
    @printf '  %-28s %s\n' 'mcp-smoke' 'Smoke-start the operational MCP surface.'
    @printf '  %-28s %s\n' 'mcp-smoke-maintenance' 'Smoke-start the maintenance MCP surface.'
    @printf '\n\033[1;33mShared behavior\033[0m\n'
    @printf '  %-28s %s\n' 'shared-behavior-capture' 'Refresh checked-in Go truth artifacts.'
    @printf '  %-28s %s\n' 'shared-behavior-compare' 'Emit Rust actuals and compare against Go truth.'
    @printf '\n\033[1;33mPreview lane\033[0m\n'
    @printf '  %-28s %s\n' 'preview-test' 'Run the merge-workflow test gate.'
    @printf '  %-28s %s\n' 'preview-build <lane>' 'Build the release binary for x86-64-v2 or x86-64-v4.'
    @printf '  %-28s %s\n' 'preview-smoke <lane>' 'Run the deployment smoke for a built lane binary.'
    @printf '  %-28s %s\n' 'preview-package <lane>' 'Package a preview artifact tarball and checksum.'
    @printf '  %-28s %s\n' 'preview-all <lane>' 'Build, smoke, and package one preview lane.'

_doctor_check name cmd:
    @if command -v {{cmd}} >/dev/null 2>&1; then \
        printf '  %-16s %s\n' '{{name}}' "$(command -v {{cmd}})"; \
    else \
        printf '  %-16s %s\n' '{{name}}' 'missing'; \
    fi

doctor:
    @printf '\033[1;36mToolchain doctor\033[0m\n'
    @just _doctor_check rustc rustc
    @just _doctor_check cargo cargo
    @just _doctor_check just just
    @just _doctor_check python3 python3
    @just _doctor_check go go
    @just _doctor_check timeout timeout
    @printf '\n'
    @rustc -Vv
    @cargo -V
    @cargo +nightly fmt --version

fmt:
    cargo +nightly fmt --all

fmt-check:
    cargo +nightly fmt --all --check

validate-governance:
    python3 tools/generate_parity_source_map.py >/dev/null
    git diff --exit-code -- docs/parity/source-map.csv
    python3 tools/validate_phase5_docs.py
    python3 tools/validate_contract_literals.py

validate-app:
    just fmt-check
    cargo check --locked -p cfdrs-bin -p cfdrs-cdc -p cfdrs-cli -p cfdrs-his -p cfdrs-shared
    cargo clippy --all-targets --locked -p cfdrs-bin -p cfdrs-cdc -p cfdrs-cli -p cfdrs-his -p cfdrs-shared -- -D warnings
    cargo test --locked -p cfdrs-bin -p cfdrs-cdc -p cfdrs-cli -p cfdrs-his -p cfdrs-shared

validate-tools:
    just fmt-check
    cargo check --locked -p cfd-rs-memory --features debtmap
    cargo clippy --all-targets --locked -p cfd-rs-memory --features debtmap -- -D warnings
    cargo test --locked -p cfd-rs-memory --features debtmap
    just mcp-smoke
    cargo check --locked -p cfd-rs-memory --no-default-features
    cargo clippy --all-targets --locked -p cfd-rs-memory --no-default-features -- -D warnings
    cargo test --locked -p cfd-rs-memory --no-default-features
    just mcp-smoke-maintenance

validate-pr: validate-governance validate-app validate-tools

mcp-run:
    exec cargo run --locked --quiet --release --manifest-path tools/mcp-cfd-rs/Cargo.toml --features debtmap

mcp-run-maintenance:
    exec cargo run --locked --quiet --release --manifest-path tools/mcp-cfd-rs/Cargo.toml --no-default-features

mcp-smoke:
    tmp="$(mktemp)"; \
    set +e; \
    timeout 5s just mcp-run < /dev/null >"${tmp}" 2>&1; \
    status=$?; \
    set -e; \
    if [ "$status" -ne 0 ] && [ "$status" -ne 124 ] && ! grep -q 'mcp:ready' "${tmp}"; then \
        cat "${tmp}"; \
        rm -f "${tmp}"; \
        exit "$status"; \
    fi; \
    rm -f "${tmp}"

mcp-smoke-maintenance:
    tmp="$(mktemp)"; \
    set +e; \
    timeout 5s just mcp-run-maintenance < /dev/null >"${tmp}" 2>&1; \
    status=$?; \
    set -e; \
    if [ "$status" -ne 0 ] && [ "$status" -ne 124 ] && ! grep -q 'mcp:ready' "${tmp}"; then \
        cat "${tmp}"; \
        rm -f "${tmp}"; \
        exit "$status"; \
    fi; \
    rm -f "${tmp}"

shared-behavior-capture:
    python3 tools/shared_behavior_parity.py capture-go-truth

shared-behavior-compare:
    python3 tools/shared_behavior_parity.py emit-rust-actual
    python3 tools/shared_behavior_parity.py compare --require-go-truth --require-rust-actual

preview-test:
    cargo test --workspace --locked

preview-build lane:
    case "{{lane}}" in \
        x86-64-v2) export RUSTFLAGS='-C target-cpu=x86-64-v2 -C strip=symbols' ;; \
        x86-64-v4) export RUSTFLAGS='-C target-cpu=x86-64-v4 -C strip=symbols' ;; \
        *) echo "unsupported lane: {{lane}}" >&2; exit 1 ;; \
    esac; \
    cargo build --release --locked --target x86_64-unknown-linux-gnu -p cfdrs-bin

preview-smoke lane:
    BINARY='target/x86_64-unknown-linux-gnu/release/cloudflared'; \
    if [ ! -x "${BINARY}" ]; then \
        echo 'preview binary missing; run just preview-build {{lane}} first' >&2; \
        exit 1; \
    fi; \
    config="$(mktemp)"; \
    printf 'tunnel: 00000000-0000-0000-0000-000000000000\ningress:\n  - service: http_status:503\n' >"${config}"; \
    "${BINARY}" --config "${config}" validate | tee /tmp/cfdrs-validate-output.txt; \
    grep -q 'OK: admitted alpha startup surface validated' /tmp/cfdrs-validate-output.txt; \
    "${BINARY}" --config "${config}" run >/tmp/cfdrs-run-output.txt 2>&1 || true; \
    grep -q 'deploy-contract:' /tmp/cfdrs-run-output.txt; \
    grep -q 'deploy-host-validation:' /tmp/cfdrs-run-output.txt; \
    grep -q 'deploy-known-gaps:' /tmp/cfdrs-run-output.txt; \
    grep -q 'deploy-evidence-scope:' /tmp/cfdrs-run-output.txt; \
    rm -f "${config}"

preview-package lane:
    case "{{lane}}" in \
        x86-64-v2|x86-64-v4) ;; \
        *) echo "unsupported lane: {{lane}}" >&2; exit 1 ;; \
    esac; \
    binary='target/x86_64-unknown-linux-gnu/release/cloudflared'; \
    if [ ! -x "${binary}" ]; then \
        echo 'preview binary missing; run just preview-build {{lane}} first' >&2; \
        exit 1; \
    fi; \
    artifact_base="cloudflared-${GITHUB_SHA:-$(git rev-parse HEAD)}-linux-x86_64-gnu-{{lane}}"; \
    rm -rf dist; \
    mkdir -p dist; \
    install -Dm755 "${binary}" 'dist/cloudflared'; \
    if [ -f README.md ]; then cp README.md dist/; fi; \
    if [ -f LICENSE ]; then cp LICENSE dist/; fi; \
    tar -C dist -czf "${artifact_base}.tar.gz" .; \
    sha256sum "${artifact_base}.tar.gz" > "${artifact_base}.tar.gz.sha256"

preview-all lane:
    just preview-build {{lane}}
    just preview-smoke {{lane}}
    just preview-package {{lane}}
