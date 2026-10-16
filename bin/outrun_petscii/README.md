# outrun_petscii

A Rust port of the **[OutRun PETSCII](https://csdb.dk/release/?id=189844)** Commodore 64 demo,
compiled to a real `.prg` that runs on actual C64 hardware or in VICE.

Credits (original demo): code & graphics by **aNdy** (CSDb 6404), music by
**jason c. brooke** (CSDb 8136). The original 6502 source is kept verbatim under
`original_asm_by_aNdy/` and is the reference the port is written against.

> **Status: in progress.** The video-system detection, charset setup and asset placement are
> ported; `src/main.rs` is still a joystick → border-colour loop standing in for the demo
> proper. See `original_asm_by_aNdy/source/source.asm` for what remains.

## Building

This crate is a member of the **rust-mos-env** workspace one level up, which owns the
toolchain (llvm-mos + rust-mos built from source via Nix), the cargo aliases and the
emulator wiring. Nothing toolchain-related lives here.

From this directory or anywhere else in the workspace:

```sh
cargo xnixdev          # dev shell: forked cargo, the mos-c64-none target and x64sc on PATH
cargo xbuild_c64 --release   # -> target/mos-c64-none/release/outrun_petscii  (a .prg)
cargo xrun_c64               # build and launch it in VICE
cargo xcheck_c64             # fast type-check for the C64 target
cargo xasm_c64 [FUNCTION]    # dump the generated 6502 assembly
```

Inside the dev shell the `cargo xbuild` / `xrun` / `xcheck` / `xasm` shims default to the C64.
Use `cargo xnixdev` rather than plain `nix develop`: this crate is its own git repository, and
nix stops its upward search for a `flake.nix` at that boundary, so `nix develop` fails here.

Plain `cargo build` targets your **host**, not the C64 — the workspace deliberately sets no
default target so the host-side tooling still builds. `build.rs` prints a warning if you
reach for a host command that has a cross-compiling counterpart.

## This crate

C64 only. `build.rs` resolves the linker script from the target vendor and accepts `c64`
alone; `memory-c64.x` pins the two assets to addresses the hardware requires:

| section | address | why |
|---|---|---|
| `.font` | `$2000` | VIC-II character base — a hardware constraint, not a preference |
| `.music` | `$b000` | under the BASIC ROM, which the SDK's startup has already unmapped |

Hardware access goes through [`c64_pac`](https://github.com/u007d/c64_pac), a typed
peripheral-access crate generated from a CMSIS-SVD description of the C64's I/O — so
register writes read as `p.vic.extcol().write(|w| w.color().red())` rather than as pokes to
magic addresses.

```
outrun_petscii/
├── Cargo.toml              # edition 2024, no_std
├── build.rs                # stages memory-c64.x as -Tmemory.x
├── memory-c64.x            # .font @ $2000, .music @ $b000
├── data/                   # font.chr, music.prg, orp-scrn.chpr
├── original_asm_by_aNdy/   # the original ACME source, unmodified
└── src/
    ├── main.rs             # #![no_std] #![no_main]
    ├── init.rs, init/      # video-system detection, charset, NMI setup
    ├── joystick/           # typed joystick state
    ├── error.rs, panic.rs, threading_model.rs
    └── shared_const.rs
```
