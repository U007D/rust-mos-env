# Handoff: rust-mos toolchain Nix flake

> **HISTORICAL — do not rely on this document.** It is the original authoring-session
> handoff, written before the toolchain was ever built. Since then the build works
> (`nix flake check` passes), the repo was renamed `rust-mos-env`, and the layout was
> restructured (`nix/` → `toolchain/`, the C64 example + check moved under `c64/`, and
> `check-prg`/`check-rainbow-border` → `c64/check.nix` as `checks.<system>.c64-rainbow-border`).
> The paths and status below are kept only as a record of the initial state. **See
> [README.md](README.md) for the current layout and instructions.**

## Goal
Nix flake building the **rust-mos** toolchain (rustc fork with LLVM-MOS 6502
backend) **from source**, natively per platform — no containers, no patchelf'd
foreign binaries in outputs. Priority `aarch64-darwin` (M3 Max), then
`x86_64-linux`, `aarch64-linux` (`x86_64-darwin` wired but untested).

## State: authored + statically verified, NEVER evaluated or built
The authoring session ran in a network-locked sandbox: no Nix install, no
GitHub egress. All repo facts were verified via a web relay against the actual
repos; the Nix code is hand-reviewed only (4 bugs found+fixed in review; shell
logic like the offline cargo-config merge was simulated in bash and
TOML-validated). The Rust xtask IS compiled, clippy-clean, unit- and
integration-tested (mock `nix`). Expect first-`nix flake check` iteration.

## Repo layout (delivered as rust-mos-flake.tar.gz)
- `flake.nix` — packages.{llvm-mos, llvm-mos-sdk, rust-mos, default} +
  plumbing attrs (stage0, rust-mos-src, check-vendor, llvm-mos-source,
  llvm-mos-sdk-source); checks.c64-prg; devShells.default. Input: nixpkgs
  nixos-25.05. nixConfig Cachix block commented with YOUR-CACHE placeholders.
- `nix/pins.nix` — all revs + hashes. 4 hashes are `sha256-AAA…=` placeholders
  marked `# PREFETCH:<flake-attr>`.
