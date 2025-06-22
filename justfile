dev:
  cargo build

clippy: 
  cargo clippy --workspace --all-targets -- -D warnings

test:
  cargo-nextest nextest run --workspace --retries 2

wtest:
  watchexec -e rs cargo nextest run --workspace  --retries 2

check:
  cargo check

fmt:
  cargo fmt

precommit: check test fmt clippy dev
