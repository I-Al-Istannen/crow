{
  description = "A suite for testing compiler submissions";

  inputs = {
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    naersk.inputs.nixpkgs.follows = "nixpkgs";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
    gitignore.url = "github:hercules-ci/gitignore.nix";
    gitignore.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      self,
      gitignore,
      nixpkgs,
      naersk,
      fenix,
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
          inherit (gitignore.lib) gitignoreSource gitignoreFilterWith;

          backend-naersk = naersk'.buildPackage {
            version = (pkgs.lib.importTOML ./Cargo.toml).workspace.package.version;
            src = pkgs.lib.cleanSourceWith {
              filter = gitignoreFilterWith {
                basePath = ./.;
                extraRules = ''
                  /frontend/
                  /.github/
                  /assets
                  /README.md
                  /deploy
                '';
              };
              src = ./.;
            };

            buildInputs = [
              pkgs.dbus
            ];

            nativeBuildInputs = [
              pkgs.pkg-config
            ];
          };
          toolchain =
            with fenix.packages.${system};
            combine [
              stable.rustc
              stable.cargo
              targets.x86_64-unknown-linux-musl.stable.rust-std
            ];

          naersk-static = naersk.lib.${system}.override {
            cargo = toolchain;
            rustc = toolchain;
          };
          executor-naersk-static = naersk-static.buildPackage {
            inherit (backend-naersk) version src;
            CARGO_BUILD_TARGET = "x86_64-unknown-linux-musl";
            CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static";
            cargoBuildOptions =
              x:
              x
              ++ [
                "-p"
                "executor"
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
          executor-static = pkgs.runCommand "executor" { } ''
            mkdir -p $out/bin
            cp ${executor-naersk-static}/bin/executor $out/bin
          '';
          client = pkgs.runCommand "client" { } ''
            mkdir -p $out/bin
            cp ${backend-naersk}/bin/client $out/bin
          '';
          frontend = pkgs.stdenv.mkDerivation (finalAttrs: {
            pname = "frontend";
            version = backend-naersk.version;

            src = gitignoreSource ./frontend;

            nativeBuildInputs = [
              pkgs.nodejs
              pkgs.pnpm_9.configHook
            ];

            buildPhase = ''
              runHook preBuild
              pnpm build
              runHook postBuild
            '';

            installPhase = ''
              runHook preInstall
              mkdir $out
              cp -r dist/* $out
              runHook postInstall
            '';

            pnpmDeps = pkgs.pnpm_9.fetchDeps {
              inherit (finalAttrs) pname version src;
              hash = "sha256-fkxNt+EVNb1PL4Y0HTr2F5D9jOi2LzVHjkCw/VgHx7A=";
            };
          });
          docker = {
            backend = pkgs.dockerTools.buildLayeredImage {
              name = "crow-backend";
              tag = backend-naersk.version;

              contents = [
                pkgs.cacert
                pkgs.sqlite
                pkgs.busybox
                pkgs.git
                pkgs.bash
                pkgs.openssh
              ];

              # https://discourse.nixos.org/t/dockertools-buildimage-and-user-writable-tmp/5397/9
              extraCommands = "mkdir -m 0777 tmp";

              fakeRootCommands = ''
                ${pkgs.dockerTools.shadowSetup}
              '';
              enableFakechroot = true;

              config = {
                Entrypoint = [ "${backend}/bin/backend-web" ];
                # The config uses the local timezone
                Env = [ "TZDIR=${pkgs.tzdata}/share/zoneinfo" ];

                Expose = {
                  "3000/tcp" = { };
                };
              };
            };
            frontend =
              let
                lighttpd-config = pkgs.writeText "lighttpd.conf" ''
                  server.document-root = "${frontend}"
                  server.error-handler-404 = "/index.html"
                  server.port = 80
                  server.upload-dirs = ( "/tmp" )
                  index-file.names    = ( "index.html" )
                '';
              in
              pkgs.dockerTools.buildLayeredImage {
                name = "crow-frontend";
                tag = backend-naersk.version;

                extraCommands = "mkdir -m 0777 tmp";

                config = {
                  Entrypoint = [
                    "${pkgs.lighttpd}/bin/lighttpd"
                    "-f"
                    "${lighttpd-config}"
                    "-D"
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
