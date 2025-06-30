{
  description = "Flake for Logos-TUI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, crane, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay)  ];
          config.allowUnfree = true;
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain
          (pkgs.rust-bin.nightly.latest.default.override {
            extensions = [ "rust-src" ];
          });
        src = craneLib.cleanCargoSource ./.;

        cargoArtifacts = craneLib.buildDepsOnly {
          inherit src;
        };

        logos-tui = craneLib.buildPackage {
          pname = "logos-tui";
          version = "0.1.0";
          inherit src cargoArtifacts;

          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.openssl ];

          doCheck = false;
        };
      in {
        packages.default = logos-tui;

        apps.default = flake-utils.lib.mkApp {
          drv = logos-tui;
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ logos-tui ];
          nativeBuildInputs = with pkgs; [
            pkg-config
            cargo
            rust-analyzer
            cargo-generate
          ];
        };
      });
}
