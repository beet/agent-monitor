use std::io::BufReader;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use agentmon_proto::{AgentInfo, ClientMessage, ServerMessage};

use crate::ingest::Ingestor;
use crate::liveness::spawn_liveness_sweep;
use crate::protocol::{read_message, write_message};

/// Fans out registry updates to every currently-subscribed client.
#[derive(Clone, Default)]
struct Broadcaster {
    subscribers: Arc<Mutex<Vec<mpsc::Sender<AgentInfo>>>>,
}

impl Broadcaster {
    fn subscribe(&self) -> mpsc::Receiver<AgentInfo> {
        let (tx, rx) = mpsc::channel();
        self.subscribers.lock().unwrap().push(tx);
        rx
    }

    fn publish(&self, agent: AgentInfo) {
        let mut subscribers = self.subscribers.lock().unwrap();
        subscribers.retain(|tx| tx.send(agent.clone()).is_ok());
    }
}

/// Accepts connections on `listener` until it is closed, handling each on
/// its own thread, and runs the liveness sweep in the background so stale
/// agents reach subscribed clients too.
pub fn serve(listener: UnixListener, ingestor: Ingestor, liveness_interval: Duration) {
    let broadcaster = Broadcaster::default();

    let sweep_registry = ingestor.registry().clone();
    let sweep_broadcaster = broadcaster.clone();
    spawn_liveness_sweep(sweep_registry, liveness_interval, move |agent| {
        sweep_broadcaster.publish(agent);
    });

    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let ingestor = ingestor.clone();
        let broadcaster = broadcaster.clone();
        thread::spawn(move || handle_connection(stream, ingestor, broadcaster));
    }
}

