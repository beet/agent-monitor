use std::process::Command;

use agentmon_proto::HostContext;

/// How far up the process tree to look for an identifying ancestor before
/// giving up and calling it the desktop app.
const MAX_ANCESTOR_DEPTH: usize = 12;

/// Process names (case-insensitive substring match) that indicate a real
/// terminal emulator or multiplexer hosting the session, as opposed to the
/// desktop app (which is not itself a terminal).
const KNOWN_TERMINAL_PROCESSES: &[&str] = &[
    "terminal", // Apple's Terminal.app
    "iterm2",
    "iterm",
    "alacritty",
    "kitty",
    "wezterm",
    "hyper",
    "warp",
    "zellij",
    "tmux",
    "screen",
];

/// Best-effort detection of where this hook process is running.
///
/// Claude Code does not document a way to distinguish its desktop app from a
/// terminal (see design.md's "Host-context detection" risk), and hook
/// subprocesses are invoked with piped stdio - they never have a
/// controlling tty regardless of host, so that can't be used as a signal
/// (confirmed empirically: even an interactive session's own shell reports
/// no tty when queried this way). Instead this walks the process ancestry
/// looking for nvim or a known terminal emulator; if neither appears, it's
/// assumed to be the desktop app. Being wrong here only affects a display
/// label, never status tracking or notifications.
pub fn detect_host_context() -> HostContext {
    classify_ancestors(std::process::id()).unwrap_or(HostContext::Desktop)
}

fn classify_ancestors(start_pid: u32) -> Option<HostContext> {
    let mut pid = start_pid;
    for _ in 0..MAX_ANCESTOR_DEPTH {
        let (ppid, comm) = ps_ppid_comm(pid)?;
        if contains_nvim(&comm) {
            return Some(HostContext::Nvim);
        }
        if is_known_terminal_process(&comm) {
            return Some(HostContext::Terminal);
        }
        if ppid == 0 || ppid == 1 || ppid == pid {
            return None;
        }
        pid = ppid;
    }
    None
}

/// The pid of the process that invoked this hook command - i.e. the actual
/// long-running Claude Code session, not this short-lived reporter. This is
/// the pid the daemon's liveness sweep should track.
pub fn parent_pid() -> Option<u32> {
    ps_ppid_comm(std::process::id()).map(|(ppid, _)| ppid)
}

fn ps_ppid_comm(pid: u32) -> Option<(u32, String)> {
    let output = Command::new("ps")
        .args(["-o", "ppid=,comm=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_ppid_comm(&String::from_utf8_lossy(&output.stdout))
}

fn contains_nvim(process_name: &str) -> bool {
    process_name.to_lowercase().contains("nvim")
}

fn is_known_terminal_process(process_name: &str) -> bool {
    let lower = process_name.to_lowercase();
    KNOWN_TERMINAL_PROCESSES.iter().any(|name| lower.contains(name))
}

/// Parses a line of `ps -o ppid=,comm=` output. `comm` may itself contain
/// spaces (e.g. a macOS app bundle path), so everything after the leading
/// ppid field is taken as the name.
fn parse_ppid_comm(line: &str) -> Option<(u32, String)> {
    let trimmed = line.trim();
    let (ppid_str, rest) = trimmed.split_once(char::is_whitespace)?;
    let ppid: u32 = ppid_str.parse().ok()?;
    let comm = rest.trim();
    if comm.is_empty() {
        return None;
    }
    Some((ppid, comm.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ppid_comm_handles_a_simple_command_name() {
        assert_eq!(
            parse_ppid_comm("  1234 zsh\n"),
            Some((1234, "zsh".to_string()))
        );
    }

    #[test]
    fn parse_ppid_comm_handles_a_path_with_spaces() {
        assert_eq!(
            parse_ppid_comm("42 /Applications/Claude Code.app/Contents/MacOS/Claude Code\n"),
            Some((42, "/Applications/Claude Code.app/Contents/MacOS/Claude Code".to_string()))
        );
    }

    #[test]
    fn parse_ppid_comm_rejects_malformed_input() {
        assert_eq!(parse_ppid_comm(""), None);
        assert_eq!(parse_ppid_comm("not-a-number zsh"), None);
        assert_eq!(parse_ppid_comm("1234"), None);
    }

    #[test]
    fn contains_nvim_is_case_insensitive() {
        assert!(contains_nvim("nvim"));
        assert!(contains_nvim("NVIM"));
        assert!(contains_nvim("/opt/homebrew/bin/nvim"));
        assert!(!contains_nvim("vim"));
        assert!(!contains_nvim("zsh"));
    }

    #[test]
    fn is_known_terminal_process_matches_common_emulators_and_multiplexers() {
        assert!(is_known_terminal_process("Terminal"));
        assert!(is_known_terminal_process("iTerm2"));
        assert!(is_known_terminal_process("/usr/local/bin/zellij"));
        assert!(is_known_terminal_process("tmux"));
        assert!(!is_known_terminal_process("zsh"));
        assert!(!is_known_terminal_process("nvim"));
    }

    #[test]
    fn detect_host_context_and_parent_pid_do_not_panic_on_a_real_process() {
        // Sanity check against our own real process tree; the exact
        // classification depends on how the test runner was launched, so
        // this only asserts the calls succeed and return something sane.
        let _ = detect_host_context();
        let parent = parent_pid();
        assert!(parent.is_some());
        assert_ne!(parent, Some(std::process::id()));
    }
}
