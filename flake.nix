{
  description = "rust-mos C64 toolchain, built from source: llvm-mos (6502 backend) + llvm-mos-sdk + rustc 1.87.0-dev fork, native per platform";

  # After creating your Cachix cache, uncomment and fill these in so
  # downstream users are offered your cache automatically (see README):
  # nixConfig = {
  #   extra-substituters = [ "https://YOUR-CACHE.cachix.org" ];
  #   extra-trusted-public-keys = [ "YOUR-CACHE.cachix.org-1:PASTE-PUBLIC-KEY-HERE" ];
  # };

  inputs = {
    # Pinned to an exact commit (tip of nixos-25.05 at 2026-07-28), not the moving
    # branch, so the build is identical even if flake.lock is ever lost or regenerated.
    # To update deliberately: bump this rev, then `nix flake lock`.
    nixpkgs.url = "github:NixOS/nixpkgs/ac62194c3917d5f474c1a844b6fd6da2db95077d";
  };

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "aarch64-darwin" # primary
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin" # untested, hashes present
      ];
      eachSystem = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
      pins = import ./toolchain/pins.nix;
    in
    {
      packages = eachSystem (
        pkgs:
        let
          call = pkgs.lib.callPackageWith (pkgs // toolchain // { inherit pins; });
          toolchain = rec {
            rust-mos-stage0 = call ./toolchain/stage0.nix { };
            llvm-mos = call ./toolchain/llvm-mos.nix { };
            llvm-mos-sdk = call ./toolchain/llvm-mos-sdk.nix { };
            mos-toolchain = call ./toolchain/mos-toolchain.nix { };
            rust-mos-src = call ./toolchain/rust-mos-src.nix { };
            rust-mos = call ./toolchain/rust-mos.nix { };
            check-vendor = call ./toolchain/check-vendor.nix { };
          };
        in
        {
          inherit (toolchain)
            llvm-mos
            llvm-mos-sdk
            mos-toolchain
            rust-mos
            ;
          default = toolchain.rust-mos;

          # Internal/plumbing attrs, exposed for prefetch-hashes.sh and debugging:
          stage0 = toolchain.rust-mos-stage0;
          rust-mos-src = toolchain.rust-mos-src;
          check-vendor = toolchain.check-vendor;
          llvm-mos-source = toolchain.llvm-mos.src;
          llvm-mos-sdk-source = toolchain.llvm-mos-sdk.src;
        }
      );

      checks = eachSystem (
        pkgs:
        let
          p = self.packages.${pkgs.stdenv.hostPlatform.system};
        in
        {
          c64-hello-world = pkgs.callPackage ./c64/check.nix {
            inherit (p) rust-mos rust-mos-src mos-toolchain check-vendor;
          };
        }
      );

      devShells = eachSystem (
        pkgs:
        let
          p = self.packages.${pkgs.stdenv.hostPlatform.system};

          # The repo's xtask binary, built with stock nixpkgs Rust (it is
          # dependency-free, std-only — nothing to vendor). Used only to back the
          # `cargo-xasm` shim below; the host `cargo xtask` alias is unaffected.
          xtask = pkgs.rustPlatform.buildRustPackage {
            pname = "xtask";
            version = "0.1.0";
            src = ./xtask;
            cargoLock.lockFile = ./xtask/Cargo.lock;
          };

          # `cargo <name>` runs any `cargo-<name>` on PATH, so this makes
          # `cargo xasm [FUNCTION]` work from any crate directory in the shell —
          # the run-from-the-crate ergonomics a cargo alias can't give (an alias
          # resolves `--manifest-path` against the cwd, so it only works from the
          # repo root). cargo may pass the subcommand name through as the first
          # arg; drop a leading "xasm", then defer to xtask's asm subcommand.
          cargo-xasm = pkgs.writeShellScriptBin "cargo-xasm" ''
            [ "''${1:-}" = "xasm" ] && shift
            exec ${xtask}/bin/xtask asm "$@"
          '';
        in
        {
          default = pkgs.mkShell {
            # rust-mos FIRST: its rustc/cargo (the forked cargo) must win.
            # cargo-xasm is a distinct binary, so it never shadows cargo.
            packages = [
              p.rust-mos
              p.mos-toolchain
              cargo-xasm
            ];

            RUST_TARGET_PATH = "${p.rust-mos}/targets";
            RUST_SRC_PATH = "${p.rust-mos}/lib/rustlib/src/rust/library";

            shellHook = ''
              # Known footgun: a stock rustc/cargo (rustup shims, homebrew,
              # another nix shell) shadowing the mos toolchain. Force our
              # paths to the front and verify.
              export PATH="${p.rust-mos}/bin:${p.mos-toolchain}/bin:$PATH"
              # rustup's shims resolve via these; make sure they can't hijack
              # `cargo build` inside this shell.
              unset RUSTUP_TOOLCHAIN RUSTUP_HOME CARGO 2>/dev/null || true

              if ! rustc --print target-list 2>/dev/null | grep -qx 'mos-unknown-none'; then
                echo "WARNING: the rustc on PATH is NOT rust-mos (no mos-unknown-none target)." >&2
                echo "         Something is shadowing ${p.rust-mos}/bin." >&2
              fi
              case "$(command -v cargo)" in
                ${p.rust-mos}/bin/cargo) ;;
                *)
                  echo "WARNING: cargo resolves to $(command -v cargo), not the forked rust-mos cargo." >&2
                  echo "         -Zbuild-std would silently use the wrong compiler_builtins." >&2
                  ;;
              esac

              echo "rust-mos $(rustc --version 2>/dev/null) | targets: $RUST_TARGET_PATH"
              echo "build:   cd into a platform crate (e.g. c64/examples/hello-world) and run 'cargo build'"
            '';
          };
        }
      );
    };
}
