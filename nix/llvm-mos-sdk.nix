# llvm-mos-sdk: 6502 platform libraries (crt0, linker scripts, per-target
# clang configs) built with the freshly built mos-clang.
#
# The SDK's CMake normally *downloads* a prebuilt llvm-mos compiler
# (cmake/bootstrap-compiler.cmake) when it can't find one. Passing
# -DLLVM_MOS=${llvm-mos} makes cmake/find-mos-compiler.cmake pick up our
# Nix-built compiler and skips every network path — required in the sandbox.
{
  lib,
  stdenv,
  fetchFromGitHub,
  cmake,
  ninja,
  llvm-mos,
  pins,
}:
stdenv.mkDerivation {
  pname = "llvm-mos-sdk";
  version = "21-unstable-2025-01-01-${builtins.substring 0 7 pins.llvm-mos-sdk.rev}";

  src = fetchFromGitHub {
    inherit (pins.llvm-mos-sdk) owner repo rev hash;
  };

  nativeBuildInputs = [
    cmake
    ninja
  ];

  cmakeFlags = [
    "-DLLVM_MOS=${llvm-mos}"
    "-DLLVM_MOS_BUILD_EXAMPLES=OFF"
    "-DLLVM_MOS_TEST_SUITE=OFF"
  ];

  # The C64 target's link step emits a PRG image directly: link.ld places RAM
  # at 0x0801 and prepends SHORT(ORIGIN(ram)) — the 2-byte load address —
  # while basic-header.S provides the BASIC "SYS" stub. Nothing extra to do
  # here; consumers link with mos-c64-clang (installed by this package).

  passthru = { inherit (pins) llvm-mos-sdk; };

  meta = {
    description = "LLVM-MOS platform SDK (C64 and friends), built against the Nix llvm-mos";
    homepage = "https://github.com/llvm-mos/llvm-mos-sdk";
    license = lib.licenses.mit;
    platforms = llvm-mos.meta.platforms;
  };
}
