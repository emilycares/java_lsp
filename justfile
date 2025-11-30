dev:
  cargo build

clippy: 
  cargo clippy --workspace --all-targets -- -D warnings

test:
  cargo-nextest nextest run --workspace

wtest:
  watchexec -e rs cargo nextest run --workspace

check:
  cargo check

fmt:
  cargo fmt

precommit: check test fmt clippy dev
