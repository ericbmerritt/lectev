{
  inputs = {
    utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-21.05";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };
  outputs = { self, nixpkgs, utils, rust-overlay }:
    utils.lib.eachSystem [ "x86_64-linux" "x86_64-darwin" ] (system:
      let
        pre_pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlay ];
        };
        rust = pre_pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain;
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            rust-overlay.overlay
            (final: prev: {
              rustc = rust.override {
                extensions = [ "rust-src" "rls" "rust-analysis" "clippy" ];
              };
              act = prev.callPackage ./nix/act.nix { };
            })
          ];
        };
        lectev =
          (import ./Cargo.nix { inherit pkgs; }).workspaceMembers.lectev.build;
      in rec {
        overlay = final: prev: { lectev = lectev; };
        # `nix build`
        packages.lectev = lectev;
        defaultPackage = lectev;

        # `nix run`
        apps.lectev = utils.lib.mkApp {
          drv = packages.lectev;
          exePath = "/bin/lectev";
        };
        defaultApp = apps.lectev;

        # `nix develop`
        devShell = with pkgs;
          mkShell {
            nativeBuildInputs = [ pkg-config glibc openssl bashInteractive ];
            buildInputs = [
              nix-linter
              nixfmt
              vale
              entr
              ripgrep
              shfmt
              shellcheck
              act
              sphinx
              crate2nix
            ] ++ lib.optionals stdenv.isDarwin [ libiconv ];
            inputsFrom = [ lectev ];
          };
      });
}
