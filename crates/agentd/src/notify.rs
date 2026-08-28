use std::process::Command;

use agentmon_proto::{AgentInfo, AgentStatus, HostContext};

/// Delivers a user-facing notification for an agent that needs attention.
pub trait Notifier: Send + Sync {
    fn notify(&self, agent: &AgentInfo);
}

/// Sends a macOS notification banner via `osascript`.
pub struct OsaScriptNotifier;

impl Notifier for OsaScriptNotifier {
    fn notify(&self, agent: &AgentInfo) {
        let Some(status_label) = status_label(agent.status) else {
            return;
        };
        let project = agent
            .cwd
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| agent.cwd.display().to_string());
        let body = format!(
            "{project} ({}) {status_label}",
            host_context_label(agent.host_context)
        );
        let script = format!(
            "display notification {} with title {}",
            applescript_literal(&body),
            applescript_literal("agentmon")
        );

        if let Err(err) = Command::new("osascript").arg("-e").arg(script).status() {
            eprintln!("agentd: failed to send notification: {err}");
        }
    }
}

fn status_label(status: AgentStatus) -> Option<&'static str> {
    match status {
        AgentStatus::Done => Some("finished"),
        AgentStatus::NeedsInput => Some("needs input"),
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
    use super::*;

    #[test]
    fn applescript_literal_escapes_quotes_and_backslashes() {
        assert_eq!(
            applescript_literal(r#"say "hi" \ bye"#),
            r#""say \"hi\" \\ bye""#
        );
    }

    #[test]
    fn status_label_only_covers_attention_states() {
        assert_eq!(status_label(AgentStatus::Done), Some("finished"));
        assert_eq!(status_label(AgentStatus::NeedsInput), Some("needs input"));
        assert_eq!(status_label(AgentStatus::Running), None);
        assert_eq!(status_label(AgentStatus::Idle), None);
        assert_eq!(status_label(AgentStatus::Stale), None);
    }
}
