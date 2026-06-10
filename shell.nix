{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  buildInputs = [
    pkgs.chafa
    pkgs.pkg-config
    pkgs.rustup
    pkgs.glib
  ];
}
