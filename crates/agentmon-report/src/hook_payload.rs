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
    /// The submitted prompt text. Only present on a `UserPromptSubmit`
    /// payload.
    #[serde(default)]
    pub prompt: Option<String>,
}

pub fn parse_hook_payload(raw: &str) -> serde_json::Result<HookPayload> {
    serde_json::from_str(raw)
}

/// Wrapper tags Claude Code itself injects around a `UserPromptSubmit`
/// payload's prompt text that never carry human-authored content - removed
/// together with their contents. Add new tags here as Claude Code's harness
/// introduces them.
const STRIP_WITH_CONTENTS: &[&str] = &[
    "command-message",
    "system-reminder",
    "task-notification",
    "local-command-stdout",
];

/// Wrapper tags that carry the literal slash command and arguments the user
/// typed - only the tag markup is removed, keeping the contents.
const UNWRAP_KEEP_CONTENTS: &[&str] = &["command-name", "command-args"];

/// Removes Claude Code's own harness-injected wrapper tags from a
/// `UserPromptSubmit` payload's prompt text, so a slash-command expansion, a
/// system-reminder injection, or a backgrounded task's completion
/// notification is never mistaken for literal human-typed text. See
/// `STRIP_WITH_CONTENTS` and `UNWRAP_KEEP_CONTENTS` for which tags are
/// removed outright and which are merely unwrapped. An opening tag with no
/// matching close is left untouched as ordinary text, and a payload with no
/// recognized tags is returned unchanged.
fn strip_harness_wrapper_tags(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut pos = 0;

    while pos < input.len() {
        let Some((tag, open_start, open_len, strip_contents)) = next_wrapper_tag(input, pos)
        else {
            result.push_str(&input[pos..]);
            break;
        };

        result.push_str(&input[pos..open_start]);

        let content_start = open_start + open_len;
        match find_matching_close(input, tag, content_start) {
            Some((close_start, close_end)) => {
                if !strip_contents {
                    result.push_str(&input[content_start..close_start]);
                }
                pos = close_end;
            }
            None => {
                // Unterminated opening tag: treat it as ordinary text rather
                // than silently dropping the rest of the payload.
                result.push_str(&input[open_start..content_start]);
                pos = content_start;
            }
        }
    }

    result
}

