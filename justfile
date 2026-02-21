dev:
  cargo build

clippy: 
  cargo clippy --workspace -- -Dwarnings

test:
  cargo-nextest nextest run --workspace

wtest:
  watchexec -e rs cargo nextest run --workspace

check:
  cargo check

fmt:
  cargo fmt

doc:
  typos
  cargo doc --no-deps --workspace

nix:
  nix flake check

jdk:
  cargo run --release -- ast-check-jdk

precommit: fmt check test clippy doc dev nix
