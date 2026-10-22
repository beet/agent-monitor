use std::io::Read;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use agentmon_proto::{write_message, AgentEvent, ClientMessage, HostContext, SessionId};

use crate::hook_payload::{parse_hook_payload, sanitized_prompt, status_for_payload, HookPayload};

/// Hook subprocesses must never block or fail the calling hook (a `Stop`
/// hook exiting with code 2 would block the user's turn from ending), so
/// every socket operation here is bounded and every error is swallowed by
/// the caller rather than propagated as a process failure.
const SEND_TIMEOUT: Duration = Duration::from_millis(500);

/// Builds the event to report, or `None` if this hook payload doesn't
/// correspond to a tracked status change. `fired_at_ms` is the time the
/// caller captured at the very start of its process - see
/// `read_and_build_event`'s doc comment - and is carried through unchanged,
/// never recomputed here.
pub fn build_event(
    payload: &HookPayload,
    host_context: HostContext,
    pid: u32,
    fired_at_ms: u64,
) -> Option<AgentEvent> {
    let status = status_for_payload(payload)?;
    // Only a `UserPromptSubmit` payload carries prompt text - every other
    // hook event reports no prompt, regardless of whether `payload.prompt`
    // happens to be set. Harness-injected wrapper tags (slash-command
    // expansions, system-reminders, task-notifications) are stripped before
    // the surviving text becomes the event's prompt - see `sanitized_prompt`.
    let prompt = if payload.hook_event_name == "UserPromptSubmit" {
        payload.prompt.as_deref().and_then(sanitized_prompt)
    } else {
        None
    };
    Some(AgentEvent {
        session_id: SessionId(payload.session_id.clone()),
        cwd: PathBuf::from(&payload.cwd),
        host_context,
        pid,
        status,
        prompt,
        fired_at_ms,
    })
}

/// Reads a hook payload from `input`, and returns the event to report (if
/// any), or the parse error. `fired_at_ms` SHOULD be captured by the caller
/// as the very first instruction of its process - before host-context/
/// parent-pid detection, connecting to the daemon, or reading this payload -
/// so it reflects true hook-fire time rather than being skewed by any of
/// this process's own slower, jitter-prone work. See the agent-daemon
/// spec's "Local socket ingestion API" requirement.
pub fn read_and_build_event(
    mut input: impl Read,
    host_context: HostContext,
    pid: u32,
    fired_at_ms: u64,
) -> Result<Option<AgentEvent>, serde_json::Error> {
    let mut raw = String::new();
    // A stdin read error here is treated the same as empty input by the
    // caller (see main.rs) - there is nothing more specific to do about it.
    let _ = input.read_to_string(&mut raw);

    let payload: HookPayload = parse_hook_payload(&raw)?;
    Ok(build_event(&payload, host_context, pid, fired_at_ms))
}

/// Opens a connection to the daemon at `socket_path`, bounded by
/// `SEND_TIMEOUT`. Callers (`main.rs::run_report`) MUST call this before
/// reading/parsing their hook payload from stdin, not after: the daemon
/// applies `ReportEvent`s in the order their connections are accepted, and a
/// `UserPromptSubmit` payload (carrying the full prompt text) takes
/// measurably longer to read and parse than a `PreToolUse` payload's
/// handful of fields - connecting only after that work would let the
/// smaller, later-fired event's connection reach the daemon first, even
/// though Claude Code invoked the larger payload's hook first. See the
/// agent-daemon spec's "Local socket ingestion API" requirement.
pub fn connect(socket_path: &Path) -> std::io::Result<UnixStream> {
    let stream = UnixStream::connect(socket_path)?;
    stream.set_write_timeout(Some(SEND_TIMEOUT))?;
    Ok(stream)
}

/// Sends `event` over an already-open connection (see `connect`).
pub fn send_on(stream: &mut UnixStream, event: &AgentEvent) -> std::io::Result<()> {
    write_message(stream, &ClientMessage::ReportEvent { event: event.clone() })
}

