{
  description = "precious — cloud infrastructure cost estimator";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs @ {
    self,
    nixpkgs,
    crane,
    rust-overlay,
    ...
  }: let
    allSystems = ["x86_64-linux" "aarch64-linux" "aarch64-darwin"];
    forAllSystems = f:
      nixpkgs.lib.genAttrs allSystems (system:
        f {
          pkgs = import nixpkgs {
            inherit system;
            overlays = [(import rust-overlay)];
          };
          inherit system;
        });
  in {
    packages = forAllSystems ({pkgs, system}: let
      craneLib = (crane.mkLib pkgs).overrideToolchain (p:
        p.rust-bin.stable.latest.default.override {
          extensions = ["rust-src" "rust-std" "clippy" "rustfmt" "rust-analyzer"];
        });
      unfilteredRoot = ./.;

      src = pkgs.lib.fileset.toSource {
        root = unfilteredRoot;
        fileset = pkgs.lib.fileset.unions [
          (craneLib.fileset.commonCargoSources unfilteredRoot)
        ];
      };

      commonArgs = {
        inherit src;
        pname = "precious";
        version = "0.1.0";
        strictDeps = true;

        nativeBuildInputs = [
          pkgs.pkg-config
        ];

        buildInputs =
          pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
            pkgs.apple-sdk_15
          ];
      };

      cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      precious = craneLib.buildPackage (
        commonArgs
        // {
          inherit cargoArtifacts;
          meta = {
            description = "Cloud infrastructure cost estimator";
            homepage = "https://github.com/polymath-as/precious";
            license = pkgs.lib.licenses.asl20;
            mainProgram = "precious";
          };
        }
      );
    in {
      default = precious;
      precious = precious;
    });

    devShells = forAllSystems ({pkgs, system}: let
      craneLib = (crane.mkLib pkgs).overrideToolchain (p:
        p.rust-bin.stable.latest.default.override {
          extensions = ["rust-src" "rust-std" "clippy" "rustfmt" "rust-analyzer"];
        });
    in {
      default = craneLib.devShell {
        buildInputs = [];
        packages = with pkgs; [];
      };
    });
  };
}
