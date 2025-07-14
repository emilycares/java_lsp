{
  description = "java_lsp";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      inherit (nixpkgs) lib;
      pkgs = import nixpkgs {
        inherit system;
        overlays = [rust-overlay.overlays.default];
      };
      msrvToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      msrvPlatform = pkgs.makeRustPlatform {
        cargo = msrvToolchain;
        rustc = msrvToolchain;
      };

      rustToolchain = pkgs.rust-bin.stable."1.89.0".default.override {
        extensions = ["rust-src" "rust-analyzer" "rustfmt"];
      };
    in {
      packages = {
        default = pkgs.callPackage ./default.nix {inherit lib;};
      };
      checks = {
        java_lsp = self.packages.${system}.java_lsp.override {
          rustPlatform = msrvPlatform;
        };
      };

      devShells = {
        default = pkgs.mkShell {
          buildInputs = [
            rustToolchain
          ];
          nativeBuildInputs = with pkgs;
            [
              lld_21
              hyperfine
              cargo-flamegraph
              cargo-nextest
              cargo-insta
            ]
            ++ (lib.optional (stdenv.isx86_64 && stdenv.isLinux) cargo-tarpaulin)
            ++ (lib.optional stdenv.isLinux lldb)
            ++ (lib.optional stdenv.isDarwin darwin.apple_sdk.frameworks.CoreFoundation);
          shellHook = ''
            export RUST_BACKTRACE="1"
            export RUSTFLAGS="''${RUSTFLAGS:-""}"
          '';
        };
      };

      overlays = {
        java_lsp = final: prev: {
          java_lsp = final.callPackage ./default.nix {inherit lib;};
        };

        default = self.overlays.java_lsp;
      };
    });
}
