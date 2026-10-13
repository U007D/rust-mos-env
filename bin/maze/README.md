# Rust on the Commodore 64

Built with [rust-mos](https://github.com/mrk-its/rust-mos) (Rust compiler + [llvm-mos](https://llvm-mos.org) 6502 backend). The toolchain runs in a Linux container; source and artifacts stay on the host. Output is a normal C64 `.prg`. Works on macOS, Linux, and Windows with [Podman](https://podman.io) (or Docker — same commands, s/podman/docker/).

## Prerequisites

**Podman**

- macOS: `brew install podman && podman machine init --cpus 8 --memory 8192 && podman machine start`
- Windows (WSL2): `winget install RedHat.Podman`, then `podman machine init && podman machine start`
- Linux: `sudo apt install podman` / `sudo dnf install podman` (no `machine` step)

**VICE** (emulator + `c1541` disk tool)

- macOS: `brew install vice` · Linux: `sudo apt/dnf install vice` · Windows: [vice-emu.sourceforge.io](https://vice-emu.sourceforge.io/)

## Toolchain image

```sh
podman pull docker.io/mrkits/rust-mos:13f2838f9-334fc98-8f3a80f8
podman tag  docker.io/mrkits/rust-mos:13f2838f9-334fc98-8f3a80f8 rust-mos
```

Use this pinned tag, **not `:latest`** (stale, amd64-only). The pinned tag is multi-arch — native on Apple Silicon.

## Build alias

The toolchain is on the container's login-shell PATH, so run everything via `bash -lc`:

```sh
# bash/zsh
alias mos='podman run --rm -it -v "$PWD":/workspace -w /workspace rust-mos bash -lc'
# fish
alias --save mos 'podman run --rm -it -v "$PWD":/workspace -w /workspace rust-mos bash -lc'
# PowerShell
function mos { param($cmd) podman run --rm -it -v "${PWD}:/workspace" -w /workspace rust-mos bash -lc $cmd }
```

SELinux hosts (Fedora/RHEL): use `-v "$PWD":/workspace:z`.

## Project setup

`.cargo/config.toml` (this file, **not** `Cargo.toml`):

```toml
[build]
target = "mos-c64-none"

[unstable]
build-std = ["core", "alloc"]
```

`Cargo.toml` — LTO is **required** (dead-strips `u128` paths in `core` that the 6502 backend can't compile):

```toml
[dependencies]
ufmt-stdio = "0.4"

[profile.release]
opt-level = "s"
lto = true
panic = "abort"
strip = true

[profile.dev]
opt-level = 2
lto = true
```

`src/main.rs`:

```rust
#![no_std]
#![feature(start)]

use core::panic::PanicInfo;
use ufmt_stdio::*;

#[start]
fn main(_argc: isize, _argv: *const *const u8) -> isize {
    println!("HELLO, C64!");
    0
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! { loop {} }
```

## Pin dependencies (important)

The image ships a **frozen Feb-2024 nightly** (`cargo 1.78.0-dev`); crates released after ~March 2024 fail to parse or compile (``feature `edition2024` is required``, `#[diagnostic] ... experimental`). Pin the usual offenders — copy/paste, skipping any "package not found" (that crate just isn't in your tree):

```sh
mos 'cargo update -p home --precise 0.5.9'
mos 'cargo update -p wasm-bindgen --precise 0.2.92'
mos 'cargo update -p prettyplease --precise 0.2.16'
mos 'cargo update -p syn@2.0.119 --precise 2.0.52'   # use the 2.x version from your Cargo.lock
mos 'cargo update -p proc-macro2 --precise 1.0.78'
mos 'cargo update -p quote --precise 1.0.35'
mos 'cargo update -p cc --precise 1.0.83'
```

For anything else that breaks: pin it to the newest release **dated before March 2024** on crates.io; if cargo refuses (`required by package X`), downgrade `X` first. In your own `[dependencies]`, prefer era-correct versions outright (e.g. `rand = "=0.8.5"` with `default-features = false`).

When the build is green, **commit `Cargo.lock`** and never run a bare `cargo update` — the lockfile keeps floating dependencies compatible with the frozen toolchain.

## Build & run

```sh
mos 'cargo build --release --target mos-c64-none'
```

`target/mos-c64-none/release/<crate-name>` is already a PRG. Always use `--release`.

```sh
x64sc target/mos-c64-none/release/<crate-name>          # run in emulator

c1541 -format mydisk,01 d64 mydisk.d64 \                 # put on a disk image
      -write target/mos-c64-none/release/<crate-name> myprog
```

CBM DOS can't overwrite: `-delete myprog` before re-writing. For THEC64: FAT32 USB stick, copy the `.d64` (name it `THEC64-drive8.d64` to auto-mount as drive 8), then `LOAD"*",8,1` / `RUN`.

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| `executable file 'cargo' not found` | Run via `bash -lc` (see alias) |
| ``feature `edition2024` is required`` / E0658 `#[diagnostic]` | Pin that crate pre-March-2024 (see above) |
| `can't find crate for core` (E0463) | `[build]`/`[unstable]` go in `.cargo/config.toml`, not `Cargo.toml` |
| `LLVM ERROR ... s128` / SIGSEGV compiling `core` | Set `lto = true` in both profiles; avoid `u128` |
| `rustc` killed (SIGKILL) | VM out of RAM: `podman machine stop; podman machine set --memory 8192; podman machine start` |
| `image platform ... does not match` warning | Use the multi-arch tag, or add `--platform linux/amd64` |
| c1541 `ERR = 63, FILE EXISTS` | `-delete <name>` first |
| c1541 `OPENCBM ... failed` | Harmless — only needed for real IEC-cable drives |

**References:** [rust-mos](https://github.com/mrk-its/rust-mos) · [rust-mos-hello-world](https://github.com/mrk-its/rust-mos-hello-world) · [mos-hardware](https://github.com/mlund/mos-hardware) · [llvm-mos wiki](https://llvm-mos.org/wiki/Rust) · [VICE](https://vice-emu.sourceforge.io/)