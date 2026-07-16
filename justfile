set shell := ["sh", "-cu"]

# Git Bash/MSYS injects a pseudo environment variable named `!::` on Windows.
# Pixi's activation environment capture trips over it, so strip it before every
# `pixi run` invocation.
pixi := "env -u '!::' pixi"

mod site

ci-fmt-check:
    cargo fmt --all --check

ci-clippy:
    cargo clippy --workspace --all-targets -- -D warnings

ci-test-lib:
    CARGO_TARGET_DIR=.cargo-target-tests cargo test --workspace --lib

ci-test:
    CARGO_TARGET_DIR=.cargo-target-tests cargo test --workspace

ci-check-release-tag-version:
    python scripts/check_release_tag_version.py

ci-check:
    just ci-fmt-check
    just ci-clippy
    just ci-test

ci-build:
    cargo build -p proxai --release

ci-release-notes:
    git-cliff --latest --output dist/release-notes.raw.md
    python scripts/polish_release_notes.py --input dist/release-notes.raw.md --output dist/release-notes.md

fmt:
    {{ pixi }} run -- cargo fmt --all

fmt_check:
    {{ pixi }} run -- rtk cargo fmt --all --check

clippy:
    {{ pixi }} run -- rtk cargo clippy --workspace --all-targets -- -D warnings

test_lib *args:
    CARGO_TARGET_DIR=.cargo-target-tests {{ pixi }} run -- rtk cargo test --workspace --lib {{ args }}

# List every `regression_*` test in the tree (real-world payload / observed-bug tests).
regression-list:
    CARGO_TARGET_DIR=.cargo-target-tests {{ pixi }} run -- rtk cargo test --workspace --lib -- --list 2>&1 | grep regression || true

# Run every `regression_*` test (real-world payload / observed-bug tests).
regression-run *args:
    CARGO_TARGET_DIR=.cargo-target-tests {{ pixi }} run -- rtk cargo test --workspace --lib regression {{ args }}

# Show which regression test files the working diff touches — review attention signal.
regression-touched:
    @git diff --name-only HEAD | grep "_regression_tests.rs" || echo "no regression tests touched"

check_release_tag_version:
    {{ pixi }} run -- python scripts/check_release_tag_version.py

check_update:
    {{ pixi }} run -- rtk cargo run -p proxai -- check-update

check:
    just fmt_check
    just clippy
    just test
    just protocol-compare
    just anthropic-protocol-compare

test:
    CARGO_TARGET_DIR=.cargo-target-tests {{ pixi }} run -- rtk cargo test --workspace

run *args:
    {{ pixi }} run -- cargo run -p proxai -- {{ args }}

run-capture *args:
    {{ pixi }} run -- cargo run -p proxai -- --capture-inbound-request --capture-provider-request --capture-upstream-response --capture-outbound-response {{ args }}

zed-probe *args:
    {{ pixi }} run -- python tools/zed_probe_server.py {{ args }}

build:
    {{ pixi }} run -- rtk cargo build -p proxai --release

test-e2e *args:
    CARGO_TARGET_DIR=.cargo-target-tests {{ pixi }} run -- rtk cargo test -p proxai --test proxy_e2e -- --nocapture {{ args }}

hooks-install:
    {{ pixi }} run -- lefthook install

capture-status:
    {{ pixi }} run -- cargo run -p proxai -- capture status

capture-enable:
    {{ pixi }} run -- cargo run -p proxai -- capture enable

capture-disable:
    {{ pixi }} run -- cargo run -p proxai -- capture disable

# Fast OpenAI protocol drift gate used by the full local check.
protocol-compare:
    {{ pixi }} run -- python -m unittest tools.protocol_compare.tests tools.openai_compare.tests
    {{ pixi }} run -- python tools/compare_openai_protocol.py --quiet --structural

# Fast Anthropic protocol drift gate used by the full local check.
anthropic-protocol-compare:
    {{ pixi }} run -- python -m unittest tools.protocol_compare.tests tools.anthropic_compare.tests
    {{ pixi }} run -- python tools/compare_anthropic_protocol.py --quiet

# Compare proxai Anthropic protocol types against official SDK with a detailed report.
compare-anthropic-protocol level="2":
    {{ pixi }} run -- python -m unittest tools.protocol_compare.tests tools.anthropic_compare.tests
    {{ pixi }} run -- python tools/compare_anthropic_protocol.py --level {{ level }}

# Compare proxai OpenAI protocol types against the official OpenAPI schema.
# Pass `--structural` for the manual schema-first coverage audit; this is not part of CI.
compare-openai-protocol *args:
    {{ pixi }} run -- python -m unittest tools.protocol_compare.tests tools.openai_compare.tests
    {{ pixi }} run -- python tools/compare_openai_protocol.py {{ args }}

# Alias for backward compatibility
compare-protocol: compare-anthropic-protocol

clean:
    {{ pixi }} run -- cargo clean
