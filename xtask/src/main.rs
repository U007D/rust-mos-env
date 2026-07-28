//! Repo tasks, cargo-xtask style: `cargo xtask <task>` (alias wired in
//! .cargo/config.toml; run from the repo root).
//!
//! # `cargo xtask init-buildenv`
//!
//! One idempotent command that gives you a working build environment: installs
//! plain upstream Nix if it's missing (pinned NixOS/nix-installer binary,
//! checksum-verified), generates `flake.lock` if absent, fills the `PREFETCH:`
//! source hashes in `nix/pins.nix` if they're still placeholders, then runs the
//! flake's full check (`nix flake check` — builds the toolchain and compiles a
//! C64 program offline to prove it actually works). Pass `--build` to build just
//! the rust-mos toolchain and skip the checks. Every step skips itself if
//! already done, so re-running just rebuilds.
//!
//! # `cargo xtask prefetch-hashes`
//!
//! Pins the `PREFETCH:` placeholder hashes in `nix/pins.nix` — the standard
//! fixed-output-derivation workflow, automated: for each target,
//! `nix build .#<target> --no-link` with the placeholder hash; Nix downloads
//! the content and reports `specified: sha256-AAA… got: sha256-…`; we
//! substitute the reported hash on the marker's line and re-run the build to
//! verify it now succeeds. Run once, on a machine with network access.
//! Idempotent: already-pinned targets are skipped.
//!
//! Note: Nix keys fixed-output derivations by their declared hash, so each
//! vendor FOD unavoidably runs twice — once to learn the hash, once (after
//! substitution) to realize it. `rust-mos-src` downloads several GB each
//! time; the stage0 toolchain (~400 MB) is built first automatically since
//! the vendor step runs on the pinned beta cargo.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

/// Flake attributes to prefetch, cheap source fetches first, then the
/// expensive vendor FODs. Each must have a matching `# PREFETCH:<name>`
/// marker in nix/pins.nix.
const TARGETS: [&str; 4] = [
    "llvm-mos-source",
    "llvm-mos-sdk-source",
    "rust-mos-src",
    "check-vendor",
];

/// `lib.fakeHash`: what an unpinned entry looks like, byte for byte.
const PLACEHOLDER: &str = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

/// Global flags threaded onto every `nix` call so the modern CLI works even
/// when `nix.conf` hasn't enabled flakes (we deliberately don't edit it).
const NIX_EXPERIMENTAL: [&str; 2] = ["--extra-experimental-features", "nix-command flakes"];

/// Pinned `NixOS/nix-installer` release. This tag also pins the upstream Nix
/// version it installs; bumping it is a deliberate action that requires
/// refreshing `INSTALLER_SHA256` from the release's `SHA256SUMS` asset.
const NIX_INSTALLER_VERSION: &str = "2.35.1";

/// sha256 of each pinned installer binary, from the release's `SHA256SUMS`.
/// Keyed by the installer's arch triple (which is also the asset suffix).
const INSTALLER_SHA256: &[(&str, &str)] = &[
    (
        "aarch64-darwin",
        "82723616373d0c3f0d07b892f5f5c023da825b8969a2351c7055926d0bcf5553",
    ),
    (
        "aarch64-linux",
        "7e6e2f753144d7f19b16a9fce4b354cb0f46d1d47e6908bfb9186c89e0e0e649",
    ),
    (
        "x86_64-linux",
        "3b49a0b91820accb76e3d9ff7ed64fc430121b9fafb3869b0d549721fbeb4c85",
    ),
];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("init-buildenv") => init_buildenv(&args[1..]),
        Some("prefetch-hashes") => prefetch_hashes(),
        Some(other) => {
            eprintln!("unknown task: {other}");
            usage();
            ExitCode::FAILURE
        }
        None => {
            usage();
            ExitCode::FAILURE
        }
    }
}

fn usage() {
    eprintln!("usage: cargo xtask <task>");
    eprintln!();
    eprintln!("tasks:");
    eprintln!("  init-buildenv [--build]   install Nix if needed, then build + check the flake");
    eprintln!("                            (default runs `nix flake check`; --build builds only");
    eprintln!("                             the rust-mos toolchain, skipping the checks)");
    eprintln!("  prefetch-hashes           pin the PREFETCH placeholder hashes in nix/pins.nix");
    eprintln!("                            (runs `nix build`, needs network; idempotent)");
}

