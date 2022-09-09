{ pkgs ? import <nixpkgs> { } }:
with pkgs; mkShell {
  buildInputs = [
    (yarn.override { nodejs = nodejs-16_x; })
  ];
}
