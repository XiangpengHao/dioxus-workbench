{
  description = "dioxus-workbench — a dockable panel workbench library for Dioxus";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ] (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
          ];
          targets = [ "wasm32-unknown-unknown" ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = [
            rustToolchain
            # `dx` from nixpkgs is built with `no-downloads`: it resolves
            # wasm-bindgen/wasm-opt from PATH instead of downloading prebuilt
            # binaries (which don't run on NixOS).
            pkgs.dioxus-cli
            pkgs.binaryen # wasm-opt, used by `dx build --release`
          ];
        };
      }
    );
}
