{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils, naersk }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        naersk-lib = pkgs.callPackage naersk { };
      in
      {
        defaultPackage = naersk-lib.buildPackage ./.;
        devShell = with pkgs; mkShell {
          buildInputs = [
            cargo
            rustc
            rustfmt
            pre-commit
            rustPackages.clippy
            clang
            llvmPackages_16.libclang.lib
            pkg-config
            alsa-lib
          ];

          shellHook = ''
            export LIBCLANG_PATH="${pkgs.llvmPackages_16.libclang.lib}/lib"
            export LD_LIBRARY_PATH="${pkgs.alsa-lib}/lib:$LD_LIBRARY_PATH"
          '';
        };
      }
    );
}
