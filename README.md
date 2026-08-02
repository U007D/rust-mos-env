# rust-mos C64 toolchain (Nix flake)

Builds the [rust-mos](https://github.com/mrk-its/rust-mos) toolchain — rustc 1.87.0-dev
with the LLVM-MOS 6502 backend — from source with Nix. No containers.

Built and tested on `aarch64-darwin` (Apple Silicon). Also configured for `x86_64-linux`,
`aarch64-linux`, and `x86_64-darwin`, but those have not been built here yet.

## Pinned versions

The toolchain is assembled from several projects that must be used in matching versions: an
LLVM fork with the 6502 backend, the platform SDK, a rustc fork, a patched cargo, and a
patched `compiler_builtins`. This flake pins the exact commit of each.

| component | repository | commit | notes |
|---|---|---|---|
| llvm-mos | `mrk-its/llvm-mos` (fork) | `13f2838f909ca839ccb4d9a6c808de7b2ea60911` | branch `cmp_fix`; exists only in the fork, not in upstream llvm-mos |
| llvm-mos-sdk | `llvm-mos/llvm-mos-sdk` | `334fc9823745284938bfba543112f95bfb489b50` | v21.0.x |
| rust-mos | `mrk-its/rust-mos` | `8f3a80f87cea2bd236d953027cb5bce9dfc63b89` | tip of `mos_rustc_1.87`; `src/version` = 1.87.0 |
| cargo (fork) | `mrk-its/cargo` | submodule `src/tools/cargo`, branch `compiler_builtins_patched` | injects the mos `compiler_builtins` into `-Zbuild-std` |
| compiler_builtins | `mrk-its/compiler-builtins` | `4505c7df…` (branch `mos-0.1.148`), `090b8f3c…` (branch `mos-0.1.150`) | both are needed — see Design notes |
| stage0 | static.rust-lang.org | beta 2025-02-18 (= 1.85.0-beta) | pinned by rust-mos' `src/stage0`; sha256s copied from that file |

Using the latest commit of each project is not likely to work: they are only known to work as this
specific set. It is the combination the upstream rust-mos project builds and tests together
— its CI workflow, `.github/workflows/build-rust-mos-image.yml`, pins exactly these commits.
Pinning them here also makes the build reproducible.

The upstream project also publishes this set as a container image,
`mrkits/rust-mos:13f2838f9-334fc98-8f3a80f8`; the tag is just the three short commit hashes
(llvm-mos, llvm-mos-sdk, rust-mos). This flake builds those same commits directly, without a
container.

## Prerequisites

You need `git`, `curl`, and a stable Rust (any current stable — if you can run
`cargo` you already have what's needed; otherwise install via [rustup](https://rustup.rs)).
Your host cargo only compiles the small `xtask` helper and runs Nix; the rust-mos compiler
itself is built by the flake. You do not install Nix yourself — `cargo xtask init-buildenv`
installs it for you.

- **macOS**: Install the [Homebrew](https://brew.sh) package manager and use it to install `git` (`brew install git`); `curl` is already present. Install Rust with `rustup`. On Apple Silicon, Nix is installed automatically by the command below. On Intel Macs, [install Nix package manager](https://nixos.org/download/) manually first — there is no pinned installer binary for Intel macOS, so
  `init-buildenv` prints the official install command instead of installing it.
- **Linux** (Debian/Ubuntu): `sudo apt install -y git curl`, then Rust via `rustup`. `sudo`
  is required — Nix installs a multi-user daemon.
- **Windows**: Nix does not run natively. Install
  [WSL2](https://learn.microsoft.com/windows/wsl/install) (`wsl --install`) and follow the
  Linux steps inside it.

## Build

```sh
git clone https://github.com/u007d/rust-mos-flake && cd rust-mos-flake
# first run: ~2–4 h+ (builds and checks the front-end and back-end compilers and the SDK from source)
cargo xtask init-buildenv          
```

`init-buildenv` installs Nix if it is missing (a pinned, checksum-verified
[NixOS/nix-installer](https://github.com/NixOS/nix-installer) binary — upstream Nix),
generates `flake.lock`, fills the source hashes in `nix/pins.nix`, then runs
`nix flake check`, which builds the toolchain and compiles a C64 test program with it
offline. Each step is skipped if it is already done.

The build is reproducible: `flake.nix` pins nixpkgs to an exact commit, and the committed
`flake.lock` and source hashes fix every input, so the result does not depend on the machine
or the date.

### First run: commit the generated files

The first run generates `flake.lock` and fills the four `PREFETCH:` hashes in
`nix/pins.nix`. Commit them so later runs, and other people, skip regenerating them:

```sh
cargo xtask init-buildenv
git init && git add -A && git commit -m "Pin flake.lock and source hashes"
```

Add git after the build, not before: in a git tree Nix ignores untracked files, so
committing afterward avoids having to `git add` before the build can see the files.
`.gitignore` already excludes build outputs (`result`, `target/`).

The hash-filling is done by `cargo xtask prefetch-hashes`, which `init-buildenv` runs
automatically; run it directly only to refresh hashes without a full build. It runs
`nix build`, reads each `got: sha256-…` mismatch, writes the hash onto the matching
`PREFETCH:` line, and rebuilds to verify. Nix identifies fixed-output derivations by their
declared hash, so each source derivation is fetched twice — once to learn the hash, once to
use it — which is several GB of downloads. The stage0 tarball hashes are pinned already
(from rust-mos' `src/stage0`).

## Building C64 programs

A worked example is included at `example/rainbow-border` (it cycles the C64 border color).
`nix develop` starts a shell with the toolchain on `PATH` — rust-mos `rustc`, its `cargo`,
and the SDK; build inside it with cargo, then leave the shell with `exit`:

```sh
nix develop
cd example/rainbow-border
cargo build --release --target mos-c64-none -Zbuild-std=core,alloc
# -> target/mos-c64-none/release/rainbow-border
#    a .prg file: 2-byte $0801 load address + BASIC SYS stub, from the SDK's C64 linker
```

Run it in the VICE emulator (`nix shell nixpkgs#vice`):
`x64sc target/mos-c64-none/release/rainbow-border`.

To build your own program, do the same in your own crate — `nix develop`, then
`cargo build --release --target mos-c64-none -Zbuild-std=core,alloc` (one-shot, no
interactive shell: `nix develop -c cargo build …`).

### Using `alloc`

`-Zbuild-std=core,alloc` compiles the `alloc` crate, but its heap types (`Box`, `Vec`,
`String`) also need a global allocator. `rainbow-border` never allocates, so none is linked;
to allocate, wire one to the SDK's `malloc`:

```rust
extern crate alloc;
use core::alloc::{GlobalAlloc, Layout};
extern "C" {
    fn malloc(n: usize) -> *mut u8;
    fn free(p: *mut u8);
}
struct SdkMalloc;
unsafe impl GlobalAlloc for SdkMalloc {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 { malloc(l.size()) }
    unsafe fn dealloc(&self, p: *mut u8, _l: Layout) { free(p) }
}
#[global_allocator]
static ALLOC: SdkMalloc = SdkMalloc;
```

Budget for the C64 target: it links into `0xC7FF` bytes of RAM (≈50 KB, `$0801`–`$CFFF`),
with the stack at `$D000` growing down. A program that actually allocates is about 1414 bytes (a
minimal one measured at 1545 bytes, vs 131 bytes with no allocator), which leaves on the
order of ~51 KB for the heap + stack (the heap shares that space with the downward-growing stack and
shrinks as your code and static data grow, so keep allocations modest).

The dev shell sets `RUST_TARGET_PATH` (the generated `mos-*-none.json` target specs) and
`RUST_SRC_PATH`, puts the toolchain first on `PATH`, and warns if another rustc/cargo
shadows the mos one — a stock cargo would resolve the wrong `compiler_builtins` under
`-Zbuild-std`.

`nix flake check` builds the whole toolchain and runs `checks.<system>.rainbow-border`, which
compiles `example/rainbow-border` offline, asserts `size_of::<core::ffi::c_uint>() == 2` at
compile time, and checks the output begins with `01 08`.

The first build is large — roughly ¾–1½ h for llvm-mos and 1–2 h for rust-mos on an M3 Max
(estimates). A binary cache avoids rebuilding.

## Binary cache (Cachix)

Push the build to a binary cache so other machines and CI download the toolchain instead of
rebuilding it. Create a cache at [app.cachix.org](https://app.cachix.org) (free for public
caches), note its name and public key, and export a push token:

```sh
export CACHIX_AUTH_TOKEN=<token-from-the-web-ui>
cargo xtask publish-cache <cache-name> --public-key <cache-name>.cachix.org-1:<key>
```

`publish-cache` builds the outputs and pushes them; it fetches `cachix` via `nix run`, so
there is nothing to install. With `--public-key` it also fills the `nixConfig` block in
`flake.nix`, so anyone who builds is offered the cache automatically — commit that change.
Omit `--public-key` to push without editing `flake.nix`.

By hand:

```sh
nix build .#llvm-mos .#llvm-mos-sdk .#rust-mos --print-out-paths | cachix push <cache-name>
# then fill nixConfig in flake.nix with https://<cache-name>.cachix.org and the key
```

Push from each platform you build on; Cachix stores them side by side.

## Design notes

**stage0.** rust-mos' `src/stage0` pins beta 2025-02-18, which is 1.85.0-beta (1.85.0 was
released 2025-02-20). The flake fetches those exact tarballs as fixed-output derivations
rather than reusing a newer nixpkgs rustc: feeding a 1.86+ stage0 into a source tree whose
`cfg(bootstrap)` gates expect 1.85 is untested upstream. The sha256s come from `src/stage0`.
stage0 also includes the nightly `rustfmt` bootstrap expects (passed via `build.rustfmt`); it
is a build-time-only input and nothing from it is installed.

**Offline vendoring.** A git checkout of rustc has no `vendor/`. `nix/rust-mos-src.nix` is
the only network-enabled derivation: it checks out rust-mos and the needed submodules
(`src/tools/cargo`, `library/stdarch`, `library/backtrace`; `src/llvm-project` is stubbed),
runs `cargo vendor --sync` over the root, `library/`, `src/tools/cargo/`, and
`src/bootstrap/` workspaces, and records the resulting source-replacement config. Everything
after that runs offline. `nix/rust-mos.nix` writes a `git-commit-info` file so bootstrap
treats the tree as a source tarball and does not require submodules that were not vendored.

**compiler_builtins 148 and 150.** `library/Cargo.toml` patches `compiler_builtins` to
branch `mos-0.1.148`; the forked cargo additionally injects branch `mos-0.1.150` into every
`-Zbuild-std` resolution. Both are vendored. The 148 branch reference is rewritten to a
fixed commit during source prep; the 150 reference is vendored under its branch id
(`nix/check-vendor.nix`), so a new push to that branch fails with a hash mismatch instead of
silently changing the build.

**c_uint.** llvm-mos' C `int` is 16-bit and the mos target sets `c_int_width = "16"`, but at
commit `8f3a80f8` `core::ffi` declares `c_int`/`c_uint` as 32-bit (the 1.78-era fix was lost
in a rebase). `nix/rust-mos.nix` re-applies it to `library/core/src/ffi/primitives.rs` (with
guards that fail the build if the file changed shape) and adds `mos` to core's `check-cfg`;
the check crate asserts the result at compile time.

**Target specs.** `mos-unknown-none` is a built-in target (`requires_lto = true`,
`linker = mos-clang`). The per-machine specs (`mos-c64-none`, etc.) are JSON overrides —
linker `mos-c64-clang`, vendor `c64` — generated at install time by `nix/mos-targets.py` (a
copy of upstream's `create_mos_targets.py`). `RUST_TARGET_PATH` points at them.

**LTO.** Left on, per the target spec; without it, `core` fails on u128 codegen
(`LLVM ERROR: unable to legalize … s128`). Both check-crate profiles set `lto = true`.

**llvm-mos-sdk.** Built against the Nix-built llvm-mos (`-DLLVM_MOS=…`, which skips its
prebuilt-compiler download). Its `libclang_rt.builtins.a` is emitted with the archive symbol
index stored as a regular member, which breaks the SDK's `llvm-ar qL` library merge;
`nix/llvm-mos-sdk.nix` normalizes the archive in a symlink overlay so the merge succeeds.

**Combined toolchain.** The SDK ships per-platform driver wrappers (`mos-c64-clang`, …) as
symlinks to `mos-clang` that rely on clang auto-loading the platform config from its install
directory. Because llvm-mos and the SDK are separate store paths, that lookup fails.
`nix/mos-toolchain.nix` provides wrapper scripts that call `mos-clang --config <platform>.cfg`
explicitly; the dev shell and the check use it.

**Shared libLLVM.** llvm-mos is built as a shared library (`--enable-llvm-link-shared`), so
rustc loads `libLLVM.dylib` at run time. `nix/rust-mos.nix` puts it on the run path: at build
time via `DYLD_FALLBACK_LIBRARY_PATH`, and in the installed toolchain via a symlink into the
rustc lib directory.

## Status

Built successfully on `aarch64-darwin`: `nix flake check` passes and the offline C64 check
produces a valid `.prg`. `x86_64-linux`, `aarch64-linux`, and `x86_64-darwin` are configured
but have not been built here; `x86_64-darwin` in particular has no pinned Nix installer
binary and is untested.

## Outputs

- `packages.<system>.{llvm-mos, llvm-mos-sdk, mos-toolchain, rust-mos, default}` — plus
  plumbing attributes `stage0`, `rust-mos-src`, `check-vendor`, and `*-source`
- `devShells.<system>.default` — the mos toolchain, with the shadowing check
- `checks.<system>.rainbow-border` — the offline PRG check with the `c_uint` compile-time assert
