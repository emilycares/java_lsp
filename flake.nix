{
  description = "java_lsp";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.11";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      flake-utils,
      rust-overlay,
      advisory-db,
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

        craneLib = (crane.mkLib pkgs).overrideToolchain (
          p:
          p.rust-bin.stable."1.93.0".default.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
              "rustfmt"
            ];
            targets = [ "x86_64-pc-windows-gnu" ];
          }
        );

        src = ./.;

        commonArgs = {
          inherit src;
          strictDeps = true;

          buildInputs = [
            # Add additional build inputs here
          ]
          ++ lib.optionals pkgs.stdenv.isDarwin [
            # Additional darwin specific inputs can be set here
            pkgs.libiconv
          ];

          # Additional environment variables can be set directly
          # MY_CUSTOM_VAR = "some value";
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        individualCrateArgs = commonArgs // {
          inherit cargoArtifacts;
          inherit (craneLib.crateNameFromCargoToml { cargoToml = ./crates/java_lsp/Cargo.toml; }) version;
          # NB: we disable tests since we'll run them all via cargo-nextest
          doCheck = false;
        };

        fileSetForCrate =
          crate:
          lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              ./Cargo.toml
              ./Cargo.lock
              (craneLib.fileset.commonCargoSources ./crates/ast)
              (craneLib.fileset.commonCargoSources ./crates/call_chain)
              (craneLib.fileset.commonCargoSources ./crates/cli)
              (craneLib.fileset.commonCargoSources ./crates/common)
              (craneLib.fileset.commonCargoSources ./crates/compile)
              (craneLib.fileset.commonCargoSources ./crates/config)
              (craneLib.fileset.commonCargoSources ./crates/document)
              (craneLib.fileset.commonCargoSources ./crates/format)
              (craneLib.fileset.commonCargoSources ./crates/get_class)
              (craneLib.fileset.commonCargoSources ./crates/gradle)
              (craneLib.fileset.commonCargoSources ./crates/imports)
              (craneLib.fileset.commonCargoSources ./crates/java_lsp)
              (craneLib.fileset.commonCargoSources ./crates/jdk)
              (craneLib.fileset.commonCargoSources ./crates/loader)
              (craneLib.fileset.commonCargoSources ./crates/java_lsp)
              (craneLib.fileset.commonCargoSources ./crates/maven)
              (craneLib.fileset.commonCargoSources ./crates/my_string)
              (craneLib.fileset.commonCargoSources ./crates/parser)
              (craneLib.fileset.commonCargoSources ./crates/position)
              (craneLib.fileset.commonCargoSources ./crates/server)
              (craneLib.fileset.commonCargoSources ./crates/tyres)
              (craneLib.fileset.commonCargoSources ./crates/variables)
              (craneLib.fileset.commonCargoSources ./crates/zip_util)
              (craneLib.fileset.commonCargoSources ./crates/lsp_extra)
              (craneLib.fileset.commonCargoSources ./crates/workspace_hack)
              (craneLib.fileset.commonCargoSources crate)
            ];
          };

        java_lsp = craneLib.buildPackage (
          individualCrateArgs
          // {
            pname = "java_lsp";
            cargoExtraArgs = "-p java_lsp";
            src = fileSetForCrate ./crates/java_lsp;
          }
        );
      in
      {
        checks = {
          # Build the crates as part of `nix flake check` for convenience
          inherit java_lsp;

          # Run clippy (and deny all warnings) on the workspace source,
          # again, reusing the dependency artifacts from above.
          #
          # Note that this is done as a separate derivation so that
          # we can block the CI if there are issues here, but not
          # prevent downstream consumers from building our crate by itself.
          workspace-clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "";
            }
          );

          workspace-doc = craneLib.cargoDoc (
            commonArgs
            // {
              inherit cargoArtifacts;
              # This can be commented out or tweaked as necessary, e.g. set to
              # `--deny rustdoc::broken-intra-doc-links` to only enforce that lint
              env.RUSTDOCFLAGS = "--deny warnings";
            }
          );

          # Check formatting
          workspace-fmt = craneLib.cargoFmt {
            inherit src;
          };

          # workspace-toml-fmt = craneLib.taploFmt {
          #   src = pkgs.lib.sources.sourceFilesBySuffices src [ ".toml" ];
          #   # taplo arguments can be further customized below as needed
          #   # taploExtraArgs = "--config ./taplo.toml";
          # };

          # Audit dependencies
          workspace-audit = craneLib.cargoAudit {
            inherit src advisory-db;
          };

          # Run tests with cargo-nextest
          # Consider setting `doCheck = false` on other crate derivations
          # if you do not want the tests to run twice
          workspace-nextest = craneLib.cargoNextest (
            commonArgs
            // {
              inherit cargoArtifacts;
              partitions = 1;
              partitionType = "count";
              cargoNextestPartitionsExtraArgs = "-- --skip integration";
            }
          );

          # Ensure that cargo-hakari is up to date
          workspace-hakari = craneLib.mkCargoDerivation {
            inherit src;
            pname = "workspace-hakari";
            cargoArtifacts = null;
            doInstallCargoArtifacts = false;

            buildPhaseCargoCommand = ''
              cargo hakari generate --diff  # workspace-hack Cargo.toml is up-to-date
              cargo hakari manage-deps --dry-run  # all workspace crates depend on workspace-hack
              cargo hakari verify
            '';

            nativeBuildInputs = [
              pkgs.cargo-hakari
            ];
          };
        };
        packages = {
          inherit java_lsp;
        };

        devShells = {
          default = craneLib.devShell {
            # Inherit inputs from checks.
            checks = self.checks.${system};

            # Additional dev-shell environment variables can be set directly
            # MY_CUSTOM_DEVELOPMENT_VAR = "something else";

            shellHook = ''
              export RUST_BACKTRACE="1"
              export RUSTFLAGS="''${RUSTFLAGS:-""}"
            '';

            # Extra inputs can be added here; cargo and rustc are provided by default.
            packages =
              with pkgs;
              [
                cargo-hakari
                cargo-flamegraph
                cargo-nextest
                cargo-insta
                javaPackages.compiler.openjdk25
                lld_21
                gdb
                hyperfine
                just
                typos
              ]
              ++ (lib.optional (stdenv.isx86_64 && stdenv.isLinux) cargo-tarpaulin)
              ++ (lib.optional stdenv.isLinux lldb)
              ++ (lib.optional stdenv.isDarwin darwin.apple_sdk.frameworks.CoreFoundation);
          };
          check_jdk = craneLib.devShell {
            # Inherit inputs from checks.
            checks = self.checks.${system};

            packages = [
              java_lsp
              pkgs.javaPackages.compiler.openjdk25
            ];
          };
        };
      }
    );
}
