//! `nixdev` — enter the toolchain dev shell, in the shell you invoked it from.
//!
//! `nix develop` searches upward for a `flake.nix` but stops at the enclosing
//! git repository, so it cannot be run from a `bin/` project: each is its own
//! repo, and the flake lives in the workspace above them. This task names the
//! flake explicitly (the repo root, resolved from the xtask manifest, never from
//! the cwd) so entering the shell works the same from anywhere.
//!
//! The shell defaults to the one that invoked the task, discovered by walking
//! the process tree rather than reading `$SHELL` — `$SHELL` is the *login*
//! shell, which is wrong whenever you are sitting in a different one.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

pub const HELP: &str = "\
cargo xnixdev [SHELL] [-- NIX ARGS…]    (also: cargo xtask nixdev)

Enter the rust-mos toolchain dev shell (`nix develop`) using the shell you ran
this from. The flake is named explicitly, so this works from a bin/ project as
well as from the repo root — plain `nix develop` does not, because it stops
searching at the project's own git repository.

SHELL       shell to launch; defaults to the invoking shell, else $SHELL,
            else bash.
-- ARGS…    everything after `--` is passed through to `nix develop`.
";

/// Process names we accept as \"a shell\" when walking up from this process.
const SHELLS: &[&str] =
    &["bash", "zsh", "fish", "sh", "dash", "ksh", "mksh", "tcsh", "csh", "nu", "elvish", "xonsh"];

pub fn run(args: &[String]) -> ExitCode {
    match run_inner(args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("ERROR [nixdev]: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run_inner(args: &[String]) -> Result<ExitCode, String> {
    // Split at `--`: before it an optional shell name, after it nix passthrough.
    let split = args.iter().position(|a| a == "--");
    let (head, nix_args) = match split {
        Some(i) => (&args[..i], &args[i + 1..]),
        None => (args, &args[args.len()..]),
    };
    if head.len() > 1 {
        return Err(format!("expected at most one SHELL argument, got {}", head.len()));
    }

    let shell = match head.first() {
        Some(s) => s.clone(),
        None => invoking_shell(),
    };

    if std::env::var("XTASK_DEVSHELL_STAMP").is_ok() {
        eprintln!("note: already inside a dev shell; entering a nested one");
    }

    let nix = crate::find_nix().ok_or(
        "cannot find `nix` on PATH or in the default profile; run `cargo xtask initenv` first",
    )?;
    let root = crate::repo_root();

    // `nix develop <root>` rather than `.`: the cwd may be a bin/ project, whose
    // own git repo would otherwise bound the flake search.
    let root_arg = root.to_string_lossy().into_owned();
    let mut argv: Vec<&str> = vec!["develop", &root_arg, "-c", &shell];
    argv.extend(nix_args.iter().map(String::as_str));

    let status = crate::nix_command(&nix, Path::new("."), &argv)
        .status()
        .map_err(|e| format!("failed to spawn `nix`: {e}"))?;

    // Propagate the shell's own exit status: this task is a passthrough.
    Ok(match status.success() {
        true => ExitCode::SUCCESS,
        false => ExitCode::FAILURE,
    })
}

/// The nearest shell above this process, or `$SHELL`, or bash.
///
/// The chain is xtask -> cargo -> shell, but `cargo run` layers vary, so walk
/// until a known shell turns up rather than assuming a fixed depth.
fn invoking_shell() -> String {
    let mut pid = std::process::id().to_string();
    for _ in 0..8 {
        let Some(ppid) = ps_field("ppid=", &pid) else { break };
        if ppid == "0" || ppid == "1" {
            break;
        }
        if let Some(name) = ps_field("comm=", &ppid) {
            // Login shells appear as `-bash`; comm may be a full path.
            let base = Path::new(name.trim_start_matches('-'))
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or(name.clone());
            let base = base.trim_start_matches('-').to_string();
            if SHELLS.contains(&base.as_str()) {
                return base;
            }
        }
        pid = ppid;
    }
    std::env::var("SHELL")
        .ok()
        .and_then(|s| PathBuf::from(s).file_name().map(|f| f.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "bash".to_string())
}

/// One `ps -o <field> -p <pid>` value, trimmed; `None` if ps fails or is empty.
fn ps_field(field: &str, pid: &str) -> Option<String> {
    let out = Command::new("ps").args(["-o", field, "-p", pid]).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!s.is_empty()).then_some(s)
}
