# rust-mos C64 toolchain — native Nix flake

Builds the March-2025 generation of the [rust-mos](https://github.com/mrk-its/rust-mos)
toolchain (rustc 1.87.0-dev with the LLVM-MOS 6502 backend) **from source**,
natively per platform. No containers, no patchelf'd foreign toolchain
binaries in the outputs.

Systems: `aarch64-darwin` (primary), `x86_64-linux`, `aarch64-linux`, and
`x86_64-darwin` (hashes present, untested).

## Provenance (verified 2026-07-24)

The community docker image `mrkits/rust-mos:13f2838f9-334fc98-8f3a80f8` is the
known-good artifact of this generation. Its tag decodes as
`<llvm-mos>-<llvm-mos-sdk>-<rust-mos>` — that ordering is taken from
`.github/workflows/build-rust-mos-image.yml` in the rust-mos repo itself, whose
`workflow_dispatch` defaults name these exact commits:

| component | repo | commit | note |
|---|---|---|---|
| llvm-mos | `mrk-its/llvm-mos` (fork) | `13f2838f909ca839ccb4d9a6c808de7b2ea60911` | branch `cmp_fix`; commit exists only in the fork, not upstream llvm-mos |
| llvm-mos-sdk | `llvm-mos/llvm-mos-sdk` (upstream) | `334fc9823745284938bfba543112f95bfb489b50` | v21.0.x era |
| rust-mos | `mrk-its/rust-mos` | `8f3a80f87cea2bd236d953027cb5bce9dfc63b89` | tip of `mos_rustc_1.87`; `src/version` = 1.87.0 |
| forked cargo | `mrk-its/cargo` | submodule `src/tools/cargo`, branch `compiler_builtins_patched` | injects mos `compiler_builtins` into `-Zbuild-std` |
| compiler_builtins | `mrk-its/compiler-builtins` | `4505c7df…` (branch `mos-0.1.148`) and `090b8f3c…` (branch `mos-0.1.150`) | see “design notes” |
| stage0 | static.rust-lang.org | beta 2025-02-18 (= 1.85.0-beta) | pinned by rust-mos' own `src/stage0`; sha256s copied from that file |

dwbrite/gametank-sdk's `tools/rust-mos-container/Containerfile` starts `FROM`
exactly this image, confirming it as the generation in community use.

## Prerequisites

You need **Rust** (to run `cargo xtask`), plus `git` and `curl`. Nix itself is installed
for you by the command below — don't install it separately.

- **macOS** (Apple Silicon): `xcode-select --install` (git + curl), then Rust via
  [rustup](https://rustup.rs). Intel Macs work too, but you must install Nix yourself with
  the official script (the command will print it) — no pinned installer binary is published
  for Intel macOS.
- **Linux** (Debian/Ubuntu): `sudo apt install -y git curl`, then Rust via rustup. `sudo`
  is required (Nix does a multi-user daemon install).
- **Windows**: Nix doesn't run natively — install
  [WSL2](https://learn.microsoft.com/windows/wsl/install) (`wsl --install`) and follow the
  Linux steps inside the WSL2 shell.

## Initialize the build environment

One command. It installs plain upstream Nix if it's missing (a pinned, checksum-verified
[NixOS/nix-installer](https://github.com/NixOS/nix-installer) binary — plain upstream Nix,
not a vendor distribution), generates `flake.lock`, fills the source hashes, and — by
default — runs the flake's full check: it builds the toolchain **and** compiles a C64
program with it offline to prove it actually works. Every step skips itself if already
done, so re-running just rebuilds.

```sh
git clone <repo> && cd rust-mos-flake     # or download + extract a tarball
cargo xtask init-buildenv                  # ~2-4 h on first run: builds + checks the whole flake
cargo xtask init-buildenv --build          # optional: build just the rust-mos toolchain, skip the checks
```

This is **deterministic**: `flake.nix` pins nixpkgs to an exact commit, and the committed
`flake.lock` plus the source hashes in `nix/pins.nix` freeze every input — so everyone who
runs it builds the same toolchain, regardless of machine or date.

### First time: commit the initialized repo

The first initialization generates `flake.lock` and fills the four `PREFETCH:` hashes in
`nix/pins.nix`. Commit the repo afterward so the next person to initialize finds them
already pinned and skips the slow steps. Add git *after* the build, not before — inside a
git tree Nix ignores untracked files, so committing afterward avoids that footgun:

```sh
cargo xtask init-buildenv                               # build + check + generate flake.lock + fill hashes
git init && git add -A && git commit -m "Pin flake.lock + source hashes"
```

`.gitignore` already excludes the build outputs (`result`, `target/`), so `git add -A`
stays clean. Under the hood the hash-filling is `cargo xtask prefetch-hashes` — the standard
[cargo-xtask](https://github.com/matklad/cargo-xtask) layout, a zero-dependency std-only
crate that drives `nix build`, harvests each `got: sha256-…` mismatch, substitutes it on the
matching `PREFETCH:` line, and re-verifies. It's idempotent (already-pinned entries are
skipped); Nix keys these fixed-output derivations by their declared hash, so each vendor FOD
runs twice (learn, then realize) — expect several GB of downloads, with the stage0 toolchain
(~400 MB) built first. `init-buildenv` calls it automatically; run it standalone only to
refresh hashes without a full build. The stage0 tarball hashes are already pinned (verbatim
from rust-mos' own `src/stage0`).

## Use

```sh
nix develop                 # rust-mos rustc + forked cargo + SDK on PATH
cd checks/c64-prg
cargo build --release --target mos-c64-none -Zbuild-std=core,alloc
# → target/mos-c64-none/release/c64-check  (a PRG: 2-byte $0801 load address
#   + BASIC SYS stub, both emitted by the SDK's C64 link step)
```

Run it: `x64sc target/mos-c64-none/release/c64-check` (VICE, e.g.
`nix shell nixpkgs#vice`). Expect a cycling border color.

`nix flake check` builds the whole chain plus `checks.<system>.c64-prg`, which
compiles that same crate **fully offline** inside the sandbox, asserts
`size_of::<core::ffi::c_uint>() == 2` at compile time, and verifies the output
begins with `01 08`.

The dev shell exports `RUST_TARGET_PATH` (the generated `mos-*-none.json`
specs) and `RUST_SRC_PATH`, prepends the toolchain to `PATH`, and **warns
loudly if a stock rustc/cargo shadows the mos one** — with a stock cargo,
`-Zbuild-std` would resolve the wrong `compiler_builtins`; that footgun is
checked on every shell entry.

First build is LLVM-scale: on an M3 Max expect very roughly ¾–1½ h for
llvm-mos and 1–2 h for rust-mos (estimates, not measurements). That's what the
cache is for:

## Cachix

```sh
# once, as the cache owner:
#   1. create a cache at https://app.cachix.org (free for public caches);
#      note the cache name and its public key
#   2. cachix authtoken <token-from-the-web-ui>
nix build .#llvm-mos .#llvm-mos-sdk .#rust-mos --print-out-paths | cachix push YOUR-CACHE

# then edit flake.nix: uncomment nixConfig and fill in
#   https://YOUR-CACHE.cachix.org  +  the cache's public key
```

Consumers then get offered the cache automatically on first `nix build`/
`nix develop` (or add it manually with `cachix use YOUR-CACHE`). Push from
each platform you've built on; Cachix stores them side by side.

## Design notes

**Stage0.** rust-mos' `src/stage0` pins beta-2025-02-18 — that is
**1.85.0-beta** (1.85.0 released 2025-02-20), which is why this flake fetches
those exact tarballs as FODs instead of borrowing a newer nixpkgs rustc:
feeding a 1.86+ stage0 into a snapshot whose `cfg(bootstrap)` gates expect
1.85 is a plausible-breakage risk the upstream CI never tested. The sha256s
are upstream's own, copied from `src/stage0`. Stage0 binaries are
autoPatchelf'd **build-time only** inputs; nothing from them lands in the
installed toolchain.

**Vendoring (offline builds).** Git checkouts of rustc have no `vendor/`.
`nix/rust-mos-src.nix` is the single network-enabled FOD: it checks out
rust-mos + the three needed submodules (`src/tools/cargo`, `library/stdarch`,
`library/backtrace`; `src/llvm-project` is stubbed the way the upstream docker
build stubs it), runs `cargo vendor --sync` over the root, `library/`,
`src/tools/cargo/` and `src/bootstrap/` workspaces, and captures the emitted
source-replacement config verbatim — it contains a git-source stanza (the
`compiler_builtins` patch) that the classic hardcoded two-stanza config would
miss.

**The compiler_builtins 148/150 duality (verified).** `library/Cargo.toml`
patches `compiler_builtins` to git branch `mos-0.1.148`; the forked cargo's
`standard_lib.rs` *additionally* injects branch `mos-0.1.150` into every
`-Zbuild-std` resolution. Both are vendored for the offline check. For
reproducibility the 148 *branch* reference is rewritten to its verified commit
during source prep; the 150 reference lives only inside the forked cargo
binary and is vendored under its branch-form source id (`nix/check-vendor.nix`
— if mrk-its ever pushes to that branch again, the FOD fails loudly with a
hash mismatch rather than drifting).

**The c_uint gate.** llvm-mos C `int` is 16-bit and the mos target spec says
`c_int_width = "16"`, but this generation ships `core::ffi` with 32-bit
`c_int`/`c_uint` — verified missing at `8f3a80f8`, present in the 1.78-era
fork, i.e. the patch was lost in the rebase. `nix/rust-mos.nix` re-applies it
to `library/core/src/ffi/primitives.rs` with count guards (refuses to build if
the file's shape changed), and the check crate const-asserts the result.

**Target JSONs.** `mos-unknown-none` is a **built-in** target in this
generation (`compiler/rustc_target/src/spec/targets/mos_unknown_none.rs`,
with `requires_lto = true`, `linker = mos-clang`). The per-machine specs
(`mos-c64-none` etc.) are JSON overrides — linker `mos-c64-clang`, vendor
`c64` — generated at install time by `nix/mos-targets.py`, a faithful replica
of upstream's `create_mos_targets.py` (present on the `mos_target_latest`
branch; referenced by this generation's own `build.sh` and Dockerfile).
`RUST_TARGET_PATH` points at them.

**LTO.** Kept required, per the target spec and because every known-good 1.87
build had it on; without LTO older generations died in `core` on u128 codegen
(`LLVM ERROR: unable to legalize … s128`). Both check-crate profiles set
`lto = true`.

**Deviations from the upstream docker recipe** (all deliberate, all in the
direction of reproducibility): git *branch* refs pinned to commits;
`LLVM_INSTALL_TOOLCHAIN_ONLY=OFF` + `LLVM_BUILD_LLVM_DYLIB/LLVM_LINK_LLVM_DYLIB=ON`
+ host codegen targets re-enabled on top of `clang/cmake/caches/MOS.cmake`
(same deltas mlund's known-good no-docker recipe applies — rustc needs
`llvm-config`, headers and a shared libLLVM); `--disable-docs`; nixpkgs clang
stdenv on Darwin rather than Xcode.

## First-run expectations (labeled honestly)

Verified facts above are from the repos themselves; the following are the
**known unknowns** — things that can only be proven by the first real build,
listed in descending order of expected friction:

1. **Offline `-Zbuild-std` resolution in the sandboxed check.** The forked
   cargo's injection mechanism is nonstandard; whether its resolver is fully
   satisfied by the two vendored source replacements is inference from cargo
   semantics, not yet demonstrated. If `checks.c64-prg` fails offline while
   the same command works in `nix develop` (online), the fix belongs in
   `nix/check-prg.nix`'s config assembly — the error will name the source id
   it wanted.
2. **`library/Cargo.lock` sync.** The committed lock pins crates.io 0.1.148
   while the manifest patches to git (verified mismatch); `rust-mos-src` runs
   `cargo fetch` to reconcile. If x.py still complains about a stale lock,
   vendoring needs a `cargo update -p compiler_builtins` instead.
3. **LLVM version handshake.** rustc 1.87 accepts LLVM 18–20; llvm-mos of
   Mar-2025 should identify as 20.x (its rust submodule pointer is
   `rustc/20.1-2025-02-13`), but the exact version string llvm-config reports
   is unverified.
4. **SDK version stamping without `.git`** (fetchFromGitHub trees have none);
   CMake projects usually fall back gracefully — unverified for this SDK rev.
5. **x86_64-darwin** is wired up but completely untested.

Everything in `nix/pins.nix` marked PREFETCH fails loudly (hash mismatch or
guard `exit 1`), never silently.

## Acceptance mapping

- `packages.<system>.{llvm-mos, llvm-mos-sdk, rust-mos, default}` — in
  `flake.nix` (plus plumbing attrs: `stage0`, `rust-mos-src`, `check-vendor`,
  `*-source`).
- devShell with mos rustc/cargo as default + shadowing guard +
  `RUST_TARGET_PATH` + rust-src — `devShells.<system>.default`.
- offline PRG check with the `c_uint` compile-time assert —
  `checks.<system>.c64-prg`.
- Cachix — above.
