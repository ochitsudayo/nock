{
    description = "nock, a full terminal clock built in rust";

    inputs = {
        nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
        flake-utils.url = "github:numtide/flake-utils";
    };

    outputs = {
        self,
        nixpkgs,
        flake-utils,
    }:

    flake-utils.lib.eachDefaultSystem (
        system:
        let
            pkgs = nixpkgs.legacyPackages.${system};
        in
        {
            packages.default = pkgs.rustPlatform.buildRustPackage {
                pname = "nock";
                version = "1.0";

                src = self;

                cargoLock.lockFile = ./Cargo.lock;

                meta = with pkgs.lib; {
                    description = "a full terminal clock built in rust";
                    mainProgram = "nock";
                    platforms = platforms.linux ++ platforms.darwin;
                };
            };

            apps.default = flake-utils.lib.mkApp {
                drv = self.packages.${system}.default;
            };

            devShells.default = pkgs.mkShell {
                inputsFrom = [ self.packages.${system}.default ];
                packages = with pkgs; [
                    cargo
                    rustc
                    rust-analyzer
                    clippy
                    rustfmt
                    gdb
                ];
            };
        }
    );
}
