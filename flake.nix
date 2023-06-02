{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        app = pkgs.rustPlatform.buildRustPackage {
          pname = "mercury";
          version = "0.0.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = with pkgs; [
            pkg-config
          ];
          buildInputs = with pkgs; [
            openssl
          ];
        };

        img = pkgs.dockerTools.buildLayeredImage {
          name = "mercury";
          tag = "latest";
          contents = [ app pkgs.cacert ];
          config.Cmd = [ "${app}/bin/mercury" ];
        };
      in
      {
        devShells = rec {
          default = pkgs.mkShell {
            inputsFrom = [ app ];

            nativeBuildInputs = with pkgs; [
              clippy
              rustfmt
            ];
          };

          ops = pkgs.mkShell {
            inputsFrom = [ app ];

            nativeBuildInputs = with pkgs; [
              heroku
            ];
          };
        };

        packages = {
          default = app;
          dockerImage = img;
        };
      });
}
