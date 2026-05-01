set shell := ["pwsh", "-NoLogo", "-NoProfile", "-Command"]

fmt:
    pixi run -- cargo fmt

check:
    pixi run -- cargo fmt --check
    pixi run -- cargo clippy --all-targets -- -D warnings
    pixi run -- cargo test

test:
    pixi run -- cargo test

run *args:
    pixi run -- cargo run -- {{args}}

build:
    pixi run -- cargo build --release

test-e2e:
    pixi run -- cargo test --test proxy_e2e -- --nocapture

repro-system-role *args:
    pixi run python tests/repro/repro_system_role.py {{args}}

clean:
    pixi run -- cargo clean
