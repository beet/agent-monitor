use agentmon_proto::AgentStatus;
use serde::Deserialize;

/// The fields we care about from a Claude Code hook's JSON payload.
///
/// Different hook events include different fields; only `session_id`,
/// `cwd`, and `hook_event_name` are common to all of them. `notification_type`
/// is only present on `Notification` events.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct HookPayload {
    pub session_id: String,
    pub cwd: String,
    pub hook_event_name: String,
    #[serde(default)]
    pub notification_type: Option<String>,
}

pub fn parse_hook_payload(raw: &str) -> serde_json::Result<HookPayload> {
    serde_json::from_str(raw)
}

/// Notification_type values (per Claude Code's hooks reference) that mean
/// the agent is waiting on the user. The `Notification` hook is registered
/// with a matcher restricting it to exactly these types (see
/// `install_hooks::NEEDS_INPUT_MATCHER`), but this is checked again here so
/// a hand-edited settings.json can't cause a wrong status.
const NEEDS_INPUT_NOTIFICATION_TYPES: &[&str] = &[
    "permission_prompt",
    "idle_prompt",
    "elicitation_dialog",
    "elicitation_url_dialog",
    "agent_needs_input",
];

/// Maps a hook payload to the status it reports, or `None` if this event
/// doesn't correspond to a tracked status change (e.g. a `Notification`
/// whose type isn't one of the "needs input" kinds).
pub fn status_for_payload(payload: &HookPayload) -> Option<AgentStatus> {
    match payload.hook_event_name.as_str() {
        "UserPromptSubmit" => Some(AgentStatus::Running),
        "PreToolUse" => Some(AgentStatus::Running),
        "Stop" => Some(AgentStatus::Done),
        "Notification" => {
            let notification_type = payload.notification_type.as_deref()?;
            NEEDS_INPUT_NOTIFICATION_TYPES
                .contains(&notification_type)
                .then_some(AgentStatus::NeedsInput)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_user_prompt_submit_payload() {
        let raw = r#"{
            "session_id": "abc-123",
            "cwd": "/tmp/project",
            "hook_event_name": "UserPromptSubmit"
        }"#;

        let payload = parse_hook_payload(raw).unwrap();

        assert_eq!(payload.session_id, "abc-123");
        assert_eq!(payload.cwd, "/tmp/project");
        assert_eq!(status_for_payload(&payload), Some(AgentStatus::Running));
    }

    #[test]
    fn parses_a_pre_tool_use_payload_as_running() {
        let raw = r#"{
            "session_id": "abc-123",
            "cwd": "/tmp/project",
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash"
        }"#;

        let payload = parse_hook_payload(raw).unwrap();

        assert_eq!(status_for_payload(&payload), Some(AgentStatus::Running));
    }

    #[test]
    fn parses_a_stop_payload_as_done() {
        let raw = r#"{
            "session_id": "abc-123",
            "cwd": "/tmp/project",
            "hook_event_name": "Stop",
            "stop_reason": "end_turn"
        }"#;

        let payload = parse_hook_payload(raw).unwrap();

        assert_eq!(status_for_payload(&payload), Some(AgentStatus::Done));
    }

    #[test]
    fn permission_prompt_notification_needs_input() {
        let raw = r#"{
            "session_id": "abc-123",
            "cwd": "/tmp/project",
            "hook_event_name": "Notification",
            "notification_type": "permission_prompt",
            "message": "Allow tool use?"
        }"#;

        let payload = parse_hook_payload(raw).unwrap();

        assert_eq!(status_for_payload(&payload), Some(AgentStatus::NeedsInput));
    }

    #[test]
    fn unrelated_notification_types_are_skipped() {
        let raw = r#"{
            "session_id": "abc-123",
            "cwd": "/tmp/project",
            "hook_event_name": "Notification",
            "notification_type": "auth_success"
        }"#;

        let payload = parse_hook_payload(raw).unwrap();

        assert_eq!(status_for_payload(&payload), None);
    }

    #[test]
    fn unknown_hook_events_are_skipped() {
        let raw = r#"{
            "session_id": "abc-123",
            "cwd": "/tmp/project",
            "hook_event_name": "SomeFutureHook"
        }"#;

        let payload = parse_hook_payload(raw).unwrap();

        assert_eq!(status_for_payload(&payload), None);
    }

    #[test]
    fn malformed_json_fails_to_parse() {
        assert!(parse_hook_payload("not json").is_err());
    }
}
