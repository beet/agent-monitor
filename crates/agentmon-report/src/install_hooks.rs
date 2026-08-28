use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

/// Restricts the `Notification` hook to only the notification types that
/// mean the agent needs the user's attention (see
/// `hook_payload::NEEDS_INPUT_NOTIFICATION_TYPES`), so Claude Code doesn't
/// invoke this reporter for every notification.
pub const NEEDS_INPUT_MATCHER: &str =
    "permission_prompt|idle_prompt|elicitation_dialog|elicitation_url_dialog|agent_needs_input";

const HOOK_EVENTS: &[(&str, &str)] = &[
    ("UserPromptSubmit", ""),
    ("Stop", ""),
    ("Notification", NEEDS_INPUT_MATCHER),
];

/// The default per-user Claude Code settings file.
pub fn default_settings_path() -> PathBuf {
    let home = std::env::var_os("HOME").expect("HOME environment variable must be set");
    PathBuf::from(home).join(".claude").join("settings.json")
}

/// Adds (or updates) this reporter's hook entries in the Claude Code
/// settings file at `settings_path`, using `command` as the hook command.
/// Any existing hooks - for these events or others - are left untouched.
pub fn install_hooks(settings_path: &Path, command: &str) -> io::Result<()> {
    let mut root = read_settings(settings_path)?;

    let hooks = root
        .as_object_mut()
        .expect("read_settings always returns an object")
        .entry("hooks")
        .or_insert_with(|| json!({}));
    if !hooks.is_object() {
        *hooks = json!({});
    }
    let hooks_obj = hooks.as_object_mut().unwrap();

    for (event, matcher) in HOOK_EVENTS {
        upsert_hook(hooks_obj, event, matcher, command);
    }

    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let pretty = serde_json::to_string_pretty(&root)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    fs::write(settings_path, format!("{pretty}\n"))
}

fn read_settings(settings_path: &Path) -> io::Result<Value> {
    if !settings_path.exists() {
        return Ok(json!({}));
    }
    let text = fs::read_to_string(settings_path)?;
    if text.trim().is_empty() {
        return Ok(json!({}));
    }
    let value: Value = serde_json::from_str(&text)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    Ok(if value.is_object() { value } else { json!({}) })
}

/// Ensures `hooks_obj[event]` contains a hook group invoking `command`,
/// without touching any existing entries for `event` or any other event.
fn upsert_hook(hooks_obj: &mut serde_json::Map<String, Value>, event: &str, matcher: &str, command: &str) {
    let groups = hooks_obj
        .entry(event.to_string())
        .or_insert_with(|| json!([]));
    if !groups.is_array() {
        *groups = json!([]);
    }
    let groups_arr = groups.as_array_mut().unwrap();

    let already_installed = groups_arr.iter().any(|group| {
        group["hooks"]
            .as_array()
            .map(|hooks| hooks.iter().any(|hook| hook["command"] == command))
            .unwrap_or(false)
    });
    if already_installed {
        return;
    }

    groups_arr.push(json!({
        "matcher": matcher,
        "hooks": [
            { "type": "command", "command": command }
        ]
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_settings_path(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "agentmon-report-test-{tag}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir.join("settings.json")
    }

    #[test]
    fn creates_settings_file_with_hooks_when_none_exists() {
        let path = unique_settings_path("fresh");

        install_hooks(&path, "/usr/local/bin/agentmon-report").unwrap();

        let written: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        for (event, _) in HOOK_EVENTS {
            let groups = written["hooks"][event].as_array().unwrap();
            assert!(groups
                .iter()
                .any(|g| g["hooks"][0]["command"] == "/usr/local/bin/agentmon-report"));
        }
        assert_eq!(written["hooks"]["Notification"][0]["matcher"], NEEDS_INPUT_MATCHER);
    }

    #[test]
    fn preserves_unrelated_existing_settings_and_hooks() {
        let path = unique_settings_path("preserve");
        fs::write(
            &path,
            r#"{
                "some_other_setting": true,
                "hooks": {
                    "PreToolUse": [
                        { "matcher": "Bash", "hooks": [{ "type": "command", "command": "my-other-tool" }] }
                    ],
                    "Stop": [
                        { "matcher": "", "hooks": [{ "type": "command", "command": "existing-stop-hook" }] }
                    ]
                }
            }"#,
        )
        .unwrap();

        install_hooks(&path, "/usr/local/bin/agentmon-report").unwrap();

        let written: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(written["some_other_setting"], true);
        assert_eq!(written["hooks"]["PreToolUse"][0]["hooks"][0]["command"], "my-other-tool");

        let stop_groups = written["hooks"]["Stop"].as_array().unwrap();
        assert!(stop_groups
            .iter()
            .any(|g| g["hooks"][0]["command"] == "existing-stop-hook"));
        assert!(stop_groups
            .iter()
            .any(|g| g["hooks"][0]["command"] == "/usr/local/bin/agentmon-report"));
        assert_eq!(stop_groups.len(), 2, "existing Stop hook must not be clobbered");
    }

    #[test]
    fn running_install_twice_does_not_duplicate_entries() {
        let path = unique_settings_path("idempotent");

        install_hooks(&path, "/usr/local/bin/agentmon-report").unwrap();
        install_hooks(&path, "/usr/local/bin/agentmon-report").unwrap();

        let written: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let stop_groups = written["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop_groups.len(), 1, "re-running install must not duplicate the hook");
    }
}
