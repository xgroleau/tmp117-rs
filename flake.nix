{
  description = "Rust dev shell";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        # Since we are cross compiling with arm-gcc-none, we don't want the CC and AR variable to bet  set in our shell
        rustpkg = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

      in with pkgs; {
        devShell = mkShell { buildInputs = [ probe-run rustpkg ]; };
      });
}