/// The repo root is one level up from this crate's manifest — never derived
/// from the cwd, so tasks behave the same from any invocation directory.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("cannot canonicalize repo root")
}

// ---------------------------------------------------------------------------
// init-buildenv
// ---------------------------------------------------------------------------

struct InitFlags {
    build_only: bool,
}

fn parse_init_flags(args: &[String]) -> Result<InitFlags, String> {
    let mut build_only = false;
    for a in args {
        match a.as_str() {
            "--build" => build_only = true,
            other => return Err(format!("unknown flag: {other}")),
        }
    }
    Ok(InitFlags { build_only })
}

/// What kind of platform we're on, from the point of view of *installing* Nix.
#[derive(Debug, PartialEq)]
enum Platform {
    /// A pinned installer binary exists for this arch triple (the asset suffix).
    Installer(&'static str),
    /// Intel macOS: no pinned binary is published — use the official script.
    IntelMac,
    /// Windows: Nix can't install natively — needs WSL2.
    Windows,
    Unsupported,
}

fn resolve_platform(os: &str, arch: &str) -> Platform {
    match (os, arch) {
        ("macos", "aarch64") => Platform::Installer("aarch64-darwin"),
        ("linux", "x86_64") => Platform::Installer("x86_64-linux"),
        ("linux", "aarch64") => Platform::Installer("aarch64-linux"),
        ("macos", "x86_64") => Platform::IntelMac,
        ("windows", _) => Platform::Windows,
        _ => Platform::Unsupported,
    }
}

fn init_buildenv(args: &[String]) -> ExitCode {
    match init_buildenv_inner(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ERROR [init-buildenv]: {e}");
            ExitCode::FAILURE
        }
    }
}

fn init_buildenv_inner(args: &[String]) -> Result<(), String> {
    let flags = parse_init_flags(args)?;
    let root = repo_root();
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    // 1. Ensure Nix. If it's already present we use it regardless of platform
    //    (e.g. an Intel Mac that installed Nix by hand still builds fine).
    let nix = match find_nix() {
        Some(n) => {
            println!("Using existing Nix: {}", n.display());
            n
        }
        None => match resolve_platform(os, arch) {
            Platform::Installer(installer_arch) => install_nix(installer_arch, &root)?,
            Platform::IntelMac => return Err(intel_mac_message()),
            Platform::Windows => return Err(windows_message()),
            Platform::Unsupported => {
                return Err(format!("unsupported platform: {os}/{arch}"));
            }
        },
    };

    // 2. Ensure flake.lock (deterministic — flake.nix pins nixpkgs to an exact rev).
    let lock_generated = ensure_lock(&nix, &root)?;

    // 3. Ensure the source hashes are filled.
    let pins_path = root.join("nix/pins.nix");
    if !pins_path.is_file() {
        return Err(format!("{} not found", pins_path.display()));
    }
    let hashes_filled = ensure_hashes(&nix, &root, &pins_path)?;

    // 4. Build. Default is the fuller `nix flake check` (pit of success: it proves the
    //    toolchain can actually compile a C64 program offline). `--build` opts into just
    //    building the toolchain.
    if flags.build_only {
        println!("Building the rust-mos toolchain (first build is LLVM-scale: very roughly 2-4 h)…");
        let out = build_toolchain(&nix, &root)?;
        println!();
        println!("Build environment ready.");
        if !out.is_empty() {
            println!("  rust-mos: {out}");
        }
    } else {
        println!("Building + checking the flake (`nix flake check`; first build is LLVM-scale: very roughly 2-4 h)…");
        if !run_nix_inherit(&nix, &root, &["flake", "check"])? {
            return Err("`nix flake check` failed".into());
        }
        println!();
        println!("Build environment ready (full flake check passed).");
    }

    // 5. If we generated anything, remind the user to commit it.
    if lock_generated || hashes_filled {
        println!();
        println!("NOTE: flake.lock / nix/pins.nix were generated on this run. Commit the repo so");
        println!("      the next init finds them pinned:  git init && git add -A && git commit");
    }

    // 6. Next steps.
    println!();
    println!("Next:");
    println!("  restart your shell (the nix profile isn't on this process's PATH yet)");
    println!("  nix develop        # rust-mos rustc + its cargo + SDK on PATH");
    println!("  cd checks/c64-prg && cargo build --release --target mos-c64-none -Zbuild-std=core,alloc");
    Ok(())
}

