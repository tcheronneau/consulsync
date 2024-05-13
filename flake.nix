{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils }:
  let
    name = "consulsync";
    version = "0.2.0";
  in
  flake-utils.lib.eachDefaultSystem (system:
    with nixpkgs.legacyPackages.${system}; {
      packages.consulsync = rustPlatform.buildRustPackage {
        name = "${name}";
        version = "${version}";

        src = lib.cleanSource ./.;

        cargoSha256 =
          "sha256-A6s3xdFC4RnIYpQ1BGKuL0Q3WEqlFuKmplGhbDb9VB8=";
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
        contents = [ self.packages.${system}.consulsync  cacert ];
        tag = "${system}-${version}";
        created = "now";
        config = {
          Cmd = [
            "${self.packages.${system}.consulsync}/bin/${name}"
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
          configFile = format.generate "config.toml" cfg.settings; 
        in {
          options.services.consulsync = {
            enable = mkOption {
              default = false;
              type = types.bool;
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
                    type = types.str;
                    default = "info";
                    description = "Log level";
                  };
                  consul = mkOption {
                    default = {};
                    type = types.submodule {
                      freeformType = format.type;
                      options = {
                        url = mkOption {
                          type = types.str;
                          default = "http://localhost:8500";
                          description = "Consul address";
                        };
                      };
                    };
                  };
                  external_kinds = mkOption {
                    type = types.listOf (types.submodule {
                      freeformType = format.type;
                      options = {
                        name = mkOption {
                          type = types.str;
                          description = "Service kind name";
                        };
                        filename = mkOption {
                          type = types.path;
                          description = "Service kind filename";
                        };
                      };
                    });
                  };
                  kinds = mkOption {
                    type = types.listOf (types.submodule {
                      freeformType = format.type;
                      options = {
                        name = mkOption {
                          type = types.str;
                          description = "Kind name";
                        };
                        tags = mkOption {
                          type = types.listOf types.str;
                          description = "Kind tags";
                        };
                      };
                    });
                  };
                  services = mkOption {
                    type = types.listOf (types.submodule {
                      freeformType = format.type;
                      options = {
                        name = mkOption {
                          type = types.str;
                          description = "Service name";
                        };
                        kind = mkOption {
                          type = types.str;
                          description = "Service kind";
                        };
                        address = mkOption {
                          type = types.str;
                          description = "Service address";
                        };
                        port = mkOption {
                          type = types.int;
                          description = "Service port";
                        };
                        tags = mkOption {
                          type = types.listOf types.str;
                          description = "Service tags";
                          default = [];
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
                ExecStart = "${getExe' cfg.package "consulsync"} -c ${configFile}";
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
