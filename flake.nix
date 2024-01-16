{
  description = "";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    alejandra = {
      url = "github:kamadorueda/alejandra";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    ocapn-test-suite = {
      url = "github:ocapn/ocapn-test-suite";
      flake = false;
    };
  };
  outputs = inputs @ {
    self,
    nixpkgs,
    ...
  }:
    with builtins; let
      std = nixpkgs.lib;
      systems = ["x86_64-linux"];
      nixpkgsFor = std.genAttrs systems (system:
        import nixpkgs {
          localSystem = builtins.currentSystem or system;
          crossSystem = system;
          overlays = [];
        });
    in {
      formatter = std.mapAttrs (system: pkgs: pkgs.default) inputs.alejandra.packages;
      devShells = std.genAttrs systems (system: let
        pkgs = nixpkgsFor.${system};
      in {
        default = pkgs.mkShell (let
          python = pkgs.python312;
        in {
          nativeBuildInputs = [
            pkgs.pkg-config
          ];
          buildInputs = [
            pkgs.sqlite
            pkgs.openssl
            pkgs.zlib
          ];
          packages = [
            python
          ];
          env.PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig:${pkgs.sqlite.dev}/lib/pkgconfig";
          env.REXA_OCAPN_TEST_SUITE_DIR = toString inputs.ocapn-test-suite;
          env.REXA_PYTHON_PATH = "${python}/bin/python";
          LD_LIBRARY_PATH = std.concatStringsSep ":" ["${pkgs.sqlite.out}/lib" "${pkgs.openssl.out}/lib"];
        });
      });
    };
}
