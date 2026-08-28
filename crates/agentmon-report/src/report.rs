use std::io::Read;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use agentmon_proto::{write_message, AgentEvent, ClientMessage, HostContext, SessionId};

use crate::hook_payload::{parse_hook_payload, status_for_payload, HookPayload};

/// Hook subprocesses must never block or fail the calling hook (a `Stop`
/// hook exiting with code 2 would block the user's turn from ending), so
/// every socket operation here is bounded and every error is swallowed by
/// the caller rather than propagated as a process failure.
const SEND_TIMEOUT: Duration = Duration::from_millis(500);

/// Builds the event to report, or `None` if this hook payload doesn't
/// correspond to a tracked status change.
pub fn build_event(payload: &HookPayload, host_context: HostContext, pid: u32) -> Option<AgentEvent> {
    let status = status_for_payload(payload)?;
    Some(AgentEvent {
        session_id: SessionId(payload.session_id.clone()),
        cwd: PathBuf::from(&payload.cwd),
        host_context,
        pid,
        status,
    })
}

/// Reads a hook payload from `input`, and returns the event to report (if
/// any), or the parse error.
pub fn read_and_build_event(
    mut input: impl Read,
    host_context: HostContext,
    pid: u32,
) -> Result<Option<AgentEvent>, serde_json::Error> {
    let mut raw = String::new();
    // A stdin read error here is treated the same as empty input by the
    // caller (see main.rs) - there is nothing more specific to do about it.
    let _ = input.read_to_string(&mut raw);

    let payload: HookPayload = parse_hook_payload(&raw)?;
    Ok(build_event(&payload, host_context, pid))
}

/// Sends `event` to the daemon at `socket_path`. Bounded by `SEND_TIMEOUT` so
/// a hung or unreachable daemon can never make a hook wait indefinitely.
pub fn send_event(socket_path: &Path, event: &AgentEvent) -> std::io::Result<()> {
    let mut stream = UnixStream::connect(socket_path)?;
    stream.set_write_timeout(Some(SEND_TIMEOUT))?;
    write_message(&mut stream, &ClientMessage::ReportEvent { event: event.clone() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentmon_proto::{read_message, AgentStatus};
    use std::io::BufReader;
    use std::os::unix::net::UnixListener;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

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

        let event = build_event(&payload, HostContext::Terminal, 4242).unwrap();

        assert_eq!(event.session_id, SessionId("abc-123".to_string()));
        assert_eq!(event.cwd, PathBuf::from("/tmp/project"));
        assert_eq!(event.pid, 4242);
        assert_eq!(event.status, AgentStatus::Done);
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

        assert!(build_event(&payload, HostContext::Terminal, 4242).is_none());
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
        let event = read_and_build_event(raw_payload.as_bytes(), HostContext::Nvim, 555)
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
    fn sending_with_no_daemon_listening_fails_fast() {
        let path = unique_socket_path("no-daemon");
        // Deliberately never bind a listener at this path.
        let event = AgentEvent {
            session_id: SessionId("session-1".to_string()),
            cwd: PathBuf::from("/tmp/project"),
            host_context: HostContext::Terminal,
            pid: 1,
            status: AgentStatus::Done,
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
