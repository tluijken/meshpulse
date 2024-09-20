{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    rustup
    rustfmt
    clippy
    gcc
    pkg-config
    cmake
  ];

  buildInputs = with pkgs; [
    openssl
  ];
}
