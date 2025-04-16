{
  description = "";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane = {
      url = "github:ipetkov/crane";
    };
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = inputs @ {
    self,
    nixpkgs,
    ...
  }:
    with builtins; let
      std = nixpkgs.lib;

      systems = attrNames inputs.crane.packages;
      nixpkgsFor = std.genAttrs systems (system:
        import nixpkgs {
          localSystem = builtins.currentSystem or system;
          crossSystem = system;
          overlays = [inputs.rust-overlay.overlays.default];
        });

      toolchainToml = fromTOML (readFile ./rust-toolchain.toml);
      toolchainFor = std.mapAttrs (system: pkgs: pkgs.rust-bin.fromRustupToolchain toolchainToml.toolchain) nixpkgsFor;

      craneFor = std.mapAttrs (system: pkgs: (inputs.crane.mkLib pkgs).overrideToolchain toolchainFor.${system}) nixpkgsFor;

      stdenvFor = std.mapAttrs (system: pkgs: pkgs.stdenvAdapters.useMoldLinker pkgs.llvmPackages_latest.stdenv) nixpkgsFor;

      commonArgsFor =
        std.mapAttrs (system: pkgs: let
          crane = craneFor.${system};
        in {
          src = crane.cleanCargoSource (crane.path ./.);
          stdenv = stdenvFor.${system};
          strictDeps = true;
          nativeBuildInputs = with pkgs; [];
          buildInputs = with pkgs; [];
        })
        nixpkgsFor;

      cargoToml = fromTOML (readFile ./Cargo.toml);
      name = cargoToml.package.metadata.crane.name or cargoToml.package.name or cargoToml.workspace.metadata.crane.name;
    in {
      formatter = std.mapAttrs (system: pkgs: pkgs.nixfmt-rfc-style) nixpkgsFor;
      devShells =
        std.mapAttrs (system: pkgs: let
          toolchain = toolchainFor.${system}.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
              "rustfmt"
              "clippy"
            ];
          };
          stdenv = commonArgsFor.${system}.stdenv;
        in {
          ${name} = (pkgs.mkShell.override {inherit stdenv;}) {
            packages =
              [toolchain]
              ++ (with pkgs; [
                cargo-audit
                cargo-license
                cargo-dist
              ]);
            shellHook = let
              extraLdPaths =
                pkgs.lib.makeLibraryPath (with pkgs; [
                  ]);
            in ''
              export LD_LIBRARY_PATH="${extraLdPaths}:$LD_LIBRARY_PATH"
            '';
            env = {
            };
          };
          default = self.devShells.${system}.${name};
        })
        nixpkgsFor;
    };
}
