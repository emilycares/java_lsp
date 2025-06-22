{
  lib,
  rustPlatform,
  git,
  installShellFiles,
}: let
  fs = lib.fileset;
in
  rustPlatform.buildRustPackage (self: {
    cargoLock = {
      lockFile = ./Cargo.lock;
      # This is not allowed in nixpkgs but is very convenient here: it allows us to
      # avoid specifying `outputHashes` here for any git dependencies we might take
      # on temporarily.
      allowBuiltinFetchGit = true;
    };

    nativeBuildInputs = [
      installShellFiles
      git
    ];

    buildType = "release";

    name = "java_lsp";
    src = fs.toSource {
      root = ./.;
      fileset = fs.gitTracked ./.;
    };

    meta.mainProgram = "java_lsp";
  })
