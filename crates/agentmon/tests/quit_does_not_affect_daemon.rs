//! Verifies task 7.1's requirement: quitting the TUI must not affect the
//! daemon or the agents it tracks. Runs a real `agentd` daemon (via its
//! library API, in-process on a background thread) and drives it with the
//! same wire protocol the TUI uses.

use std::io::BufReader;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use agentd::ingest::Ingestor;
use agentd::notify::Notifier;
use agentd::registry::Registry;
use agentd::server::serve;
use agentd::socket::bind_socket;

use agentmon::client::{spawn_client, ClientEvent};
use agentmon_proto::{
    read_message, write_message, AgentEvent, AgentInfo, AgentStatus, ClientMessage, HostContext,
    ServerMessage, SessionId,
};

struct NoopNotifier;
impl Notifier for NoopNotifier {
    fn notify(&self, _agent: &AgentInfo) {}
}

fn unique_socket_path(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        % 1_000_000;
    let dir = PathBuf::from(format!("/tmp/amq-{tag}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("s")
}

#[test]
fn quitting_a_tui_client_does_not_affect_the_daemon_or_its_tracked_agents() {
    let path = unique_socket_path("quit");

    let listener = bind_socket(&path).expect("bind daemon socket");
    let ingestor = Ingestor::new(Registry::new(), Arc::new(NoopNotifier));
    thread::spawn(move || serve(listener, ingestor, Duration::from_secs(3600)));

    // Register an agent, as agentmon-report would via a hook.
    let mut reporter = UnixStream::connect(&path).expect("connect as reporter");
    write_message(
        &mut reporter,
        &ClientMessage::ReportEvent {
            event: AgentEvent {
                session_id: SessionId("session-1".to_string()),
                cwd: PathBuf::from("/tmp/project"),
                host_context: HostContext::Terminal,
                pid: std::process::id(),
                status: AgentStatus::Running,
            },
        },
    )
    .expect("report an event");
    drop(reporter);

    // A TUI client connects and subscribes, like `agentmon` starting up,
    // and receives the snapshot containing the agent. This uses the same
    // wire protocol agentmon::client uses, but as a plain connection (not
    // through spawn_client's background thread) so it can be closed
    // immediately below without waiting on a blocking socket read that the
    // daemon has no further reason to unblock.
    //
    // The reporter's registry update happens on a separate daemon-side
    // thread than this subscribe, so retry briefly rather than assuming
    // it's already landed.
    let mut agents = Vec::new();
    for _ in 0..20 {
        let mut quitting_client = UnixStream::connect(&path).expect("connect as TUI client");
        write_message(&mut quitting_client, &ClientMessage::Subscribe).expect("subscribe");
        let mut quitting_reader = BufReader::new(quitting_client.try_clone().unwrap());
        match read_message::<_, ServerMessage>(&mut quitting_reader).unwrap() {
            Some(ServerMessage::Snapshot { agents: snapshot }) if !snapshot.is_empty() => {
                agents = snapshot;
                // "Quit": close the connection, exactly as process exit
                // would close the real TUI's socket fd.
                drop(quitting_reader);
                drop(quitting_client);
                break;
            }
            Some(ServerMessage::Snapshot { .. }) => {
                drop(quitting_reader);
                drop(quitting_client);
                thread::sleep(Duration::from_millis(20));
            }
            other => panic!("expected a snapshot, got {other:?}"),
        }
    }
    assert_eq!(agents.len(), 1, "the reported agent should have landed in time");

    // A fresh client connects afterward and must still see the same agent -
    // proving the daemon and its registry were unaffected by the first
    // client quitting.
    let (tx2, rx2) = mpsc::channel();
    spawn_client(path, tx2);
    match rx2.recv_timeout(Duration::from_secs(2)) {
        Ok(ClientEvent::Snapshot(agents)) => {
            assert_eq!(agents.len(), 1, "daemon should still be tracking the agent");
            assert_eq!(agents[0].session_id, SessionId("session-1".to_string()));
        }
        other => panic!("expected a snapshot from the still-running daemon, got {other:?}"),
    }
}
