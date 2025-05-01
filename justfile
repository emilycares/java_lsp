dev:
  cargo build

clippy: 
  cargo clippy --workspace --all-targets -- -D warnings

test:
  cargo nextest run --workspace

wtest:
  watchexec -e rs cargo nextest run --workspace

fmt:
  cargo fmt

precommit: fmt clippy test dev
