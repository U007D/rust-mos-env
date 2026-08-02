# rust-mos-env — 6502 Rust toolchain (Nix flake)

Builds the [rust-mos](https://github.com/mrk-its/rust-mos) toolchain — rustc 1.87.0-dev with the
LLVM-MOS 6502 backend — from source with Nix, and uses it to build programs for 6502 platforms.
No containers.

Built and tested on `aarch64-darwin` (Apple Silicon). Also configured for `x86_64-linux`,
`aarch64-linux`, and `x86_64-darwin`, but those have not been built here yet.

## How to

### 1. Install prerequisites

You need `git`, `curl`, and any current stable Rust (install with [rustup](https://rustup.rs)).
Your host Rust only builds the small `xtask` helper and runs Nix; the rust-mos compiler itself is
built by the flake. You do not install Nix yourself — `cargo xtask initenv` does that.

- **macOS**: install [Homebrew](https://brew.sh), then `brew install git`. For `cargo run`, also
  `brew install vice` (nixpkgs has no macOS VICE). On Apple Silicon, `initenv` installs Nix; on
  Intel it prints the install command for you to run first.
- **Linux** (Debian/Ubuntu): `sudo apt install -y git curl`. `sudo` is needed — Nix installs a
  multi-user daemon.
- **Windows**: install [WSL2](https://learn.microsoft.com/windows/wsl/install) and follow the
  Linux steps inside it.

### 2. Build the toolchain

```sh
git clone https://github.com/u007d/rust-mos-env && cd rust-mos-env
cargo xtask initenv
```

`initenv` installs Nix if it is missing, generates `flake.lock`, fills in the source hashes, then
builds the toolchain and compiles a C64 test program to check it. The first run takes roughly
2–4 h if there are no cached binaries to download; later runs skip steps already done. Commit the
generated files afterward — see [DETAILS.md](DETAILS.md#first-run-commit-the-generated-files).

### 3. Build a C64 program

An example is at `c64/examples/hello-world`. `nix develop` starts a shell with the toolchain on
`PATH`.

```sh
nix develop
cd c64/examples/hello-world
cargo build --release
# -> target/mos-c64-none/release/hello-world  (a .prg file)
```

### 4. Run it

```sh
cargo run --release
```

This launches the program in the VICE emulator (`x64sc`). On Linux the dev shell provides VICE;
on macOS install it with `brew install vice`.

## Understanding the crate

The toolchain is shared across all 6502 platforms. Platform-specific programs and their checks
live in their own top-level folder — today that is `c64/` (Commodore 64) — with room to add
others (`a800xl/`, `nes/`, …) beside it without changing the toolchain build.

### Layout

```
rust-mos-env/
├── flake.nix        # toolchain packages + per-platform checks + dev shell
├── flake.lock       # nixpkgs pinned to an exact commit
├── xtask/           # repo tasks: initenv, prefetch-hashes, publish-cache, asm
├── toolchain/       # shared 6502 toolchain build (all platforms)
│   ├── rust-mos.nix       # rustc/cargo fork build
│   ├── llvm-mos.nix       # LLVM with the 6502 backend
│   ├── llvm-mos-sdk.nix   # platform SDK (runtime + linker config)
│   ├── mos-toolchain.nix  # mos-clang wrapper scripts
│   └── pins.nix           # pinned commits + source hashes
└── c64/             # Commodore 64 platform
    ├── check.nix          # offline PRG build check
    └── examples/hello-world/
```

### Building inside the dev shell

`nix develop` puts the rust-mos `rustc`, its `cargo`, and the SDK on `PATH`. The example ships a
`.cargo/config.toml` that targets the C64 by default:

```toml
[build]
target = "mos-c64-none"

[unstable]
build-std = ["core", "alloc"]
```

so plain `cargo build` and `cargo check` work inside the shell, and rust-analyzer checks against
`mos-c64-none` too. These settings only work **inside** `nix develop`, which provides the forked
cargo, the mos target, and `RUST_TARGET_PATH`. Outside the shell, a plain `cargo build` targets
your host and fails. To build your own program, copy that `.cargo/config.toml` into your crate.

### Pinned versions

The toolchain is assembled from several projects that must be used in matching versions: an LLVM
fork with the 6502 backend, the platform SDK, a rustc fork, a patched cargo, and a patched
`compiler_builtins`. This flake pins the exact commit of each.

| component | repository | commit | notes |
|---|---|---|---|
| llvm-mos | `mrk-its/llvm-mos` (fork) | `13f2838f909ca839ccb4d9a6c808de7b2ea60911` | branch `cmp_fix`; exists only in the fork, not in upstream llvm-mos |
| llvm-mos-sdk | `llvm-mos/llvm-mos-sdk` | `334fc9823745284938bfba543112f95bfb489b50` | v21.0.x |
| rust-mos | `mrk-its/rust-mos` | `8f3a80f87cea2bd236d953027cb5bce9dfc63b89` | tip of `mos_rustc_1.87`; `src/version` = 1.87.0 |
| cargo (fork) | `mrk-its/cargo` | submodule `src/tools/cargo`, branch `compiler_builtins_patched` | injects the mos `compiler_builtins` into `-Zbuild-std` |
| compiler_builtins | `mrk-its/compiler-builtins` | `4505c7df…` (branch `mos-0.1.148`), `090b8f3c…` (branch `mos-0.1.150`) | both are needed — see [DETAILS.md](DETAILS.md#design-notes) |
| stage0 | static.rust-lang.org | beta 2025-02-18 (= 1.85.0-beta) | pinned by rust-mos' `src/stage0`; sha256s copied from that file |

Using the latest commit of each project is not likely to work — these are the exact commits
upstream rust-mos builds and tests together (its CI workflow pins them), which also makes the
build reproducible. Upstream also publishes this set as the container image
`mrkits/rust-mos:13f2838f9-334fc98-8f3a80f8` (the tag is the three short commit hashes). This
flake builds the same commits directly, without a container.

---

For internals — the `initenv` options, allocator setup, inspecting assembly, testing, the binary
cache, and design notes — see [DETAILS.md](DETAILS.md).
