set shell := ["pwsh", "-NoLogo", "-NoProfile", "-Command"]

fmt:
    pixi run -- cargo fmt

fmt_check:
    pixi run -- rtk cargo fmt --check

clippy:
    pixi run -- rtk cargo clippy --all-targets -- -D warnings

test_lib:
    $env:CARGO_TARGET_DIR = ".cargo-target-tests"; pixi run -- rtk cargo test --lib

check_release_tag_version:
    pixi run -- python scripts/check_release_tag_version.py

check_update:
    pixi run -- rtk cargo run -- check-update

check:
    just fmt_check
    just clippy
    just test

test:
    $env:CARGO_TARGET_DIR = ".cargo-target-tests"; pixi run -- rtk cargo test

run *args:
    pixi run -- cargo run -- {{ args }}

run-capture *args:
    pixi run -- cargo run -- --capture-inbound-request --capture-forwarded-request --capture-upstream-response --capture-outbound-response {{ args }}

zed-probe *args:
    pixi run -- python tools/zed_probe_server.py {{ args }}

build:
    pixi run -- rtk cargo build --release

test-e2e:
    $env:CARGO_TARGET_DIR = ".cargo-target-tests"; pixi run -- rtk cargo test --test proxy_e2e -- --nocapture

hooks-install:
    pixi run -- lefthook install

capture-status:
    pixi run -- cargo run -- capture status

capture-enable:
    pixi run -- cargo run -- capture enable

capture-disable:
    pixi run -- cargo run -- capture disable

# Compare proxai Anthropic protocol types against official SDK
compare-anthropic-protocol level="2":
    pixi run -- python tools/compare_anthropic_protocol.py --level {{level}}

# Compare proxai OpenAI protocol types against async-openai v0.40.2
compare-openai-protocol level="2":
    pixi run -- python tools/compare_openai_protocol.py --level {{level}}

# Alias for backward compatibility
compare-protocol: compare-anthropic-protocol

clean:
    pixi run -- cargo clean