/// Finds the earliest recognized opening tag at or after `from`, returning
/// its name, byte offset, byte length of the opening tag itself, and whether
/// it (and its contents) should be stripped outright (`true`) or merely
/// unwrapped (`false`).
fn next_wrapper_tag(input: &str, from: usize) -> Option<(&'static str, usize, usize, bool)> {
    let mut best: Option<(&'static str, usize, usize, bool)> = None;
    let candidates = STRIP_WITH_CONTENTS
        .iter()
        .map(|&tag| (tag, true))
        .chain(UNWRAP_KEEP_CONTENTS.iter().map(|&tag| (tag, false)));
    for (tag, strip_contents) in candidates {
        if let Some((start, len)) = find_open_tag(input, from, tag) {
            if best.is_none_or(|(_, best_start, _, _)| start < best_start) {
                best = Some((tag, start, len, strip_contents));
            }
        }
    }
    best
}

fn find_open_tag(input: &str, from: usize, tag: &str) -> Option<(usize, usize)> {
    let needle = format!("<{tag}>");
    input[from..]
        .find(needle.as_str())
        .map(|rel| (from + rel, needle.len()))
}

/// Finds the close tag matching the opening tag whose contents start at
/// `from`, accounting for same-tag nesting depth (a nested occurrence of the
/// same tag name re-opens before its own close is counted). Tags of a
/// different name nested inside do not affect this matching, since only the
/// literal open/close markers for `tag` are tracked. Returns `None` if no
/// matching close is found.
fn find_matching_close(input: &str, tag: &str, from: usize) -> Option<(usize, usize)> {
    let open_needle = format!("<{tag}>");
    let close_needle = format!("</{tag}>");
    let mut depth = 1;
    let mut pos = from;
    loop {
        let next_open = input[pos..].find(open_needle.as_str()).map(|r| pos + r);
        let next_close = input[pos..].find(close_needle.as_str()).map(|r| pos + r);
        match (next_open, next_close) {
            (Some(o), Some(c)) if o < c => {
                depth += 1;
                pos = o + open_needle.len();
            }
            (_, Some(c)) => {
                depth -= 1;
                if depth == 0 {
                    return Some((c, c + close_needle.len()));
                }
                pos = c + close_needle.len();
            }
            _ => return None,
        }
    }
}

/// Strips harness-injected wrapper tags from a submitted prompt (see
/// `strip_harness_wrapper_tags`) and treats the result as "no prompt" when
/// nothing human-authored survives - the same as an event whose hook never
/// carried a prompt at all - rather than an empty or whitespace-only string.
pub(crate) fn sanitized_prompt(raw: &str) -> Option<String> {
    let stripped = strip_harness_wrapper_tags(raw);
    let trimmed = stripped.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
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
        "PostToolUse" => Some(AgentStatus::Running),
        "Stop" => Some(AgentStatus::Done),
        "PermissionDenied" => Some(AgentStatus::Declined),
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
    fn parses_a_user_prompt_submit_payloads_prompt_text() {
        let raw = r#"{
            "session_id": "abc-123",
            "cwd": "/tmp/project",
            "hook_event_name": "UserPromptSubmit",
            "prompt": "Fix the flaky test"
        }"#;

        let payload = parse_hook_payload(raw).unwrap();

        assert_eq!(payload.prompt.as_deref(), Some("Fix the flaky test"));
    }

    #[test]
    fn a_payload_with_no_prompt_field_parses_with_no_prompt() {
        let raw = r#"{
            "session_id": "abc-123",
            "cwd": "/tmp/project",
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash"
        }"#;

        let payload = parse_hook_payload(raw).unwrap();

        assert_eq!(payload.prompt, None);
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
    fn parses_a_post_tool_use_payload_as_running() {
        let raw = r#"{
            "session_id": "abc-123",
            "cwd": "/tmp/project",
            "hook_event_name": "PostToolUse",
            "tool_name": "Bash"
        }"#;

        let payload = parse_hook_payload(raw).unwrap();

        assert_eq!(status_for_payload(&payload), Some(AgentStatus::Running));
    }

    #[test]
    fn parses_a_permission_denied_payload_as_declined() {
        let raw = r#"{
            "session_id": "abc-123",
            "cwd": "/tmp/project",
            "hook_event_name": "PermissionDenied",
            "tool_name": "Bash",
            "denied_by": "user"
        }"#;

        let payload = parse_hook_payload(raw).unwrap();

        assert_eq!(status_for_payload(&payload), Some(AgentStatus::Declined));
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

    #[test]
    fn a_pure_task_notification_payload_strips_to_nothing() {
        let raw = "<task-notification><task-id>b3sq8biwk</task-id><status>done</status></task-notification>";

        assert_eq!(strip_harness_wrapper_tags(raw).trim(), "");
        assert_eq!(sanitized_prompt(raw), None);
    }

    #[test]
    fn a_system_reminder_is_removed_leaving_the_users_own_text() {
        let raw = "<system-reminder>Some injected context.</system-reminder>Fix the flaky test";

        assert_eq!(strip_harness_wrapper_tags(raw), "Fix the flaky test");
        assert_eq!(sanitized_prompt(raw).as_deref(), Some("Fix the flaky test"));
    }

    #[test]
    fn a_slash_command_payload_keeps_its_name_and_args() {
        let raw = "<command-message>opsx:propose is running</command-message>\n\
                   <command-name>/opsx:propose</command-name>\n\
                   <command-args>Agent prompts appearing as raw XML</command-args>";

        assert_eq!(
            strip_harness_wrapper_tags(raw),
            "\n/opsx:propose\nAgent prompts appearing as raw XML"
        );
        assert_eq!(
            sanitized_prompt(raw).as_deref(),
            Some("/opsx:propose\nAgent prompts appearing as raw XML")
        );
    }

    #[test]
    fn nested_tags_inside_task_notification_are_all_removed() {
        let raw = "<task-notification><payload><task-id>abc</task-id><nested><task-id>def</task-id></nested></payload></task-notification>";

        assert_eq!(strip_harness_wrapper_tags(raw), "");
    }

    #[test]
    fn a_payload_with_no_recognized_tags_is_unaffected() {
        let raw = "Fix the flaky test in ui.rs";

        assert_eq!(strip_harness_wrapper_tags(raw), raw);
        assert_eq!(sanitized_prompt(raw).as_deref(), Some(raw));
    }

    #[test]
    fn an_unterminated_opening_tag_is_left_untouched() {
        let raw = "<system-reminder>never closed";

        assert_eq!(strip_harness_wrapper_tags(raw), raw);
    }

    #[test]
    fn a_fully_wrapped_payload_with_only_whitespace_remaining_collapses_to_none() {
        let raw = "  <task-notification><task-id>abc</task-id></task-notification>  ";

        assert_eq!(sanitized_prompt(raw), None);
    }
}
