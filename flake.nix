{
  description = "Minimal Rust todo experiment";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

  outputs = { nixpkgs, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in {
      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          cargo
          rustc
          uv
          python312
        ];

        shellHook = ''
          export UV_TOOL_DIR="$PWD/.uv-tools"
          export UV_TOOL_BIN_DIR="$PWD/.uv-tools/bin"
          export PATH="$UV_TOOL_BIN_DIR:$PATH"
        '';
      };
    };
}
