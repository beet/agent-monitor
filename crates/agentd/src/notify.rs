use std::process::Command;

use agentmon_proto::{AgentInfo, AgentStatus, HostContext, TestRunStatus};

/// Delivers a user-facing notification for an agent that needs attention.
pub trait Notifier: Send + Sync {
    fn notify(&self, agent: &AgentInfo, project_name: &str);

    /// Notifies about a test-run event ("started", "passed", or "failed"),
    /// independent of whether its working directory has a tracked agent - a
    /// long agent session can start/stop tests many times before it
    /// finishes, and each event is heard on its own. Defaults to a no-op so
    /// existing implementors (e.g. test doubles that don't care about test
    /// runs) don't need to change.
    fn notify_test_run(&self, _project_name: &str, _status: TestRunStatus) {}

    /// Notifies that a reminder completed on its own (its elapsed time
    /// reached its duration) - never sent for a manual stop. Defaults to a
    /// no-op, matching `notify_test_run`.
    fn notify_reminder(&self, _project_name: &str, _name: &str) {}
}

/// Sends a macOS notification banner via `osascript`.
pub struct OsaScriptNotifier;

impl Notifier for OsaScriptNotifier {
    fn notify(&self, agent: &AgentInfo, project_name: &str) {
        let Some(script) = notification_script(agent, project_name) else {
            return;
        };

        if let Err(err) = Command::new("osascript").arg("-e").arg(script).status() {
            eprintln!("agentd: failed to send notification: {err}");
        }
    }

    fn notify_test_run(&self, project_name: &str, status: TestRunStatus) {
        let script = test_run_notification_script(project_name, status);
        if let Err(err) = Command::new("osascript").arg("-e").arg(script).status() {
            eprintln!("agentd: failed to send test-run notification: {err}");
        }
    }

    fn notify_reminder(&self, project_name: &str, name: &str) {
        let script = reminder_notification_script(project_name, name);
        if let Err(err) = Command::new("osascript").arg("-e").arg(script).status() {
            eprintln!("agentd: failed to send reminder notification: {err}");
        }
    }
}

/// Builds the `osascript` AppleScript for a reminder's natural completion.
/// `Blow` is distinct from `Glass`/`Ping` (agent) and `Pop`/`Tink`/`Basso`
/// (test-run), so a reminder notification is never confused with either by
/// ear.
fn reminder_notification_script(project_name: &str, name: &str) -> String {
    format!(
        "display notification {} with title {} sound name {}",
        applescript_literal(&format!("⏰ {project_name}: {name}")),
        applescript_literal("agentmon"),
        applescript_literal("Blow")
    )
}

/// Builds the `osascript` AppleScript for a test-run event. `Pop` (started),
/// `Tink` (passed), and `Basso` (failed) are each distinct from one another
/// and from the `Glass`/`Ping` sounds used for agent completion
/// notifications, so a test-run event is never confused with a different
/// event or with an agent notification by ear.
fn test_run_notification_script(project_name: &str, status: TestRunStatus) -> String {
    let (emoji, status_label, sound_name) = match status {
        TestRunStatus::Running => ("🧪", "tests started", "Pop"),
        TestRunStatus::Passed => ("✅", "tests passed", "Tink"),
        TestRunStatus::Failed => ("❌", "tests failed", "Basso"),
    };
    format!(
        "display notification {} with title {} sound name {}",
        applescript_literal(&format!("{emoji} {project_name} {status_label}")),
        applescript_literal("agentmon"),
        applescript_literal(sound_name)
    )
}

