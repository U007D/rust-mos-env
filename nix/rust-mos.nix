# rust-mos: mrk-its' rustc 1.87.0-dev fork with the MOS backend, built from
# source against the Nix llvm-mos, bootstrapped by the exact stage0 its
# src/stage0 pins. Follows nixpkgs' rustc.nix pattern (configure.py + x.py),
# with the nixpkgs fork-specific patches dropped (they target release
# tarballs of stock rustc and do not apply here).
#
# Installed layout (single output):
#   bin/rustc, bin/rustdoc
#   bin/cargo                      <- mrk-its' forked cargo (submodule
#                                     src/tools/cargo, branch
#                                     compiler_builtins_patched). REQUIRED:
#                                     it injects the mos compiler_builtins
#                                     into -Zbuild-std resolution. A stock
#                                     cargo must never shadow it.
#   lib/rustlib/src/rust/library   <- rust-src for -Zbuild-std (with the
#                                     c_uint fix and the rev-pinned
#                                     compiler_builtins patch)
#   targets/mos-{sim,c64,a800xl}-none.json  <- RUST_TARGET_PATH points here
{
  lib,
  stdenv,
  python3,
  cmake,
  pkg-config,
  openssl,
  zlib,
  curl,
  libiconv,
  llvm-mos,
  rust-mos-stage0,
  rust-mos-src,
  pins,
}:
let
  triple = stdenv.hostPlatform.rust.rustcTarget;
in
stdenv.mkDerivation {
  pname = "rust-mos";
  version = pins.rust-mos.version;

  src = rust-mos-src;

  nativeBuildInputs = [
    python3
    cmake # libgit2-sys (forked cargo build)
    pkg-config
  ];
  buildInputs = [
    openssl
    zlib
    curl
  ]
  ++ lib.optionals stdenv.hostPlatform.isDarwin [ libiconv ];

  # Everything is pre-vendored in rust-mos-src; hard-fail on any network use.
  CARGO_NET_OFFLINE = "true";

  postPatch = ''
    patchShebangs src/etc x.py 2>/dev/null || true

    # --- the c_uint gate (critical correctness item) ----------------------
    # llvm-mos C int is 16-bit and the target spec says c_int_width = "16",
    # but this generation ships core::ffi with c_int/c_uint = 32-bit (the
    # 1.78-era patch was not carried over the rebase; verified at 8f3a80f8).
    # Add mos to the 16-bit cfg list, with count guards.
    prim=library/core/src/ffi/primitives.rs
    before=$(grep -c 'any(target_arch = "avr", target_arch = "msp430")' "$prim" || true)
    if [ "$before" -lt 1 ]; then
      echo "GUARD FAILED: 16-bit c_int cfg not found in $prim - fork layout changed, refusing to build" >&2
      exit 1
    fi
    sed -i 's/any(target_arch = "avr", target_arch = "msp430")/any(target_arch = "avr", target_arch = "msp430", target_arch = "mos")/g' "$prim"
    after=$(grep -c 'any(target_arch = "avr", target_arch = "msp430", target_arch = "mos")' "$prim" || true)
    if [ "$after" -ne "$before" ]; then
      echo "GUARD FAILED: c_uint patch applied $after/$before times in $prim" >&2
      exit 1
    fi
    echo "c_uint gate: patched $after cfg site(s) in $prim"
  ''
  + lib.optionalString stdenv.hostPlatform.isDarwin ''
    # nixpkgs carries this substitution for Darwin: rustc shells out to
    # /usr/bin/strip when stripping dylibs; point it at llvm-strip instead.
    # Guarded: skip silently if the fork's code drifted.
    if grep -q '"/usr/bin/strip"' compiler/rustc_codegen_ssa/src/back/link.rs; then
      substituteInPlace compiler/rustc_codegen_ssa/src/back/link.rs \
        --replace-fail '"/usr/bin/strip"' '"${llvm-mos}/bin/llvm-strip"'
    fi
  '';

  # rustc's configure rejects autotools-style --build/--host values; pass
  # rust triples explicitly (nixpkgs does the same).
  configurePlatforms = [ ];
  configurePhase = ''
    runHook preConfigure
    python3 src/bootstrap/configure.py \
      --prefix=$out \
      --sysconfdir=$out/etc \
      --build=${triple} --host=${triple} --target=${triple} \
      --set=build.rustc=${rust-mos-stage0}/bin/rustc \
      --set=build.cargo=${rust-mos-stage0}/bin/cargo \
      --enable-vendor \
      --enable-extended \
      --tools=rustc,rustdoc,cargo \
      --disable-docs \
      --enable-rpath \
      --enable-llvm-link-shared \
      --set=llvm.download-ci-llvm=false \
      --set=target.${triple}.llvm-config=${llvm-mos}/bin/llvm-config
    runHook postConfigure
  '';

  buildPhase = ''
    runHook preBuild
    python3 x.py build --stage 2 -j $NIX_BUILD_CORES
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    python3 x.py install --stage 2 -j $NIX_BUILD_CORES
    runHook postInstall
  '';

  postInstall = ''
    # --- rust-src for -Zbuild-std -----------------------------------------
    # Same content the docker image got from the rust-src dist tarball:
    # the (patched) library workspace, locks included. compiler_builtins in
    # its [patch.crates-io] is already rev-pinned by rust-mos-src.
    srcdir=$out/lib/rustlib/src/rust
    mkdir -p "$srcdir/src/llvm-project/libunwind"
    rm -rf "$srcdir/library" # in case x.py install already placed one
    cp -r library "$srcdir/library"

    # --- mos-*-none target JSONs (RUST_TARGET_PATH) ------------------------
    # Replica of upstream create_mos_targets.py; see nix/mos-targets.py.
    $out/bin/rustc --target mos-unknown-none \
      -Z unstable-options --print target-spec-json > mos-spec.json
    python3 ${./mos-targets.py} mos-spec.json $out/targets

    # x.py install bookkeeping not needed downstream
    rm -rf $out/lib/rustlib/{install.log,uninstall.sh,rust-installer-version,manifest-*,components}
  '';

  # rustc rlibs contain bitcode-ish sections strip would mangle; nixpkgs
  # also avoids stripping rustc's lib dir.
  stripExclude = [ "*.rlib" ];

  enableParallelBuilding = true;
  requiredSystemFeatures = [ "big-parallel" ];

  passthru = {
    inherit llvm-mos rust-mos-stage0 rust-mos-src;
    targetsDir = "targets";
  };

  meta = {
    description = "rustc 1.87.0-dev with the LLVM-MOS 6502 backend (mrk-its fork) + forked cargo";
    homepage = "https://github.com/mrk-its/rust-mos";
    license = with lib.licenses; [
      mit
      asl20
    ];
    platforms = [
      "x86_64-linux"
      "aarch64-linux"
      "aarch64-darwin"
      "x86_64-darwin"
    ];
  };
}
