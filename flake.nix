{
  description = "A suite for testing compiler submissions";

  inputs = {
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    naersk.inputs.nixpkgs.follows = "nixpkgs";
    gitignore.url = "github:hercules-ci/gitignore.nix";
    gitignore.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      self,
      gitignore,
      nixpkgs,
      naersk,
    }:
    let
      forAllSystems = nixpkgs.lib.genAttrs nixpkgs.lib.systems.flakeExposed;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          naersk' = pkgs.callPackage naersk { };
          inherit (gitignore.lib) gitignoreSource;

          backend-naersk = naersk'.buildPackage {
            version = (pkgs.lib.importTOML ./Cargo.toml).workspace.package.version;
            src = gitignoreSource ./.;

            buildInputs = [
              pkgs.dbus
            ];

            nativeBuildInputs = [
              pkgs.pkg-config
            ];
          };
        in
        rec {
          backend = pkgs.runCommand "backend-web" { } ''
            mkdir -p $out/bin
            cp ${backend-naersk}/bin/backend-web $out/bin
          '';
          executor = pkgs.runCommand "executor" { } ''
            mkdir -p $out/bin
            cp ${backend-naersk}/bin/executor $out/bin
          '';
          client = pkgs.runCommand "client" { } ''
            mkdir -p $out/bin
            cp ${backend-naersk}/bin/client $out/bin
          '';
          frontend = pkgs.buildNpmPackage {
            pname = "frontend";
            version = backend-naersk.version;
            src = gitignoreSource ./frontend;
            npmDepsHash = "sha256-vzf8Y6qo+KFe8g4/IKe1357cbq0TkByn+iEM/YZ3EHU=";

            installPhase = ''
              mkdir $out
              cp -r dist/* $out
            '';
          };
          docker = {
            backend = pkgs.dockerTools.buildLayeredImage {
              name = "crow-backend";
              tag = backend-naersk.version;

              contents = [
                pkgs.cacert
                pkgs.sqlite
                pkgs.coreutils
              ];

              config = {
                Entrypoint = [ "${backend}/bin/backend-web" ];

                Expose = {
                  "3000/tcp" = { };
                };
              };
            };
            frontend =
              let
                caddy-config = pkgs.writeText "Caddyfile" ''
                  :80 {
                    try_files {path} /
                    encode gzip
                    root * ${frontend}
                    file_server
                  }
                '';
              in
              pkgs.dockerTools.buildLayeredImage {
                name = "crow-frontend";
                tag = backend-naersk.version;

                config = {
                  Entrypoint = [
                    "${pkgs.caddy}/bin/caddy"
                    "run"
                    "--adapter"
                    "caddyfile"
                    "--config"
                    caddy-config
                  ];

                  Expose = {
                    "80/tcp" = { };
                  };
                };
              };
          };
        }
      );

      formatter.x86_64-linux = nixpkgs.legacyPackages.x86_64-linux.nixfmt-rfc-style;
    };
}