fn intel_mac_message() -> String {
    "no pinned Nix installer binary is published for Intel macOS (x86_64-darwin).\n\
     Install Nix with the official script, then re-run `cargo xtask init-buildenv`:\n  \
     curl -L https://nixos.org/nix/install | sh -s -- --daemon"
        .to_string()
}

fn windows_message() -> String {
    "Nix cannot install on native Windows. Install WSL2 (`wsl --install`), then run\n\
     `cargo xtask init-buildenv` inside the WSL2 Linux shell."
        .to_string()
}

/// Locate a usable `nix`: on PATH, else the multi-user default profile, else
/// the single-user profile. Returns the bare name `nix` when it's on PATH
/// (Command resolves it), otherwise an absolute path.
fn find_nix() -> Option<PathBuf> {
    if let Ok(out) = Command::new("nix").arg("--version").output() {
        if out.status.success() {
            return Some(PathBuf::from("nix"));
        }
    }
    let default_profile = PathBuf::from("/nix/var/nix/profiles/default/bin/nix");
    if default_profile.exists() {
        return Some(default_profile);
    }
    if let Some(home) = std::env::var_os("HOME") {
        let user_profile = PathBuf::from(home).join(".nix-profile/bin/nix");
        if user_profile.exists() {
            return Some(user_profile);
        }
    }
    None
}

/// Download the pinned installer, verify its checksum, run it, and return the
/// absolute path to the installed `nix` (which is NOT on this process's PATH).
fn install_nix(installer_arch: &str, root: &Path) -> Result<PathBuf, String> {
    let want = INSTALLER_SHA256
        .iter()
        .find(|(a, _)| *a == installer_arch)
        .map(|(_, h)| *h)
        .ok_or_else(|| format!("no pinned installer checksum for {installer_arch}"))?;

    let url = format!(
        "https://github.com/NixOS/nix-installer/releases/download/{NIX_INSTALLER_VERSION}/nix-installer-{installer_arch}"
    );
    let dst = root.join("target").join(format!("nix-installer-{installer_arch}"));
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("creating {}: {e}", parent.display()))?;
    }

    println!("Downloading pinned Nix installer {NIX_INSTALLER_VERSION} ({installer_arch})…");
    let status = Command::new("curl")
        .args(["--proto", "=https", "--tlsv1.2", "-sSfL", &url, "-o"])
        .arg(&dst)
        .status()
        .map_err(|e| format!("failed to spawn curl: {e} (is curl installed?)"))?;
    if !status.success() {
        return Err("downloading the Nix installer failed".into());
    }

    let got = sha256_file(&dst)?;
    if !got.eq_ignore_ascii_case(want) {
        let _ = fs::remove_file(&dst);
        return Err(format!(
            "installer checksum mismatch for {installer_arch}: got {got}, want {want}"
        ));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dst, fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("chmod {}: {e}", dst.display()))?;
    }

    println!("Installing plain upstream Nix (multi-user daemon; sudo may prompt for your password)…");
    let status = Command::new(&dst)
        .args(["install", "--no-confirm"])
        .status()
        .map_err(|e| format!("failed to run the Nix installer: {e}"))?;
    if !status.success() {
        return Err("the Nix installer failed".into());
    }

    let nix = PathBuf::from("/nix/var/nix/profiles/default/bin/nix");
    if !nix.exists() {
        return Err(format!(
            "installer finished but {} was not found",
            nix.display()
        ));
    }
    println!("Nix installed. Restart your shell later for interactive `nix` use.");
    Ok(nix)
}

