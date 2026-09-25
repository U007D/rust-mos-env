# Fixed-output derivation: the bundled example's dependency tree, vendored for
# the offline `checks` build.
#
# Why this exists: bin/hello-world depends on c64_pac (git) and on crates.io
# crates, and the check sandbox has no network. A fixed-output derivation is the
# one place allowed to fetch — its pinned hash is the contract — so the sources
# land in the store here and the check builds against them.
#
# The inputs are hello-world's manifest and the shared lockfile, nothing else:
# vendoring depends on the dependency graph, not on the example's code, so
# editing main.rs must not invalidate this. The root workspace deliberately
# includes user projects with `bin/*`; their manifests are untracked and do not
# enter a flake source, while their old entries may remain in Cargo.lock. Build
# a temporary workspace containing only hello-world so those entries cannot
# make `cargo vendor --locked` reject the lockfile.
#
# Re-pin (reset the hash to lib.fakeHash, then `cargo xprefetch-hashes`) whenever
# the example's dependencies change — until then a change fails loudly with a
# hash mismatch rather than drifting silently.
{
  lib,
  stdenvNoCC,
  git,
  cacert,
  rust-mos-stage0,
  pins,
}:
let
  # A fixed-output derivation's output path otherwise depends only on its
  # declared output hash. Including the dependency inputs in its name makes a
  # manifest or lockfile change request a fresh store path, forcing Nix to
  # verify that the vendor hash was re-pinned instead of reusing an old tree.
  inputHash = builtins.hashString "sha256" (
    builtins.readFile ../Cargo.lock + builtins.readFile ../bin/hello-world/Cargo.toml
  );
  inputFingerprint = builtins.replaceStrings [ "sha256-" "/" "+" "=" ] [ "" "-" "-" "" ] inputHash;
in
stdenvNoCC.mkDerivation {
  pname = "rust-mos-check-vendor";
  version = "hello-world-${builtins.substring 0 16 inputFingerprint}";

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.lock
      ../bin/hello-world/Cargo.toml
    ];
  };

  outputHash = pins.example-vendor-hash;
  outputHashAlgo = "sha256";
  outputHashMode = "recursive";

  nativeBuildInputs = [
    git
    rust-mos-stage0
  ];

  SSL_CERT_FILE = "${cacert}/etc/ssl/certs/ca-bundle.crt";
  GIT_SSL_CAINFO = "${cacert}/etc/ssl/certs/ca-bundle.crt";
  CARGO_HTTP_CAINFO = "${cacert}/etc/ssl/certs/ca-bundle.crt";

  dontConfigure = true;
  dontBuild = true;
  dontFixup = true;

  installPhase = ''
    runHook preInstall
    export HOME=$TMPDIR
    export CARGO_HOME=$TMPDIR/cargo-home

    cat > Cargo.toml <<'EOF'
    [workspace]
    resolver = "3"
    members = ["bin/hello-world"]
    EOF

    # A workspace member with no src/lib.rs, src/main.rs, [lib] or [[bin]] fails
    # to parse, so give hello-world the stub Cargo insists on. The vendor tree
    # is unaffected by what the file contains.
    mkdir -p bin/hello-world/src
    : > bin/hello-world/src/main.rs

    # Prune entries from local bin projects that are absent from this temporary
    # workspace. The subsequent locked vendor command consumes this resulting
    # lockfile; a changed hello-world dependency graph produces a fixed-output
    # hash mismatch and requires an explicit re-pin.
    cargo metadata --format-version=1 > /dev/null

    # --versioned-dirs matches the rust-mos-src vendor tree's layout, so the
    # check can merge the two directories (see check.nix).
    cargo vendor --versioned-dirs --locked vendor > vendor-config.toml

    mkdir -p $out
    cp -r vendor $out/vendor
    cp vendor-config.toml $out/vendor-config.toml
    runHook postInstall
  '';
}
