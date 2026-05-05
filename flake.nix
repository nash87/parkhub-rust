{
  description = "ParkHub Rust - Tauri/headless server plus React 19 toolchain";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        nodejs = pkgs.nodejs_22;
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            rustToolchain
            rust-analyzer
            nodejs
            pkg-config
            cmake
            clang
            mold
            sccache
            openssl
            sqlite
            postgresql
            curl
            jq
            git
            zlib
            fontconfig
            freetype
            wayland
            wayland-protocols
            libxkbcommon
            libGL
            xorg.libX11
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi
          ];

          env = {
            CI = "true";
            RUSTC_WRAPPER = "sccache";
            RUSTFLAGS = "-C link-arg=-fuse-ld=mold";
          };

          shellHook = ''
            echo "ParkHub Rust dev shell: $(rustc --version), Node $(node --version)"
          '';
        };

        checks = {
          toolchain-contract = pkgs.runCommand "parkhub-rust-toolchain-contract"
            {
              nativeBuildInputs = [
                rustToolchain
                nodejs
                pkgs.jq
                pkgs.gnugrep
              ];
            }
            ''
              rustc --version | grep -q '1.94.1'
              cargo --version >/dev/null
              node --version | grep -Eq '^v22\.'
              npm --version >/dev/null
              grep -q 'channel = "1.94.1"' ${./rust-toolchain.toml}
              jq -e '.engines.node == ">=22.12.0"' ${./parkhub-web/package.json} >/dev/null
              touch "$out"
            '';

          garnix-contract = pkgs.runCommand "parkhub-rust-garnix-contract"
            {
              nativeBuildInputs = [ pkgs.gnugrep ];
            }
            ''
              test -f ${./garnix.yaml}
              grep -q 'checks.x86_64-linux.*' ${./garnix.yaml}
              grep -q 'devShells.x86_64-linux.default' ${./garnix.yaml}
              touch "$out"
            '';
        };

        formatter = pkgs.nixpkgs-fmt;
      });
}