/// sha256 of a file as lowercase hex, via the platform digest tool (keeps the
/// crate dependency-free).
fn sha256_file(path: &Path) -> Result<String, String> {
    let (prog, prog_args): (&str, &[&str]) = if cfg!(target_os = "macos") {
        ("shasum", &["-a", "256"])
    } else {
        ("sha256sum", &[])
    };
    let out = Command::new(prog)
        .args(prog_args)
        .arg(path)
        .output()
        .map_err(|e| format!("failed to spawn {prog}: {e}"))?;
    if !out.status.success() {
        return Err(format!("{prog} failed on {}", path.display()));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    parse_sha256_line(&stdout)
        .ok_or_else(|| format!("could not parse {prog} output: {}", stdout.trim()))
}

/// The 64-hex digest is the first whitespace-delimited field of `shasum` /
/// `sha256sum` output (`<hex>  <file>`), returned lowercased.
fn parse_sha256_line(s: &str) -> Option<String> {
    let first = s.split_whitespace().next()?;
    if first.len() == 64 && first.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(first.to_ascii_lowercase())
    } else {
        None
    }
}

/// Generate `flake.lock` if it's missing. Returns whether it was generated.
fn ensure_lock(nix: &Path, root: &Path) -> Result<bool, String> {
    if root.join("flake.lock").is_file() {
        println!("flake.lock present.");
        return Ok(false);
    }
    println!("Generating flake.lock (pinning nixpkgs)…");
    if !run_nix_inherit(nix, root, &["flake", "lock"])? {
        return Err("`nix flake lock` failed".into());
    }
    Ok(true)
}

/// Fill any remaining `PREFETCH:` placeholders. Returns whether anything changed.
fn ensure_hashes(nix: &Path, root: &Path, pins_path: &Path) -> Result<bool, String> {
    let contents = fs::read_to_string(pins_path).map_err(|e| format!("reading pins.nix: {e}"))?;
    if !contents.contains(PLACEHOLDER) {
        println!("Source hashes already pinned.");
        return Ok(false);
    }
    println!("Filling source hashes (first run only; several GB of downloads)…");
    pin_all(nix, root, pins_path)
}

// ---------------------------------------------------------------------------
// prefetch-hashes
// ---------------------------------------------------------------------------

