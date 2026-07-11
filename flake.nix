{
  description = "Minimal Rust todo experiment";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

  outputs = { nixpkgs, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };

      todo-tui = pkgs.rustPlatform.buildRustPackage {
        pname = "todo-tui";
        version = "0.8.2";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;
        meta.mainProgram = "todo-tui";
      };
    in {
      packages.${system}.default = todo-tui;

      apps.${system}.default = {
        type = "app";
        program = "${todo-tui}/bin/todo-tui";
      };

      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [ cargo rustc ];
      };
    };
}
