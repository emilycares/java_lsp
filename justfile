dev:
  cargo build

clippy: 
  cargo clippy --workspace

test:
  cargo-nextest nextest run --workspace

wtest:
  watchexec -e rs cargo nextest run --workspace

check:
  cargo check

fmt:
  cargo fmt

doc:
  cargo doc --no-deps --workspace

precommit: check test fmt clippy doc dev
