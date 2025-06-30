{
  description = "java_lsp";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    ...
  }: let
    inherit (nixpkgs) lib;
    systems = [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ];
    eachSystem = lib.genAttrs systems;
    pkgsFor = eachSystem (system:
      import nixpkgs {
        localSystem.system = system;
        overlays = [(import rust-overlay) self.overlays.java_lsp];
      });
  in {
    packages = eachSystem (system: {
      inherit (pkgsFor.${system}) java_lsp;
      /*
      The default java_lsp build. Uses the latest stable Rust toolchain, and unstable
      nixpkgs.

      The build inputs can be overridden with the following:

      packages.${system}.default.override { rustPlatform = newPlatform; };

      Overriding a derivation attribute can be done as well:

      packages.${system}.default.overrideAttrs { buildType = "debug"; };
      */
      default = self.packages.${system}.java_lsp;
    });
    checks =
      lib.mapAttrs (system: pkgs: let
        # Get java_lsp's MSRV toolchain to build with by default.
        msrvToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        msrvPlatform = pkgs.makeRustPlatform {
          cargo = msrvToolchain;
          rustc = msrvToolchain;
        };
      in {
        java_lsp = self.packages.${system}.java_lsp.override {
          rustPlatform = msrvPlatform;
        };
      })
      pkgsFor;

    # Devshell behavior is preserved.
    devShells =
      lib.mapAttrs (system: pkgs: {
        default = pkgs.mkShell {
          nativeBuildInputs = with pkgs;
            [
              lld
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
      })
      pkgsFor;

    overlays = {
      java_lsp = final: prev: {
        java_lsp = final.callPackage ./default.nix {inherit lib;};
      };

      default = self.overlays.java_lsp;
    };
  };
}
