{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils }:
  let
    name = "consul";
    version = "0.1.0";
  in
  flake-utils.lib.eachDefaultSystem (system:
    with nixpkgs.legacyPackages.${system}; {
      packages.consul = rustPlatform.buildRustPackage {
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
      defaultPackage = self.packages.${system}.consul;
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
    });
}
