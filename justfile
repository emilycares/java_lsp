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
  cargo check --target x86_64-pc-windows-gnu

fmt:
  cargo fmt

doc:
  cargo doc --no-deps --workspace

precommit: fmt check test clippy doc dev
