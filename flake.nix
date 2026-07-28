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
      pins = import ./nix/pins.nix;
    in
    {
      packages = eachSystem (
        pkgs:
        let
          call = pkgs.lib.callPackageWith (pkgs // toolchain // { inherit pins; });
          toolchain = rec {
            rust-mos-stage0 = call ./nix/stage0.nix { };
            llvm-mos = call ./nix/llvm-mos.nix { };
            llvm-mos-sdk = call ./nix/llvm-mos-sdk.nix { };
            rust-mos-src = call ./nix/rust-mos-src.nix { };
            rust-mos = call ./nix/rust-mos.nix { };
            check-vendor = call ./nix/check-vendor.nix { };
          };
        in
        {
          inherit (toolchain)
            llvm-mos
            llvm-mos-sdk
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
          c64-prg = pkgs.callPackage ./nix/check-prg.nix {
            inherit (p) rust-mos rust-mos-src llvm-mos-sdk check-vendor;
          };
        }
      );

      devShells = eachSystem (
        pkgs:
        let
          p = self.packages.${pkgs.stdenv.hostPlatform.system};
        in
        {
          default = pkgs.mkShell {
            # rust-mos FIRST: its rustc/cargo (the forked cargo) must win.
            packages = [
              p.rust-mos
              p.llvm-mos-sdk
              p.llvm-mos
            ];

            RUST_TARGET_PATH = "${p.rust-mos}/targets";
            RUST_SRC_PATH = "${p.rust-mos}/lib/rustlib/src/rust/library";

            shellHook = ''
              # Known footgun: a stock rustc/cargo (rustup shims, homebrew,
              # another nix shell) shadowing the mos toolchain. Force our
              # paths to the front and verify.
              export PATH="${p.rust-mos}/bin:${p.llvm-mos-sdk}/bin:${p.llvm-mos}/bin:$PATH"
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
              echo "build:   cargo build --release --target mos-c64-none -Zbuild-std=core,alloc"
            '';
          };
        }
      );
    };
}
