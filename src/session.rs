//! Which agy session owns the console the statusline may write to.
//!
//! The statusline writes its terminal-title escape sequence straight to the console
//! buffer (`CONOUT$`), which is *shared by every process attached to that console*. A
//! child `agy -p` inherits its parent's console, so its statusline refreshes land on the
//! parent's tab title — an eval run spawning a dozen headless sessions overwrites the
//! interactive session's title a dozen times a second.
//!
//! Stdout is not affected: each session's statusline text goes back to whoever invoked it.
//! Only the console write crosses sessions, so only it is gated here.
//!
//! Ownership is decided from the invoking process chain: a headless session (`-p` /
//! `--print` / `--prompt`) has no tab of its own, so anything it paints lands on the tab
//! of whoever started it. The whole chain is checked, not just the nearest CLI, so a
//! session spawned *by* a headless session stays silent too.
//!
//! Counting agy processes would be the obvious shortcut and is wrong: a CLI that forks or
//! re-execs itself appears twice in the chain for a single session, which would silence a
//! perfectly ordinary interactive session. Only the print-mode flag distinguishes a
//! session that owns a console from one that borrows it.

use crate::procinfo;

/// How far up the process tree to look for the invoking agy CLI. The statusline may be
/// launched through a shell wrapper, so the CLI is not always the immediate parent.
const MAX_DEPTH: usize = 12;

/// Executable stem of the Antigravity CLI.
const AGY_EXE_STEM: &str = "agy";

/// Flags that put the CLI in non-interactive print mode. `--prompt-interactive` (and its
/// `-i` alias) keeps a real session on the console and is deliberately absent.
const HEADLESS_FLAGS: [&str; 3] = ["p", "print", "prompt"];

/// True when the command line belongs to the Antigravity CLI itself.
fn is_agy_cli(cmdline: &str) -> bool {
    procinfo::executable_stem(cmdline).as_deref() == Some(AGY_EXE_STEM)
}

/// True when an agy command line runs a single prompt non-interactively.
///
/// Go's flag package accepts `-print`, `--print`, and `--print=value` alike, so the
/// argument is normalised to its bare flag name before matching. Matching whole names
/// keeps `--prompt-interactive` out.
fn is_headless(cmdline: &str) -> bool {
    procinfo::split_args(cmdline).iter().skip(1).any(|arg| {
        let name = arg.trim_start_matches('-');
        if name.len() == arg.len() {
            return false; // a bare word, not a flag
        }
        let name = name.split('=').next().unwrap_or(name);
        HEADLESS_FLAGS.contains(&name)
    })
}

/// True when this statusline invocation is allowed to repaint the terminal title.
///
/// Falls back to `true` whenever no headless agy ancestor is found — a manual run, a
/// non-Windows build, or a chain link that could not be opened. That keeps the previous
/// behaviour for every case the walk cannot speak to, and only suppresses a write when a
/// borrowed console is positively identified.
pub(crate) fn owns_terminal_title() -> bool {
    // Index 0 is the statusline itself; every entry above it is a potential invoker.
    !procinfo::ancestry(MAX_DEPTH)
        .iter()
        .skip(1)
        .filter_map(|e| e.cmdline.as_deref())
        .any(|c| is_agy_cli(c) && is_headless(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    const AGY: &str = r#""C:\Users\u\AppData\Local\agy\bin\agy.exe""#;

    #[test]
    fn test_is_agy_cli() {
        assert!(is_agy_cli(AGY));
        assert!(is_agy_cli("agy --continue"));
        assert!(is_agy_cli(r"C:\bin\AGY.EXE -p hi"));
        assert!(!is_agy_cli("pwsh -NoProfile -File statusline.ps1"));
        assert!(!is_agy_cli(r#""C:\agy\bin\agy-statusline.exe""#));
        assert!(!is_agy_cli(""));
    }

    #[test]
    fn test_is_headless() {
        assert!(is_headless(&format!("{} -p \"run the eval\"", AGY)));
        assert!(is_headless(&format!("{} --print hi", AGY)));
        assert!(is_headless(&format!("{} --prompt hi", AGY)));
        assert!(is_headless(&format!("{} --print=hi", AGY)));
        assert!(is_headless(&format!("{} --model gemini -p hi", AGY)));
    }

    /// `--prompt-interactive` and `-i` keep a real session on the console.
    #[test]
    fn test_interactive_flags_are_not_headless() {
        assert!(!is_headless(&format!("{} --prompt-interactive \"hi\"", AGY)));
        assert!(!is_headless(&format!("{} -i \"hi\"", AGY)));
        assert!(!is_headless(&format!("{} --continue", AGY)));
        assert!(!is_headless(AGY));
    }

    /// A bare positional argument that happens to read like a flag name must not count.
    #[test]
    fn test_positional_argument_is_not_a_flag() {
        assert!(!is_headless(&format!("{} --continue print", AGY)));
        assert!(!is_headless(&format!("{} -i \"use --print later\"", AGY)));
    }

    /// The decision, over a synthesised ancestor chain so it is testable off-console.
    /// Mirrors `owns_terminal_title` with the process walk replaced by fixed cmdlines.
    fn owns(ancestors: &[&str]) -> bool {
        !ancestors.iter().any(|c| is_agy_cli(c) && is_headless(c))
    }

    #[test]
    fn test_ownership_decision() {
        // A lone interactive session owns its title.
        assert!(owns(&[AGY]));
        assert!(owns(&[&format!("{} --continue", AGY)]));

        // No agy ancestor identified: keep painting, as before.
        assert!(owns(&[]));
        assert!(owns(&["pwsh -NoProfile"]));

        // A headless eval child must never touch the console.
        assert!(!owns(&[&format!("{} -p \"eval case 3\"", AGY)]));

        // The eval parent still owns its own title while its children run.
        assert!(owns(&[AGY, "pwsh -NoProfile"]));

        // Headless child of an interactive parent: the parent's tab is not the child's.
        assert!(!owns(&[&format!("{} -p \"eval case 3\"", AGY), AGY]));

        // A session spawned by a headless one is just as far from owning the console.
        assert!(!owns(&[AGY, &format!("{} -p \"eval case 3\"", AGY)]));
    }

    /// A CLI that forks or re-execs itself shows up twice for one session; that must not
    /// be read as nesting and silence an ordinary interactive session.
    #[test]
    fn test_self_fork_still_owns_title() {
        assert!(owns(&[AGY, AGY]));
        assert!(owns(&[
            &format!("{} --continue", AGY),
            &format!("{} --continue", AGY),
        ]));
    }

    /// Only the agy CLI's own flags count -- an unrelated `-p` upstream is not ours.
    #[test]
    fn test_foreign_print_flag_is_ignored() {
        assert!(owns(&["pwsh -p", AGY]));
        assert!(owns(&[AGY, "git log -p"]));
    }
}
