{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils }:
  let
    name = "consulsync";
    version = "0.1.0";
  in
  flake-utils.lib.eachDefaultSystem (system:
    with nixpkgs.legacyPackages.${system}; {
      packages.consulsync = rustPlatform.buildRustPackage {
        name = "${name}";
        version = "${version}";

        src = lib.cleanSource ./.;

        cargoSha256 =
          "sha256-Kyrqm1WeygIyFvLMXhvaLGE8E2ef/ZW+676q00QO8zs=";
        nativeBuildInputs = [
          rustc
          cargo
          pkg-config
          openssl.dev
        ];
        buildInputs = [
          openssl.dev
        ];
      };
      packages.docker = dockerTools.buildLayeredImage {
        name = "mcth/${name}";
        contents = [ self.packages.${system}.consul  cacert ];
        tag = "${system}-${version}";
        created = "now";
        config = {
          Cmd = [
            "${self.packages.${system}.consul}/bin/${name}"
          ];
        };
      };
      defaultPackage = self.packages.${system}.consulsync;
      devShell = mkShell {
        inputsFrom = builtins.attrValues self.packages.${system};

        buildInputs = [
          rustc
          rustfmt
          rust-analyzer
          cargo
          pkg-config
          openssl.dev
        ];
      };
      nixosModules.consulsync = { config, lib, pkgs, ...}:
        with lib;
        let 
          cfg = config.services.consulsync;
          format = pkgs.formats.toml { };
          config_file = format.generate "config.toml" cfg.settings; 
        in {
          options.services.consulsync = {
            enable = lib.mkOption {
              default = false;
              type = lib.types.bool;
              description = "Enable consul sync";
            };
            package = mkOption {
              type = types.package;
              default = self.packages.${system}.consulsync;
              description = "Consulsync package";
            };
            settings = mkOption {
              type = types.submodule {
                freeformType = format.type;
                options = {
                  log_level = mkOption {
                    type = types.string;
                    default = "info";
                    description = "Log level";
                  };
                  consul = mkOption {
                    type = types.submodule {
                      freeformType = format.type;
                      options = {
                        url = mkOption {
                          type = types.string;
                          default = "http://localhost:8500";
                          description = "Consul address";
                        };
                      };
                    };
                  };
                  services = mkOption {
                    type = types.listOf (types.submodule {
                      default = [];
                      freeformType = format.type;
                      options = {
                        name = mkOption {
                          type = types.string;
                          description = "Service name";
                        };
                        kind = mkOption {
                          type = types.string;
                          description = "Service kind";
                        };
                        address = mkOption {
                          type = types.string;
                          description = "Service address";
                        };
                        port = mkOption {
                          type = types.int;
                          description = "Service port";
                        };
                        tags = mkOption {
                          type = types.listOf types.string;
                          description = "Service tags";
                        };
                      };
                    });
                  };
                };
              };
            };
          };
          config = mkIf cfg.enable {
            systemd.services.consulsync = {
              description = "Consul sync service";
              after = [ "network.target" ];
              wantedBy = [ "multi-user.target" ];
              serviceConfig = {
                User = "consulsync";
                Group = "consulsync";
                Type = "simple";
                ExecStart = "${getExe cfg.package} -d -c ${configFile}";
                ExecReload = "${pkgs.coreutils}/bin/kill -SIGHUP $MAINPID";
                KillSignal = "SIGINT";
                TimeoutStopSec = "30s";
                Restart = "on-failure";
              };
            };
            users.users.consulsync = {
                  group = "consulsync";
                  isSystemUser = true;
              };
            users.groups.consulsync = {};
            meta.maintainers = with maintainers; [ tcheronneau ];
          };
      };
  });
}
