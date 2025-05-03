dev:
  cargo build

clippy: 
  cargo clippy --workspace --all-targets -- -D warnings

test:
  cargo nextest run --workspace --retries 2

wtest:
  watchexec -e rs cargo nextest run --workspace  --retries 2

fmt:
  cargo fmt

precommit: test fmt clippy dev
