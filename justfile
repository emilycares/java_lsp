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

audit:
  cargo audit fix
  cd ./editor/vscode && pnpm audit --fix
  cd ./editor/vscode && pnpm install
  cd ./editor/vscode && pnpm audit
  cd ./editor/vscode/client && npm audit fix
  cd ./editor/vscode/client && npm audit

cleanup:
  cargo-machete || true

precommit: fmt check test clippy doc dev cleanup nix 
