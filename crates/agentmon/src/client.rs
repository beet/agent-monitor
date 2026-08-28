use std::io::BufReader;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use agentmon_proto::{read_message, write_message, AgentInfo, ClientMessage, ServerMessage};

const INITIAL_BACKOFF: Duration = Duration::from_millis(250);
const MAX_BACKOFF: Duration = Duration::from_secs(5);

/// What the client thread reports back to the UI loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientEvent {
    /// Never (yet) connected - the daemon may not be running.
    Unreachable(String),
    /// A previously-established connection dropped and a reconnect attempt
    /// is under way.
    Reconnecting,
    Snapshot(Vec<AgentInfo>),
    Update(AgentInfo),
}

/// Connects to the daemon at `socket_path`, subscribes, and forwards every
/// message it sends as a `ClientEvent`. If the connection drops or is never
/// reachable, retries with exponential backoff (reset after each successful
/// connection) rather than giving up, so a daemon restart is picked back up
/// automatically. Runs on its own thread so the UI loop is never blocked on
/// socket I/O.
pub fn spawn_client(socket_path: PathBuf, events: Sender<ClientEvent>) -> JoinHandle<()> {
    thread::spawn(move || run_client(&socket_path, &events))
}

enum StreamOutcome {
    ConnectFailed(String),
    Disconnected,
    /// The UI's receiver was dropped; nothing more to do.
    ReceiverGone,
}

