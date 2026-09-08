use std::path::Path;
use std::process::Command;

use agentmon_proto::{AgentInfo, AgentStatus, HostContext};

/// Delivers a user-facing notification for an agent that needs attention.
pub trait Notifier: Send + Sync {
    fn notify(&self, agent: &AgentInfo);

    /// Notifies about a failing test run whose working directory has no
    /// tracked agent - the only case where the daemon itself, rather than a
    /// tracked agent's row in a connected client, is what surfaces the
    /// failure to the user. Defaults to a no-op so existing implementors
    /// (e.g. test doubles that don't care about test runs) don't need to
    /// change.
    fn notify_test_run_failure(&self, _cwd: &Path) {}
}

/// Sends a macOS notification banner via `osascript`.
pub struct OsaScriptNotifier;

impl Notifier for OsaScriptNotifier {
    fn notify(&self, agent: &AgentInfo) {
        let Some(script) = notification_script(agent) else {
            return;
        };

        if let Err(err) = Command::new("osascript").arg("-e").arg(script).status() {
            eprintln!("agentd: failed to send notification: {err}");
        }
    }

    fn notify_test_run_failure(&self, cwd: &Path) {
        let script = test_run_failure_script(cwd);
        if let Err(err) = Command::new("osascript").arg("-e").arg(script).status() {
            eprintln!("agentd: failed to send test-run notification: {err}");
        }
    }
}

/// Builds the `osascript` AppleScript for a test-run failure with no tracked
/// agent, played with the `Basso` sound - distinct from the `Glass`/`Ping`
/// sounds used for agent completion notifications, so it's not confused with
/// either by ear.
fn test_run_failure_script(cwd: &Path) -> String {
    let project = cwd
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| cwd.display().to_string());
    format!(
        "display notification {} with title {} sound name {}",
        applescript_literal(&format!("{project} tests failed")),
        applescript_literal("agentmon"),
        applescript_literal("Basso")
    )
}

/// Builds the `osascript` AppleScript for a notification, or `None` if
/// `agent`'s status doesn't warrant one.
fn notification_script(agent: &AgentInfo) -> Option<String> {
    let (status_label, sound_name) = status_notification(agent.status)?;
    let project = agent
        .cwd
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| agent.cwd.display().to_string());
    let body = format!(
        "{project} ({}) {status_label}",
        host_context_label(agent.host_context)
    );
    Some(format!(
        "display notification {} with title {} sound name {}",
        applescript_literal(&body),
        applescript_literal("agentmon"),
        applescript_literal(sound_name)
    ))
}

/// Notification text and built-in macOS sound for statuses that warrant a
/// notification. `None` for statuses that should not notify at all.
fn status_notification(status: AgentStatus) -> Option<(&'static str, &'static str)> {
    match status {
        AgentStatus::Done => Some(("finished", "Glass")),
        AgentStatus::NeedsInput => Some(("needs input", "Ping")),
        _ => None,
    }
}

fn host_context_label(context: HostContext) -> &'static str {
    match context {
        HostContext::Nvim => "nvim",
        HostContext::Terminal => "terminal",
        HostContext::Desktop => "desktop",
    }
}

/// Escapes `value` for safe interpolation into an AppleScript string literal.
fn applescript_literal(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use agentmon_proto::SessionId;

    use super::*;

    fn sample_agent(status: AgentStatus) -> AgentInfo {
        AgentInfo {
            session_id: SessionId("session-123".to_string()),
            cwd: PathBuf::from("/Users/beet/Documents/Projects/enclaudinate"),
            host_context: HostContext::Terminal,
            pid: 4242,
            status,
            last_updated_ms: 1_700_000_000_000,
            status_since_ms: 1_700_000_000_000,
        }
    }

    #[test]
    fn notification_script_uses_glass_sound_when_done() {
        let script = notification_script(&sample_agent(AgentStatus::Done)).unwrap();
        assert!(
            script.ends_with("sound name \"Glass\""),
            "script did not end with the Glass sound clause: {script}"
        );
    }

    #[test]
    fn notification_script_uses_ping_sound_when_needs_input() {
        let script = notification_script(&sample_agent(AgentStatus::NeedsInput)).unwrap();
        assert!(
            script.ends_with("sound name \"Ping\""),
            "script did not end with the Ping sound clause: {script}"
        );
    }

    #[test]
    fn notification_script_is_none_for_non_attention_states() {
        assert!(notification_script(&sample_agent(AgentStatus::Running)).is_none());
        assert!(notification_script(&sample_agent(AgentStatus::Idle)).is_none());
        assert!(notification_script(&sample_agent(AgentStatus::Stale)).is_none());
        assert!(notification_script(&sample_agent(AgentStatus::Declined)).is_none());
    }

    #[test]
    fn test_run_failure_script_uses_basso_sound() {
        let script = test_run_failure_script(&PathBuf::from("/tmp/project"));
        assert!(
            script.ends_with("sound name \"Basso\""),
            "script did not end with the Basso sound clause: {script}"
        );
    }

    #[test]
    fn test_run_failure_script_identifies_the_directory() {
        let script = test_run_failure_script(&PathBuf::from("/tmp/project"));
        assert!(
            script.contains("project"),
            "script did not mention the project directory: {script}"
        );
    }

    #[test]
    fn applescript_literal_escapes_quotes_and_backslashes() {
        assert_eq!(
            applescript_literal(r#"say "hi" \ bye"#),
            r#""say \"hi\" \\ bye""#
        );
    }

    #[test]
    fn status_notification_only_covers_attention_states() {
        assert_eq!(
            status_notification(AgentStatus::Done),
            Some(("finished", "Glass"))
        );
        assert_eq!(
            status_notification(AgentStatus::NeedsInput),
            Some(("needs input", "Ping"))
        );
        assert_eq!(status_notification(AgentStatus::Running), None);
        assert_eq!(status_notification(AgentStatus::Idle), None);
        assert_eq!(status_notification(AgentStatus::Stale), None);
        assert_eq!(status_notification(AgentStatus::Declined), None);
    }
}
