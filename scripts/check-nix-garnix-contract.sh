#!/usr/bin/env bash
set -euo pipefail

fail() {
  printf 'ERROR: %s\n' "$*" >&2
  exit 1
}

require_file() {
  [[ -f "$1" ]] || fail "missing $1"
}

require_grep() {
  local pattern="$1"
  local file="$2"
  local note="$3"
  grep -Eq "$pattern" "$file" || fail "$note ($file)"
}

require_file flake.nix
require_file garnix.yaml
require_file rust-toolchain.toml

require_grep 'nixos-unstable' flake.nix 'flake must use nixos-unstable'
require_grep 'flake-utils' flake.nix 'flake must use flake-utils'
require_grep 'rust-overlay' flake.nix 'flake must use rust-overlay'
require_grep 'fromRustupToolchainFile \./rust-toolchain\.toml' flake.nix 'flake must read rust-toolchain.toml'
require_grep 'nodejs_22' flake.nix 'flake must pin Node 22'
require_grep 'mold' flake.nix 'flake must include mold linker'
require_grep 'sccache' flake.nix 'flake must include sccache'
require_grep 'devShells\.default' flake.nix 'flake must expose devShells.default'
require_grep 'toolchain-contract' flake.nix 'flake must expose toolchain-contract check'
require_grep 'garnix-contract' flake.nix 'flake must expose garnix-contract check'
require_grep 'formatter = pkgs\.nixpkgs-fmt' flake.nix 'flake must expose nixpkgs-fmt formatter'

require_grep 'channel[[:space:]]*=[[:space:]]*"1\.94\.1"' rust-toolchain.toml 'rust-toolchain.toml must pin Rust 1.94.1'
require_grep '"node"[[:space:]]*:[[:space:]]*">=22\.12\.0"' parkhub-web/package.json 'parkhub-web package must require Node >=22.12.0'

require_grep 'checks\.x86_64-linux\.\*' garnix.yaml 'Garnix must build Linux checks'
require_grep 'devShells\.x86_64-linux\.default' garnix.yaml 'Garnix must build the Linux dev shell'

if [[ ! -f flake.lock ]]; then
  printf 'WARN: flake.lock is not committed yet; run nix flake lock once nix is available.\n' >&2
fi

printf 'ParkHub Rust Nix/Garnix contract OK.\n'