fn run_client(socket_path: &Path, events: &Sender<ClientEvent>) {
    let mut backoff = INITIAL_BACKOFF;
    let mut ever_connected = false;

    loop {
        match connect_and_stream(socket_path, events) {
            StreamOutcome::ReceiverGone => return,
            StreamOutcome::ConnectFailed(reason) => {
                let sent = if ever_connected {
                    events.send(ClientEvent::Reconnecting)
                } else {
                    events.send(ClientEvent::Unreachable(reason))
                };
                if sent.is_err() {
                    return;
                }
            }
            StreamOutcome::Disconnected => {
                ever_connected = true;
                backoff = INITIAL_BACKOFF; // we just proved the daemon is reachable
                if events.send(ClientEvent::Reconnecting).is_err() {
                    return;
                }
            }
        }

        thread::sleep(backoff);
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}

/// Connects once, subscribes, and streams messages until the connection
/// ends (cleanly or with an error) or the UI goes away.
fn connect_and_stream(socket_path: &Path, events: &Sender<ClientEvent>) -> StreamOutcome {
    let stream = match UnixStream::connect(socket_path) {
        Ok(stream) => stream,
        Err(err) => return StreamOutcome::ConnectFailed(err.to_string()),
    };

    let mut writer = match stream.try_clone() {
        Ok(writer) => writer,
        Err(err) => return StreamOutcome::ConnectFailed(err.to_string()),
    };
    if let Err(err) = write_message(&mut writer, &ClientMessage::Subscribe) {
        return StreamOutcome::ConnectFailed(err.to_string());
    }

    let mut reader = BufReader::new(stream);
    loop {
        match read_message::<_, ServerMessage>(&mut reader) {
            Ok(Some(ServerMessage::Snapshot { agents })) => {
                if events.send(ClientEvent::Snapshot(agents)).is_err() {
                    return StreamOutcome::ReceiverGone;
                }
            }
            Ok(Some(ServerMessage::AgentUpdate { agent })) => {
                if events.send(ClientEvent::Update(agent)).is_err() {
                    return StreamOutcome::ReceiverGone;
                }
            }
            Ok(None) | Err(_) => return StreamOutcome::Disconnected,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentmon_proto::{AgentStatus, HostContext, SessionId};
    use std::os::unix::net::UnixListener;
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_socket_path(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            % 1_000_000;
        let dir = PathBuf::from(format!("/tmp/amon-{tag}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("s")
    }

    fn sample_agent(status: AgentStatus) -> AgentInfo {
        AgentInfo {
            session_id: SessionId("session-1".to_string()),
            cwd: PathBuf::from("/tmp/project"),
            host_context: HostContext::Terminal,
            pid: 42,
            status,
            last_updated_ms: 0,
        }
    }

    #[test]
    fn reports_unreachable_when_nothing_is_listening() {
        let path = unique_socket_path("unreachable");
        let (tx, rx) = mpsc::channel();

        spawn_client(path, tx);

        match rx.recv_timeout(Duration::from_secs(2)) {
            Ok(ClientEvent::Unreachable(_)) => {}
            other => panic!("expected Unreachable, got {other:?}"),
        }
    }

    #[test]
    fn forwards_a_snapshot_then_update_then_stale_sequence_from_a_mock_daemon() {
        let path = unique_socket_path("mock-daemon");
        let listener = UnixListener::bind(&path).expect("bind mock daemon listener");

        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept client connection");
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut writer = stream;

            let subscribe: ClientMessage = read_message(&mut reader)
                .expect("read subscribe")
                .expect("connection should not close before subscribing");
            assert_eq!(subscribe, ClientMessage::Subscribe);

            write_message(
                &mut writer,
                &ServerMessage::Snapshot {
                    agents: vec![sample_agent(AgentStatus::Running)],
                },
            )
            .unwrap();
            write_message(
                &mut writer,
                &ServerMessage::AgentUpdate {
                    agent: sample_agent(AgentStatus::Done),
                },
            )
            .unwrap();
            write_message(
                &mut writer,
                &ServerMessage::AgentUpdate {
                    agent: sample_agent(AgentStatus::Stale),
                },
            )
            .unwrap();
        });

        let (tx, rx) = mpsc::channel();
        spawn_client(path, tx);

        let snapshot = rx.recv_timeout(Duration::from_secs(2)).expect("snapshot event");
        assert_eq!(
            snapshot,
            ClientEvent::Snapshot(vec![sample_agent(AgentStatus::Running)])
        );

        let update = rx.recv_timeout(Duration::from_secs(2)).expect("update event");
        assert_eq!(update, ClientEvent::Update(sample_agent(AgentStatus::Done)));

        let stale = rx.recv_timeout(Duration::from_secs(2)).expect("stale event");
        assert_eq!(stale, ClientEvent::Update(sample_agent(AgentStatus::Stale)));

        server.join().expect("mock daemon thread should not panic");
    }

    #[test]
    fn reconnects_and_repopulates_after_the_daemon_stops_and_restarts() {
        let path = unique_socket_path("restart");
        let daemon_path = path.clone();

        // Bind the first listener before the client thread starts, so its
        // very first connect attempt is guaranteed to succeed rather than
        // racing the daemon's startup.
        let listener = UnixListener::bind(&daemon_path).expect("bind first listener");

        let daemon = thread::spawn(move || {
            // First daemon instance: accept once, send a snapshot, then
            // disappear entirely (like a crash / restart).
            {
                let (stream, _) = listener.accept().expect("accept first connection");
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut writer = stream;
                let subscribe: ClientMessage = read_message(&mut reader).unwrap().unwrap();
                assert_eq!(subscribe, ClientMessage::Subscribe);
                write_message(
                    &mut writer,
                    &ServerMessage::Snapshot {
                        agents: vec![sample_agent(AgentStatus::Running)],
                    },
                )
                .unwrap();
                // Dropping the connection and the listener stops it from
                // accepting further connections, like the daemon exiting.
                drop(writer);
                drop(reader);
                drop(listener);
            }
            std::fs::remove_file(&daemon_path).ok();

            // Give the client a chance to notice the drop and fail at
            // least one reconnect attempt before the new daemon comes up.
            thread::sleep(Duration::from_millis(400));

            // "Restarted" daemon: accept the client's retry and send an
            // updated snapshot.
            let listener2 = UnixListener::bind(&daemon_path).expect("bind second listener");
            let (stream2, _) = listener2.accept().expect("accept reconnect");
            let mut reader2 = BufReader::new(stream2.try_clone().unwrap());
            let mut writer2 = stream2;
            let subscribe2: ClientMessage = read_message(&mut reader2).unwrap().unwrap();
            assert_eq!(subscribe2, ClientMessage::Subscribe);
            write_message(
                &mut writer2,
                &ServerMessage::Snapshot {
                    agents: vec![sample_agent(AgentStatus::Done)],
                },
            )
            .unwrap();
        });

        let (tx, rx) = mpsc::channel();
        spawn_client(path, tx);

        match rx.recv_timeout(Duration::from_secs(2)).expect("initial snapshot") {
            ClientEvent::Snapshot(agents) => {
                assert_eq!(agents, vec![sample_agent(AgentStatus::Running)])
            }
            other => panic!("expected initial Snapshot, got {other:?}"),
        }

        let mut saw_reconnecting = false;
        let mut final_snapshot = None;
        for _ in 0..50 {
            match rx.recv_timeout(Duration::from_secs(3)) {
                Ok(ClientEvent::Reconnecting) => saw_reconnecting = true,
                Ok(ClientEvent::Unreachable(_)) => {}
                Ok(ClientEvent::Snapshot(agents)) => {
                    final_snapshot = Some(agents);
                    break;
                }
                Ok(other) => panic!("unexpected event while waiting to reconnect: {other:?}"),
                Err(_) => break,
            }
        }

        assert!(
            saw_reconnecting,
            "client should report a reconnect attempt after the daemon drops"
        );
        assert_eq!(
            final_snapshot,
            Some(vec![sample_agent(AgentStatus::Done)]),
            "list should repopulate from the restarted daemon"
        );

        daemon.join().expect("mock daemon thread should not panic");
    }
}
