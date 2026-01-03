{
  description = "java_lsp";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.11";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        inherit (nixpkgs) lib;
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        msrvToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        msrvPlatform = pkgs.makeRustPlatform {
          cargo = msrvToolchain;
          rustc = msrvToolchain;
        };

        rustToolchain = pkgs.rust-bin.stable."1.92.0".default.override {
          # rustToolchain = pkgs.rust-bin.nightly."2025-11-02".default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
            "rustfmt"
          ];
        };
      in
      {
        packages = {
          java_lsp = pkgs.callPackage ./default.nix { inherit lib; };
          default = self.packages.${system}.java_lsp;
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
            nativeBuildInputs =
              with pkgs;
              [
                javaPackages.compiler.openjdk25
                lld_21
                gdb
                hyperfine
                cargo-flamegraph
                cargo-nextest
                cargo-insta
                just
              ]
              ++ (lib.optional (stdenv.isx86_64 && stdenv.isLinux) cargo-tarpaulin)
              ++ (lib.optional stdenv.isLinux lldb)
              ++ (lib.optional stdenv.isDarwin darwin.apple_sdk.frameworks.CoreFoundation);
            shellHook = ''
              export RUST_BACKTRACE="1"
              export RUSTFLAGS="''${RUSTFLAGS:-""}"
            '';
          };
          check_jdk = pkgs.mkShell {
            inputsFrom = [ ];

            nativeBuildInputs =
              with pkgs;
              [
                javaPackages.compiler.openjdk25
              ]
              ++ [
                self.checks.${system}.java_lsp
              ];
          };
        };

        overlays = {
          java_lsp = final: prev: {
            java_lsp = final.callPackage ./default.nix { inherit lib; };
          };

          default = self.overlays.java_lsp;
        };
      }
    );
}
