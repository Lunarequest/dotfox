{
  description = "A very basic for development and packaging of dotfox";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
    naersk,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      overlays = [(import rust-overlay)];
      pkgs = import nixpkgs {
        inherit system overlays;
      };
    in
      with pkgs; {
        formatter = pkgs.alejandra;
        devShells.default = mkShell {
          buildInputs = with pkgs; [
            pkgs.zsh
            pkgs.nil
            rust-bin.stable.latest.default
            pkgs.rust-analyzer
            pkgs.nil
          ];
          shellHook = ''
            test ~/.zshrc && exec zsh
          '';
        };
        packages = rec {
          dotfox = naersk.lib.${system}.buildPackage {
            pname = "dotfox";
            root = ./.;
            nativeBuildInputs = with pkgs; [llvmPackages_18.libcxxClang mold libgit2 libgpg-error gpgme openssl pkg-config];
          };

          default = dotfox;
        };

        apps = {
          dotfox = {
            program = self.packages.dotfox;
          };
          defaultApp = dotfox;
        };
      });
}