fn handle_connection(stream: UnixStream, ingestor: Ingestor, broadcaster: Broadcaster) {
    let Ok(clone) = stream.try_clone() else { return };
    let mut reader = BufReader::new(clone);
    let mut writer = stream;

    let message = match read_message::<_, ClientMessage>(&mut reader) {
        Ok(message) => message,
        Err(err) => {
            eprintln!("agentd: rejecting malformed message on connection: {err}");
            return;
        }
    };

    match message {
        Some(ClientMessage::ReportEvent { event }) => {
            let agent = ingestor.ingest_event(event);
            broadcaster.publish(agent);
        }
        Some(ClientMessage::Subscribe) => {
            let snapshot = ingestor.registry().snapshot();
            let sent = write_message(&mut writer, &ServerMessage::Snapshot { agents: snapshot });
            if sent.is_err() {
                return;
            }

            for agent in broadcaster.subscribe() {
                if write_message(&mut writer, &ServerMessage::AgentUpdate { agent }).is_err() {
                    break;
                }
            }
        }
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notify::Notifier;
    use crate::registry::Registry;
    use crate::socket::bind_socket;
    use agentmon_proto::{AgentEvent, AgentStatus, HostContext, SessionId};
    use std::io::Write as _;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    const NO_LIVENESS_SWEEP: Duration = Duration::from_secs(3600);

    #[derive(Default)]
    struct RecordingNotifier {
        calls: Mutex<Vec<AgentInfo>>,
    }

    impl Notifier for RecordingNotifier {
        fn notify(&self, agent: &AgentInfo) {
            self.calls.lock().unwrap().push(agent.clone());
        }
    }

    fn unique_socket_path(tag: &str) -> PathBuf {
        // See the matching helper in socket.rs for why this lives under
        // /tmp rather than std::env::temp_dir().
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            % 1_000_000;
        let dir = PathBuf::from(format!("/tmp/ads-{tag}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("s")
    }

    /// Binds a socket, starts `serve` on it (liveness sweep disabled so it
    /// can't interfere with these tests), and returns the socket path plus
    /// the notifier so tests can assert on dispatched notifications.
    fn spawn_test_server(tag: &str) -> (PathBuf, Arc<RecordingNotifier>) {
        let path = unique_socket_path(tag);
        let listener = bind_socket(&path).expect("bind should succeed");
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier.clone());
        thread::spawn(move || serve(listener, ingestor, NO_LIVENESS_SWEEP));
        (path, notifier)
    }

    fn sample_event(status: AgentStatus) -> AgentEvent {
        AgentEvent {
            session_id: SessionId("session-1".to_string()),
            cwd: PathBuf::from("/tmp/project"),
            host_context: HostContext::Terminal,
            pid: 123,
            status,
        }
    }

    fn read_snapshot_until_nonempty(path: &PathBuf) -> Vec<AgentInfo> {
        for _ in 0..20 {
            let client = UnixStream::connect(path).expect("connect as subscriber");
            let mut writer = client.try_clone().expect("clone stream");
            write_message(&mut writer, &ClientMessage::Subscribe).expect("send subscribe");
            let mut reader = BufReader::new(client);
            let message: ServerMessage = read_message(&mut reader)
                .expect("read snapshot")
                .expect("connection should not close before snapshot");

            let ServerMessage::Snapshot { agents } = message else {
                panic!("expected a Snapshot message, got {message:?}");
            };
            if !agents.is_empty() {
                return agents;
            }
            thread::sleep(Duration::from_millis(10));
        }
        Vec::new()
    }

    /// Polls the daemon's snapshot until `session_id` is reported with
    /// `want`. Used to establish a happens-before point between two events
    /// on the same session sent over separate (deliberately one-shot,
    /// per design.md) reporter connections, each handled on its own thread.
    fn wait_for_status(
        path: &PathBuf,
        session_id: &SessionId,
        want: AgentStatus,
    ) -> Vec<AgentInfo> {
        for _ in 0..50 {
            let client = UnixStream::connect(path).expect("connect as subscriber");
            let mut writer = client.try_clone().expect("clone stream");
            write_message(&mut writer, &ClientMessage::Subscribe).expect("send subscribe");
            let mut reader = BufReader::new(client);
            let message: ServerMessage = read_message(&mut reader)
                .expect("read snapshot")
                .expect("connection should not close before snapshot");

            let ServerMessage::Snapshot { agents } = message else {
                panic!("expected a Snapshot message, got {message:?}");
            };
            if agents
                .iter()
                .any(|a| a.session_id == *session_id && a.status == want)
            {
                return agents;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("timed out waiting for session {session_id:?} to reach status {want:?}");
    }

    #[test]
    fn subscriber_receives_a_snapshot_containing_a_reported_event() {
        let (path, _notifier) = spawn_test_server("snapshot");

        {
            let mut reporter = UnixStream::connect(&path).expect("connect as reporter");
            write_message(
                &mut reporter,
                &ClientMessage::ReportEvent {
                    event: sample_event(AgentStatus::Running),
                },
            )
            .expect("send event");
        }

        let agents = read_snapshot_until_nonempty(&path);

        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].session_id, SessionId("session-1".to_string()));
        assert_eq!(agents[0].status, AgentStatus::Running);
    }

    #[test]
    fn malformed_payload_is_rejected_without_crashing_the_server() {
        let (path, _notifier) = spawn_test_server("malformed");

        {
            let mut bad_connection = UnixStream::connect(&path).expect("connect");
            bad_connection
                .write_all(b"not json at all\n")
                .expect("write malformed payload");
        }

        // A subsequent, well-formed connection must still be served
        // correctly - one bad connection must not take the server down.
        {
            let mut reporter = UnixStream::connect(&path).expect("connect as reporter");
            write_message(
                &mut reporter,
                &ClientMessage::ReportEvent {
                    event: sample_event(AgentStatus::Running),
                },
            )
            .expect("send event");
        }

        let agents = read_snapshot_until_nonempty(&path);
        assert_eq!(agents.len(), 1, "server must keep serving after malformed input");
    }

    #[test]
    fn reported_completion_updates_registry_and_triggers_a_notification() {
        let (path, notifier) = spawn_test_server("end-to-end");
        let session_id = SessionId("session-1".to_string());

        // Hook events are one-shot, short-lived connections (per design.md),
        // so each event gets its own connection. Wait for the first to land
        // before sending the second so the two aren't racing each other on
        // separate server threads.
        {
            let mut reporter = UnixStream::connect(&path).expect("connect as reporter");
            write_message(
                &mut reporter,
                &ClientMessage::ReportEvent {
                    event: sample_event(AgentStatus::Running),
                },
            )
            .expect("send running event");
        }
        wait_for_status(&path, &session_id, AgentStatus::Running);

        {
            let mut reporter = UnixStream::connect(&path).expect("connect as reporter");
            write_message(
                &mut reporter,
                &ClientMessage::ReportEvent {
                    event: sample_event(AgentStatus::Done),
                },
            )
            .expect("send done event");
        }
        let agents = wait_for_status(&path, &session_id, AgentStatus::Done);
        assert_eq!(agents.len(), 1);

        let mut notified = 0;
        for _ in 0..20 {
            notified = notifier.calls.lock().unwrap().len();
            if notified > 0 {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(notified, 1);
    }

    #[test]
    fn late_needs_input_event_after_done_is_ignored_across_connections() {
        let (path, notifier) = spawn_test_server("late-needs-input");
        let session_id = SessionId("session-1".to_string());

        {
            let mut reporter = UnixStream::connect(&path).expect("connect as reporter");
            write_message(
                &mut reporter,
                &ClientMessage::ReportEvent {
                    event: sample_event(AgentStatus::Running),
                },
            )
            .expect("send running event");
        }
        wait_for_status(&path, &session_id, AgentStatus::Running);

        {
            let mut reporter = UnixStream::connect(&path).expect("connect as reporter");
            write_message(
                &mut reporter,
                &ClientMessage::ReportEvent {
                    event: sample_event(AgentStatus::Done),
                },
            )
            .expect("send done event");
        }
        wait_for_status(&path, &session_id, AgentStatus::Done);

        // Simulates a delayed idle-prompt Notification hook landing on its
        // own connection/thread after the session already reported Stop.
        {
            let mut reporter = UnixStream::connect(&path).expect("connect as reporter");
            write_message(
                &mut reporter,
                &ClientMessage::ReportEvent {
                    event: sample_event(AgentStatus::NeedsInput),
                },
            )
            .expect("send late needs-input event");
        }

        // Give the server ample time to process (and, if the guard were
        // broken, wrongly apply) the late event before asserting it never
        // took effect.
        let mut agents = Vec::new();
        for _ in 0..20 {
            agents = read_snapshot_until_nonempty(&path);
            thread::sleep(Duration::from_millis(10));
        }

        assert_eq!(agents.len(), 1);
        assert_eq!(
            agents[0].status,
            AgentStatus::Done,
            "a needs-input event arriving after done must not change the session's status"
        );
        assert_eq!(
            notifier.calls.lock().unwrap().len(),
            1,
            "the ignored needs-input event must not trigger a second notification"
        );
    }
}
