//! Black-box tests that drive the built binary.
//!
//! The crate is binary-only, so `tests/` cannot import its modules -- but Cargo still exports
//! `CARGO_BIN_EXE_agy-statusline`, which is enough to test the one layer the in-crate unit tests
//! structurally cannot reach: `main`'s own wiring.
//!
//! That layer is where the home directory is resolved. Every function below `main` takes
//! `home_path` as a parameter and is tested by injection, which is the right design and the reason
//! those tests are hermetic -- but it also means the `USERPROFILE` -> `HOME` lookup at the top had
//! no coverage at all. Env vars are process-global and Rust runs tests on threads, so this is the
//! only place it *can* be tested honestly: a child process gets an environment of its own.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const EXE: &str = env!("CARGO_BIN_EXE_agy-statusline");

/// Run the binary in `--print-title` mode with a controlled environment, returning the parsed
/// JSON report. `env` entries with a `None` value are removed from the child's environment.
fn run(payload: &str, env: &[(&str, Option<&str>)]) -> serde_json::Value {
    let mut cmd = Command::new(EXE);
    cmd.arg("--print-title")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    for (k, v) in env {
        match v {
            Some(val) => cmd.env(k, val),
            None => cmd.env_remove(k),
        };
    }

    let mut child = cmd.spawn().expect("failed to spawn statusline binary");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(payload.as_bytes())
        .unwrap();

    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "binary exited with {:?}", out.status);

    let text = String::from_utf8(out.stdout).expect("output was not utf-8");
    serde_json::from_str(text.trim()).unwrap_or_else(|e| panic!("bad JSON {:?}: {}", text, e))
}

/// A payload whose `cwd` is the home directory itself. `get_workspace_name` collapses that to `~`,
/// but only if `main` resolved the same directory as home -- so the tilde is a precise assertion
/// that the env lookup produced the value under test, and not merely a non-empty string.
fn payload_at(home: &str) -> String {
    serde_json::json!({ "cwd": home }).to_string()
}

/// A directory to stand in for the user's home. Derived from the platform's own temp location so
/// nothing here assumes a drive letter, a path separator, or that `C:\Users` is where homes live.
fn temp_dir_named(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("agy-cli-{}-{}", label, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn temp_home() -> PathBuf {
    temp_dir_named("home")
}

#[test]
fn home_comes_from_userprofile() {
    let home = temp_home();
    let home = home.to_str().unwrap();

    let r = run(&payload_at(home), &[("USERPROFILE", Some(home)), ("HOME", None)]);
    assert_eq!(r["workspace"], "~", "USERPROFILE was not used as the home directory");
}

/// The `or_else` fallback in `main`. Windows sets `USERPROFILE`, but the binary also runs under
/// shells that export only `HOME` (git-bash, MSYS), where losing this branch would silently stop
/// every `~` collapsing and every config load.
#[test]
fn home_falls_back_to_home_var() {
    let home = temp_home();
    let home = home.to_str().unwrap();

    let r = run(&payload_at(home), &[("USERPROFILE", None), ("HOME", Some(home))]);
    assert_eq!(r["workspace"], "~", "HOME fallback did not resolve the home directory");
}

#[test]
fn userprofile_wins_over_home() {
    let home = temp_home();
    let home = home.to_str().unwrap();

    let other = temp_dir_named("not-home");
    let r = run(
        &payload_at(home),
        &[
            ("USERPROFILE", Some(home)),
            ("HOME", Some(other.to_str().unwrap())),
        ],
    );
    assert_eq!(r["workspace"], "~", "HOME took precedence over USERPROFILE");
}

/// With neither set, `unwrap_or_default()` yields an empty home. Nothing may panic: the statusline
/// runs on every prompt refresh, and a crash here costs the user their prompt.
#[test]
fn missing_home_vars_do_not_panic() {
    let cwd = temp_dir_named("home").join("project");
    let r = run(
        &payload_at(cwd.to_str().unwrap()),
        &[("USERPROFILE", None), ("HOME", None)],
    );
    assert_eq!(r["workspace"], "project");
}

/// `main` parses stdin with `unwrap_or_default()`, so garbage must degrade to a default statusline
/// rather than an error. This is the contract with the CLI: it pipes whatever it has.
#[test]
fn malformed_stdin_still_reports() {
    for payload in ["", "not json at all", "{\"cwd\":", "null", "[]"] {
        let r = run(payload, &[]);
        assert!(
            r["icon"].as_str().is_some_and(|s| !s.is_empty()),
            "no icon for payload {:?}",
            payload
        );
    }
}
