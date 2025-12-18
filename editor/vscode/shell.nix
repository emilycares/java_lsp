{
  pkgs ? import <nixpkgs> { },
}:
pkgs.mkShell {
  nativeBuildInputs = with pkgs.buildPackages; [
    pnpm
    nodejs
    javaPackages.compiler.openjdk25
  ];
}