- `nix/stage0.nix` — FODs for beta-2025-02-18 rustc/cargo/rust-std tarballs
  (= 1.85.0-beta; exactly what rust-mos' src/stage0 pins), one merged prefix,
  autoPatchelf on Linux (build-time only). sha256s copied from src/stage0.
- `nix/llvm-mos.nix` — mrk-its/llvm-mos, clang/cmake/caches/MOS.cmake cache
  + overrides: LLVM_INSTALL_TOOLCHAIN_ONLY=OFF, LLVM_BUILD/LINK_LLVM_DYLIB=ON,
  LLVM_TARGETS_TO_BUILD=X86;AArch64 (cache leaves it empty; rustc needs host
  codegen), MinSizeRel, `ninja install` (not install-distribution).
- `nix/llvm-mos-sdk.nix` — upstream SDK; `-DLLVM_MOS=${llvm-mos}` defeats its
  prebuilt-compiler download (cmake/bootstrap-compiler.cmake).
- `nix/rust-mos-src.nix` — THE network FOD: shallow clone rust-mos, init 3
  submodules (src/tools/cargo, library/stdarch, library/backtrace;
  src/llvm-project stubbed with mkdir libunwind, as upstream docker does),
  rewrite library/Cargo.toml compiler_builtins `branch="mos-0.1.148"` →
  `rev="4505c7df…"` (guarded), `cargo fetch` to sync library/Cargo.lock
  (committed lock is out of sync with the git patch — verified), then
  `cargo vendor --versioned-dirs --sync` over root + library + src/tools/cargo
  + src/bootstrap, capturing emitted config to .cargo/config.toml; strip .git.
- `nix/rust-mos.nix` — configure.py + x.py build/install --stage 2. Stage0 fed
  via --set=build.rustc/cargo; external LLVM via
  --set=target.<triple>.llvm-config + --enable-llvm-link-shared;
  --enable-vendor --enable-extended --tools=rustc,rustdoc,cargo
  --disable-docs --enable-rpath; llvm.download-ci-llvm=false. postPatch
  applies the **c_uint gate** (see below) with count guards; Darwin
  /usr/bin/strip→llvm-strip substitution (guarded). postInstall: copies
  library/ to lib/rustlib/src/rust (rust-src for -Zbuild-std) and generates
  targets/mos-{sim,c64,a800xl}-none.json via nix/mos-targets.py.
- `nix/mos-targets.py` — replica of upstream create_mos_targets.py (recovered
  from branch mos_target_latest; absent at the pinned rev though its own
  build.sh references it): dump built-in mos-unknown-none spec, set
  linker=mos-<platform>-clang + vendor.
- `nix/check-vendor.nix` — FOD vendoring compiler_builtins branch
  mos-0.1.150 under its **branch-form** source id (must match the forked
  cargo's injected id exactly).
- `nix/check-prg.nix` — offline check: merges rust-mos-src's captured vendor
  config (retargeted to store path) + check-vendor's (crates-io block awk'd
  out, replace-with renamed) + [net] offline; `cargo build --release
  --target mos-c64-none -Zbuild-std=core,alloc`; asserts output starts
  `01 08` ($0801 PRG load address).
- `checks/c64-prg/` — #![no_std] #![no_main], `#[no_mangle] extern "C" fn
  main() -> !` (`#[start]` is E0557/removed in this generation), border-color
  loop, `const _: () = assert!(size_of::<core::ffi::c_uint>() == 2)`,
  lto=true in BOTH profiles (target has requires_lto; non-LTO historically
  died on u128: `LLVM ERROR: unable to legalize … s128`).
- `xtask/` + `.cargo/config.toml` — canonical cargo-xtask (zero deps, std
  only, tested): `cargo xtask prefetch-hashes` runs `nix build .#<attr>`,
  harvests `got: sha256-…`, substitutes on the PREFETCH line, re-builds to
  verify. Idempotent. No root workspace (deliberate; empty [workspace] in
  both crates' manifests).

## Verified pins (from the repos' own CI config; do not re-litigate)
Docker tag `mrkits/rust-mos:13f2838f9-334fc98-8f3a80f8` decodes
`<llvm-mos>-<sdk>-<rust-mos>` (NOT the order originally assumed):
- llvm-mos: **mrk-its/llvm-mos** (fork; commit absent upstream)
  `13f2838f909ca839ccb4d9a6c808de7b2ea60911` (branch cmp_fix)
- llvm-mos-sdk: **upstream llvm-mos/llvm-mos-sdk**
  `334fc9823745284938bfba543112f95bfb489b50` (v21.0.x era)
- rust-mos: `8f3a80f87cea2bd236d953027cb5bce9dfc63b89` (tip of
  mos_rustc_1.87; src/version=1.87.0) — chosen over mos_1.87_gh 969386d
  (delta = workflow files only)
- forked cargo: submodule src/tools/cargo → mrk-its/cargo branch
  compiler_builtins_patched (tip a2df727d)
- compiler_builtins: mrk-its fork; branch mos-0.1.148 tip `4505c7df…`
  (referenced by library/Cargo.toml [patch.crates-io]); branch mos-0.1.150
  tip `090b8f3c…` (injected by forked cargo's standard_lib.rs into every
  -Zbuild-std resolve). BOTH needed; the "148/150 duality".
- stage0: src/stage0 pins compiler_date=2025-02-18, version=beta =
  **1.85.0-beta** (not ≈1.86 — this is why nixpkgs-rustc stage0 was rejected
  in favor of exact tarball FODs).
- mos-unknown-none is a BUILT-IN target at this rev
  (spec/targets/mos_unknown_none.rs: c_int_width 16, requires_lto,
  linker mos-clang). RUST_TARGET_PATH only needed for the JSON variants.
- **c_uint gate is real**: at 8f3a80f8, library/core/src/ffi/primitives.rs
  16-bit cfg list is `any(target_arch = "avr", target_arch = "msp430")` —
  no mos. The 1.78-era fix was lost in the rebase. Flake re-adds it.
- MOS.cmake at pinned llvm-mos rev DOES configure compiler-rt builtins
  (LLVM_ENABLE_RUNTIMES=compiler-rt, LLVM_BUILTIN/RUNTIME_TARGETS=
  mos-unknown-unknown) and sets LLVM_INSTALL_TOOLCHAIN_ONLY=ON (overridden).
- SDK C64 link emits the PRG directly: link.ld ram @0x0801,
  SHORT(ORIGIN(ram)) prefix + basic-header.S SYS stub.

## Next steps (user machine, M3 Max)
1. `cargo xtask prefetch-hashes` (fills 4 FOD hashes; several GB; each vendor
   FOD runs twice by nature).
2. `nix flake check` — builds llvm-mos (~¾–1½ h est.), rust-mos (~1–2 h est.),
   then the offline PRG check.
3. VICE: `x64sc <check output>/c64-check.prg` → cycling border.
4. Cachix: create cache, `nix build .#llvm-mos .#llvm-mos-sdk .#rust-mos
   --print-out-paths | cachix push <name>`, fill nixConfig in flake.nix.

## Known risks, ranked (labeled inference; details in README)
1. Offline -Zbuild-std resolution in checks.c64-prg — forked cargo's
   injection is nonstandard; the two vendored source replacements may need
   adjustment in nix/check-prg.nix (error will name the missing source id).
   If it fails offline but works in `nix develop` (online), that's the spot.
2. library/Cargo.lock sync — if x.py still complains, use
   `cargo update -p compiler_builtins` in rust-mos-src.nix instead of fetch.
3. LLVM version handshake (rustc 1.87 accepts 18–20; llvm-mos should report
   20.x — unverified string).
4. SDK CMake version-stamping without .git (fetchFromGitHub tree).
5. Nix eval errors — the .nix files were never parsed by Nix.
6. x86_64-darwin entirely untested.

## Session decisions log (user-confirmed)
- rust-mos ref = 8f3a80f8 (user corrected an initial mis-selection of 969386d).
- Stage0 = pinned beta tarball FODs (user's original plan; assistant's
  nixpkgs-1.86 suggestion withdrawn after the 1.85-beta finding).
- Cachix = placeholder name; user will create cache later.
- Prefetch tool = Rust cargo-xtask (rewritten from bash for auditability),
  canonical alias layout.
- devShell guards against stock rustc/cargo shadowing (warns on entry; forked
  cargo is REQUIRED for -Zbuild-std correctness).