/// Connects to the daemon at `socket_path` and sends `event` in one call.
/// Equivalent to `connect` followed by `send_on` - convenient when, unlike
/// `run_report`, there is no slower payload-building step that needs to
/// happen between opening the connection and sending on it.
pub fn send_event(socket_path: &Path, event: &AgentEvent) -> std::io::Result<()> {
    let mut stream = connect(socket_path)?;
    send_on(&mut stream, event)
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentmon_proto::{read_message, AgentStatus};
    use std::io::BufReader;
    use std::os::unix::net::UnixListener;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    fn unique_socket_path(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            % 1_000_000;
        let dir = PathBuf::from(format!("/tmp/amr-{tag}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("s")
    }

    #[test]
    fn build_event_maps_a_stop_payload_to_done() {
        let raw = r#"{
            "session_id": "abc-123",
            "cwd": "/tmp/project",
            "hook_event_name": "Stop"
        }"#;
        let payload = parse_hook_payload(raw).unwrap();

        let event = build_event(&payload, HostContext::Terminal, 4242, 1_700_000_000_000).unwrap();

        assert_eq!(event.session_id, SessionId("abc-123".to_string()));
        assert_eq!(event.cwd, PathBuf::from("/tmp/project"));
        assert_eq!(event.pid, 4242);
        assert_eq!(event.status, AgentStatus::Done);
    }

    #[test]
    fn build_event_carries_the_caller_supplied_fired_at_ms_unchanged() {
        let raw = r#"{
            "session_id": "abc-123",
            "cwd": "/tmp/project",
            "hook_event_name": "Stop"
        }"#;
        let payload = parse_hook_payload(raw).unwrap();

        let event = build_event(&payload, HostContext::Terminal, 4242, 1_234_567_890).unwrap();

        assert_eq!(
            event.fired_at_ms, 1_234_567_890,
            "fired_at_ms must be the caller-supplied value, not derived from SystemTime::now() inside build_event"
        );
    }

    #[test]
    fn build_event_carries_the_prompt_for_user_prompt_submit() {
        let raw = r#"{
            "session_id": "abc-123",
            "cwd": "/tmp/project",
            "hook_event_name": "UserPromptSubmit",
            "prompt": "Fix the flaky test"
        }"#;
        let payload = parse_hook_payload(raw).unwrap();

        let event = build_event(&payload, HostContext::Terminal, 4242, 1_700_000_000_000).unwrap();

        assert_eq!(event.prompt.as_deref(), Some("Fix the flaky test"));
    }

    #[test]
    fn build_event_strips_harness_wrapper_tags_from_the_prompt() {
        let raw = r#"{
            "session_id": "abc-123",
            "cwd": "/tmp/project",
            "hook_event_name": "UserPromptSubmit",
            "prompt": "<task-notification><task-id>b3sq8biwk</task-id></task-notification>"
        }"#;
        let payload = parse_hook_payload(raw).unwrap();

        let event = build_event(&payload, HostContext::Terminal, 4242, 1_700_000_000_000).unwrap();

        assert_eq!(
            event.prompt, None,
            "a prompt that is entirely a harness wrapper tag must carry no prompt text, not raw tag soup"
        );
    }

    #[test]
    fn build_event_carries_no_prompt_for_other_hook_events() {
        let raw = r#"{
            "session_id": "abc-123",
            "cwd": "/tmp/project",
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash"
        }"#;
        let payload = parse_hook_payload(raw).unwrap();

        let event = build_event(&payload, HostContext::Terminal, 4242, 1_700_000_000_000).unwrap();

        assert_eq!(event.prompt, None);
    }

    #[test]
    fn build_event_returns_none_for_a_non_status_notification() {
        let raw = r#"{
            "session_id": "abc-123",
            "cwd": "/tmp/project",
            "hook_event_name": "Notification",
            "notification_type": "auth_success"
        }"#;
        let payload = parse_hook_payload(raw).unwrap();

        assert!(build_event(&payload, HostContext::Terminal, 4242, 1_700_000_000_000).is_none());
    }

    #[test]
    fn sent_event_is_received_by_a_mock_daemon_listener() {
        let path = unique_socket_path("sample-payload");
        let listener = UnixListener::bind(&path).expect("bind mock listener");

        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept connection");
            let mut reader = BufReader::new(stream);
            read_message::<_, ClientMessage>(&mut reader)
                .expect("read message")
                .expect("connection should not close before a message arrives")
        });

        let raw_payload = r#"{
            "session_id": "session-xyz",
            "cwd": "/Users/beet/project",
            "hook_event_name": "Stop",
            "stop_reason": "end_turn"
        }"#;
        let event = read_and_build_event(raw_payload.as_bytes(), HostContext::Nvim, 555, 1_700_000_000_000)
            .expect("parse hook payload")
            .expect("Stop payload should produce an event");
        send_event(&path, &event).expect("send event to mock listener");

        let received = handle.join().expect("listener thread should not panic");
        match received {
            ClientMessage::ReportEvent { event: received_event } => {
                assert_eq!(received_event.session_id, SessionId("session-xyz".to_string()));
                assert_eq!(received_event.host_context, HostContext::Nvim);
                assert_eq!(received_event.pid, 555);
                assert_eq!(received_event.status, AgentStatus::Done);
            }
            other => panic!("expected ReportEvent, got {other:?}"),
        }
    }

    #[test]
    fn a_slow_payload_read_does_not_delay_opening_the_connection() {
        let path = unique_socket_path("slow-payload");
        let listener = UnixListener::bind(&path).expect("bind mock listener");

        let accepted_at: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
        let accepted_at_clone = accepted_at.clone();
        let listener_handle = thread::spawn(move || {
            let (_stream, _) = listener.accept().expect("accept connection");
            *accepted_at_clone.lock().unwrap() = Some(Instant::now());
        });

        // Mirrors `run_report`'s order: connect first...
        let _stream = connect(&path).expect("connect should succeed promptly");
        let connected_at = Instant::now();

        // ...then read/parse a deliberately slow payload (standing in for a
        // large `UserPromptSubmit` prompt taking measurably longer to read
        // than a small `PreToolUse` payload would).
        struct SlowReader;
        impl Read for SlowReader {
            fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
                thread::sleep(Duration::from_millis(200));
                Ok(0)
            }
        }
        let _ = read_and_build_event(SlowReader, HostContext::Terminal, 1, 1_700_000_000_000);

        listener_handle.join().expect("listener thread should not panic");
        let accepted_at = accepted_at.lock().unwrap().unwrap();
        assert!(
            accepted_at.duration_since(connected_at) < Duration::from_millis(50),
            "the daemon must accept the connection promptly, not after the slow payload read"
        );
    }

    #[test]
    fn a_connection_with_no_message_sent_closes_cleanly() {
        let path = unique_socket_path("no-message-sent");
        let listener = UnixListener::bind(&path).expect("bind mock listener");

        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept connection");
            let mut reader = BufReader::new(stream);
            read_message::<_, ClientMessage>(&mut reader)
        });

        {
            // Simulates a parse failure/no-trackable-event outcome after
            // connecting (see `run_report`): the stream is dropped without
            // ever calling `send_on`.
            let _stream = connect(&path).expect("connect should succeed");
        }

        let result = handle.join().expect("listener thread should not panic");
        assert!(
            matches!(result, Ok(None)),
            "a connection closed with no message sent must read as a clean EOF, not an error, got: {result:?}"
        );
    }

    #[test]
    fn sending_with_no_daemon_listening_fails_fast() {
        let path = unique_socket_path("no-daemon");
        // Deliberately never bind a listener at this path.
        let event = AgentEvent {
            session_id: SessionId("session-1".to_string()),
            cwd: PathBuf::from("/tmp/project"),
            host_context: HostContext::Terminal,
            pid: 1,
            status: AgentStatus::Done,
            prompt: None,
            fired_at_ms: 1_700_000_000_000,
        };

        let started = std::time::Instant::now();
        let result = send_event(&path, &event);

        assert!(result.is_err(), "sending to a nonexistent socket must fail");
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "must fail fast rather than hang the calling hook"
        );
    }
}
