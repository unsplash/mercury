{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config.allowUnfreePredicate = pkg: nixpkgs.lib.getName pkg == "ngrok";
        };

        darwinDeps = with pkgs; lib.optionals stdenv.isDarwin [
          darwin.apple_sdk.frameworks.Security
        ];

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
          ] ++ darwinDeps;
        };

        img = pkgs.dockerTools.streamLayeredImage {
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

          webhooks = pkgs.mkShell {
            inputsFrom = [ default ];

            nativeBuildInputs = with pkgs; [
              heroku
              ngrok
            ];
          };

          ops = pkgs.mkShell {
            inputsFrom = [ app ];

            nativeBuildInputs = with pkgs; [
              awscli2
            ];
          };
        };

        packages = {
          default = app;
          dockerImage = img;
        };
      });
}