fn prefetch_hashes() -> ExitCode {
    let root = repo_root();
    let pins_path = root.join("nix/pins.nix");
    if !pins_path.is_file() {
        eprintln!("ERROR: {} not found", pins_path.display());
        return ExitCode::FAILURE;
    }
    let nix = match find_nix() {
        Some(n) => n,
        None => {
            eprintln!("ERROR: Nix not found. Install it or run `cargo xtask init-buildenv`.");
            return ExitCode::FAILURE;
        }
    };

    match pin_all(&nix, &root, &pins_path) {
        Ok(_) => {
            println!();
            println!("All hashes pinned. Next:");
            println!("  nix flake check   # builds llvm-mos + rust-mos + the offline C64 PRG check");
            println!("  nix develop       # mos toolchain shell");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("ERROR: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Pin every target in `TARGETS`. Returns whether any were newly pinned.
fn pin_all(nix: &Path, root: &Path, pins_path: &Path) -> Result<bool, String> {
    let mut changed = false;
    for target in TARGETS {
        if pin_target(nix, root, pins_path, target)? {
            changed = true;
        }
    }
    Ok(changed)
}

/// Pin one target. Returns whether it was newly pinned (false = already done).
fn pin_target(nix: &Path, root: &Path, pins_path: &Path, target: &str) -> Result<bool, String> {
    let marker = format!("PREFETCH:{target}");
    let contents = fs::read_to_string(pins_path).map_err(|e| format!("reading pins.nix: {e}"))?;

    let line_idx = find_marker_line(&contents, &marker)?;
    if !contents.lines().nth(line_idx).unwrap().contains(PLACEHOLDER) {
        println!("== {target}: already pinned, skipping");
        return Ok(false);
    }

    let spec = format!(".#{target}");
    println!("== {target}: running `nix build {spec}` (expected to fail with a hash mismatch)…");
    let (success, output) = nix_run_capture(nix, root, &["build", spec.as_str(), "--no-link"])?;
    if success {
        return Err(
            "build unexpectedly SUCCEEDED with the placeholder hash — refusing to continue".into(),
        );
    }

    let got = extract_got_hash(&output).ok_or_else(|| {
        format!(
            "no `got: sha256-…` in nix output — a real build error, not a hash mismatch.\n\
             ---- captured output ----\n{output}\n-------------------------"
        )
    })?;

    let patched = replace_on_line(&contents, line_idx, PLACEHOLDER, &got)?;
    fs::write(pins_path, patched).map_err(|e| format!("writing pins.nix: {e}"))?;
    println!("== {target}: pinned {got}");

    println!("== {target}: verifying (re-running the build)…");
    let (success, output) = nix_run_capture(nix, root, &["build", spec.as_str(), "--no-link"])?;
    if !success {
        return Err(format!(
            "verification build FAILED after pinning.\n\
             ---- captured output ----\n{output}\n-------------------------"
        ));
    }
    println!("== {target}: verified");
    Ok(true)
}

/// Index of the single line carrying `marker`; ambiguity is an error.
fn find_marker_line(contents: &str, marker: &str) -> Result<usize, String> {
    let hits: Vec<usize> = contents
        .lines()
        .enumerate()
        .filter(|(_, l)| l.contains(marker))
        .map(|(i, _)| i)
        .collect();
    match hits.as_slice() {
        [one] => Ok(*one),
        [] => Err(format!("marker '{marker}' not found in pins.nix")),
        many => Err(format!(
            "marker '{marker}' found on {} lines — pins.nix is malformed",
            many.len()
        )),
    }
}

/// Replace `from` with `to`, only on line `idx`. Preserves the trailing
/// newline state of the file.
fn replace_on_line(contents: &str, idx: usize, from: &str, to: &str) -> Result<String, String> {
    let mut lines: Vec<&str> = contents.lines().collect();
    let line = lines
        .get(idx)
        .ok_or_else(|| format!("line {idx} out of range"))?;
    if !line.contains(from) {
        return Err(format!("line {idx} no longer contains the placeholder"));
    }
    let replaced = line.replacen(from, to, 1);
    lines[idx] = &replaced;
    let mut out = lines.join("\n");
    if contents.ends_with('\n') {
        out.push('\n');
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// nix invocation helpers
// ---------------------------------------------------------------------------

/// A `nix` Command with the experimental-features flags prepended and the cwd
/// set to the repo root. Callers choose how to wire stdio.
fn nix_command(nix: &Path, root: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new(nix);
    cmd.args(NIX_EXPERIMENTAL).args(args).current_dir(root);
    cmd
}

/// Run `nix …`, capturing combined stdout+stderr (hash-mismatch reports go to
/// stderr). Returns (success, combined output).
fn nix_run_capture(nix: &Path, root: &Path, args: &[&str]) -> Result<(bool, String), String> {
    let out = nix_command(nix, root, args)
        .output()
        .map_err(|e| format!("failed to spawn `nix`: {e} (is Nix installed and on PATH?)"))?;
    let mut combined = String::from_utf8_lossy(&out.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&out.stderr));
    Ok((out.status.success(), combined))
}

/// Run `nix …` with inherited stdio (so long build logs stream live). Returns
/// whether it succeeded.
fn run_nix_inherit(nix: &Path, root: &Path, args: &[&str]) -> Result<bool, String> {
    let status = nix_command(nix, root, args)
        .status()
        .map_err(|e| format!("failed to spawn `nix`: {e}"))?;
    Ok(status.success())
}

/// Build `.#rust-mos`, streaming logs (stderr) live while capturing the printed
/// out-path (stdout). Returns the trimmed store path.
fn build_toolchain(nix: &Path, root: &Path) -> Result<String, String> {
    let out = nix_command(nix, root, &["build", ".#rust-mos", "--print-out-paths"])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("failed to spawn `nix`: {e}"))?
        .wait_with_output()
        .map_err(|e| format!("waiting on `nix`: {e}"))?;
    if !out.status.success() {
        return Err("`nix build .#rust-mos` failed".into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Find the LAST `got:` in nix's output and return the `sha256-…` SRI token
/// that follows it (44 base64 chars after the prefix).
fn extract_got_hash(output: &str) -> Option<String> {
    let is_b64 = |c: char| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=';
    let mut result = None;
    let mut rest = output;
    while let Some(pos) = rest.find("got:") {
        let after = rest[pos + 4..].trim_start();
        if let Some(stripped) = after.strip_prefix("sha256-") {
            let b64: String = stripped.chars().take_while(|&c| is_b64(c)).collect();
            if b64.len() == 44 && b64.ends_with('=') {
                result = Some(format!("sha256-{b64}"));
            }
        }
        rest = &rest[pos + 4..];
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const PINS: &str = "\
{\n  a = \"sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=\"; # PREFETCH:llvm-mos-source\n  b = \"sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=\"; # PREFETCH:check-vendor\n}\n";

    #[test]
    fn finds_unique_marker() {
        assert_eq!(find_marker_line(PINS, "PREFETCH:check-vendor").unwrap(), 2);
        assert!(find_marker_line(PINS, "PREFETCH:nope").is_err());
    }

    #[test]
    fn replaces_only_target_line() {
        let got = "sha256-dGhpcyBpcyBhIHRlc3QgaGFzaCwgNDQgY2hhcnMhIQ=";
        let out = replace_on_line(PINS, 2, PLACEHOLDER, got).unwrap();
        assert_eq!(out.matches(got).count(), 1);
        assert_eq!(out.matches(PLACEHOLDER).count(), 1); // line 1 untouched
        assert!(out.lines().nth(2).unwrap().contains(got));
        assert!(out.ends_with('\n'));
    }

    #[test]
    fn extracts_last_got_hash() {
        let log = "error: hash mismatch in fixed-output derivation '/nix/store/x.drv':\n\
                   \x20        specified: sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=\n\
                   \x20           got:    sha256-ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0=\n";
        assert_eq!(
            extract_got_hash(log).unwrap(),
            "sha256-ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0="
        );
        assert!(extract_got_hash("some unrelated failure").is_none());
    }

    #[test]
    fn resolves_platforms() {
        assert_eq!(
            resolve_platform("macos", "aarch64"),
            Platform::Installer("aarch64-darwin")
        );
        assert_eq!(
            resolve_platform("linux", "x86_64"),
            Platform::Installer("x86_64-linux")
        );
        assert_eq!(
            resolve_platform("linux", "aarch64"),
            Platform::Installer("aarch64-linux")
        );
        assert_eq!(resolve_platform("macos", "x86_64"), Platform::IntelMac);
        assert_eq!(resolve_platform("windows", "x86_64"), Platform::Windows);
        assert_eq!(resolve_platform("freebsd", "x86_64"), Platform::Unsupported);
    }

    #[test]
    fn every_installer_arch_has_a_checksum() {
        for (os, arch) in [("macos", "aarch64"), ("linux", "x86_64"), ("linux", "aarch64")] {
            if let Platform::Installer(triple) = resolve_platform(os, arch) {
                assert!(
                    INSTALLER_SHA256.iter().any(|(a, _)| *a == triple),
                    "no checksum pinned for {triple}"
                );
            } else {
                panic!("{os}/{arch} should map to an installer arch");
            }
        }
    }

    #[test]
    fn parses_sha256_line() {
        let hex = "82723616373d0c3f0d07b892f5f5c023da825b8969a2351c7055926d0bcf5553";
        assert_eq!(
            parse_sha256_line(&format!("{hex}  nix-installer-aarch64-darwin")).unwrap(),
            hex
        );
        // uppercase is normalized to lowercase
        assert_eq!(parse_sha256_line(&hex.to_ascii_uppercase()).unwrap(), hex);
        // too short / non-hex are rejected
        assert!(parse_sha256_line("deadbeef  file").is_none());
        assert!(parse_sha256_line("").is_none());
        assert!(parse_sha256_line(&format!("{}  file", "z".repeat(64))).is_none());
    }

    #[test]
    fn parses_init_flags() {
        assert!(!parse_init_flags(&[]).unwrap().build_only);
        assert!(parse_init_flags(&["--build".to_string()]).unwrap().build_only);
        assert!(parse_init_flags(&["--bogus".to_string()]).is_err());
    }
}