/// Builds the `osascript` AppleScript for a notification, or `None` if
/// `agent`'s status doesn't warrant one.
fn notification_script(agent: &AgentInfo, project_name: &str) -> Option<String> {
    let (status_label, sound_name) = status_notification(agent.status)?;
    let body = format!(
        "🤖 {project_name} ({}) {status_label}",
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
            run_started_ms: 1_700_000_000_000,
            last_prompt: None,
        }
    }

    #[test]
    fn notification_script_uses_glass_sound_when_done() {
        let script = notification_script(&sample_agent(AgentStatus::Done), "enclaudinate").unwrap();
        assert!(
            script.ends_with("sound name \"Glass\""),
            "script did not end with the Glass sound clause: {script}"
        );
    }

    #[test]
    fn notification_script_uses_ping_sound_when_needs_input() {
        let script = notification_script(&sample_agent(AgentStatus::NeedsInput), "enclaudinate").unwrap();
        assert!(
            script.ends_with("sound name \"Ping\""),
            "script did not end with the Ping sound clause: {script}"
        );
    }

    #[test]
    fn notification_script_body_is_prefixed_with_the_robot_emoji_when_done() {
        let script = notification_script(&sample_agent(AgentStatus::Done), "enclaudinate").unwrap();
        assert!(
            script.contains("\"🤖 enclaudinate"),
            "script's body did not start with the 🤖 prefix: {script}"
        );
    }

    #[test]
    fn notification_script_body_is_prefixed_with_the_robot_emoji_when_needs_input() {
        let script = notification_script(&sample_agent(AgentStatus::NeedsInput), "enclaudinate").unwrap();
        assert!(
            script.contains("\"🤖 enclaudinate"),
            "script's body did not start with the 🤖 prefix: {script}"
        );
    }

    #[test]
    fn notification_script_uses_the_given_project_name_rather_than_the_agent_cwd() {
        let agent = sample_agent(AgentStatus::Done);
        let script = notification_script(&agent, "renamed-project").unwrap();
        assert!(
            script.contains("renamed-project"),
            "script did not contain the custom project name: {script}"
        );
        assert!(
            !script.contains("enclaudinate"),
            "script unexpectedly contained the raw cwd directory name: {script}"
        );
    }

    #[test]
    fn notification_script_is_none_for_non_attention_states() {
        assert!(notification_script(&sample_agent(AgentStatus::Running), "enclaudinate").is_none());
        assert!(notification_script(&sample_agent(AgentStatus::Idle), "enclaudinate").is_none());
        assert!(notification_script(&sample_agent(AgentStatus::Stale), "enclaudinate").is_none());
        assert!(notification_script(&sample_agent(AgentStatus::Declined), "enclaudinate").is_none());
    }

    #[test]
    fn test_run_notification_script_uses_basso_sound_when_failed() {
        let script = test_run_notification_script("project", TestRunStatus::Failed);
        assert!(
            script.ends_with("sound name \"Basso\""),
            "script did not end with the Basso sound clause: {script}"
        );
    }

    #[test]
    fn test_run_notification_script_uses_tink_sound_when_passed() {
        let script = test_run_notification_script("project", TestRunStatus::Passed);
        assert!(
            script.ends_with("sound name \"Tink\""),
            "script did not end with the Tink sound clause: {script}"
        );
    }

    #[test]
    fn test_run_notification_script_uses_pop_sound_when_started() {
        let script = test_run_notification_script("project", TestRunStatus::Running);
        assert!(
            script.ends_with("sound name \"Pop\""),
            "script did not end with the Pop sound clause: {script}"
        );
    }

    #[test]
    fn test_run_notification_script_body_is_prefixed_with_the_test_tube_emoji_when_started() {
        let script = test_run_notification_script("project", TestRunStatus::Running);
        assert!(
            script.contains("\"🧪 project"),
            "script's body did not start with the 🧪 prefix: {script}"
        );
    }

    #[test]
    fn test_run_notification_script_body_is_prefixed_with_a_checkmark_when_passed() {
        let script = test_run_notification_script("project", TestRunStatus::Passed);
        assert!(
            script.contains("\"✅ project"),
            "script's body did not start with the ✅ prefix: {script}"
        );
    }

    #[test]
    fn test_run_notification_script_body_is_prefixed_with_a_cross_mark_when_failed() {
        let script = test_run_notification_script("project", TestRunStatus::Failed);
        assert!(
            script.contains("\"❌ project"),
            "script's body did not start with the ❌ prefix: {script}"
        );
    }

    #[test]
    fn test_run_notification_script_identifies_the_project() {
        let script = test_run_notification_script("project", TestRunStatus::Failed);
        assert!(
            script.contains("project"),
            "script did not mention the project name: {script}"
        );
    }

    #[test]
    fn test_run_notification_script_uses_the_given_project_name_rather_than_a_raw_cwd() {
        let script = test_run_notification_script("renamed-project", TestRunStatus::Passed);
        assert!(
            script.contains("renamed-project"),
            "script did not contain the custom project name: {script}"
        );
    }

    #[test]
    fn reminder_notification_script_uses_blow_sound() {
        let script = reminder_notification_script("project", "Check the build");
        assert!(
            script.ends_with("sound name \"Blow\""),
            "script did not end with the Blow sound clause: {script}"
        );
    }

    #[test]
    fn reminder_notification_script_body_is_prefixed_with_the_clock_emoji() {
        let script = reminder_notification_script("project", "Check the build");
        assert!(
            script.contains("\"⏰ project"),
            "script's body did not start with the ⏰ prefix: {script}"
        );
    }

    #[test]
    fn reminder_notification_script_identifies_the_project_and_name() {
        let script = reminder_notification_script("project", "Check the build");
        assert!(script.contains("project"), "script did not mention the project: {script}");
        assert!(
            script.contains("Check the build"),
            "script did not mention the reminder name: {script}"
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
