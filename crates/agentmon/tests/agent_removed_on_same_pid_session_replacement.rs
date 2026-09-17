//! Verifies the fix for the details modal's Agents pane showing two rows for
//! one pid: when a pid's session id is replaced (e.g. `/clear`), the daemon
//! must broadcast the old session's removal, and an already-connected TUI
//! client must drop it - ending up with exactly one tracked agent for that
//! pid, rendered as exactly one line in the details modal's Agents pane. See
//! the `agent-daemon` and `agent-monitor-tui` delta specs in
//! `openspec/changes/fix-agents-pane-duplicate-pid-rows`.

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

use agentmon::app::App;
use agentmon::client::{spawn_client, ClientEvent};
use agentmon::ui::render;
use agentmon_proto::{write_message, AgentEvent, AgentInfo, AgentStatus, ClientMessage, HostContext, SessionId};

use ratatui::backend::TestBackend;
use ratatui::Terminal;

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

fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    let mut text = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            text.push_str(buffer[(x, y)].symbol());
        }
        text.push('\n');
    }
    text
}

#[test]
fn a_same_pid_session_replacement_leaves_exactly_one_agent_and_one_details_modal_row() {
    let path = unique_socket_path("agent-removed-e2e");

    let listener = bind_socket(&path).expect("bind daemon socket");
    let ingestor = Ingestor::new(Registry::new(), Arc::new(NoopNotifier));
    thread::spawn(move || serve(listener, ingestor, Duration::from_secs(3600)));

    let pid = 123;
    let cwd = PathBuf::from("/tmp/project");

    // session-1 registers first, as agentmon-report would on the agent's
    // first hook event.
    let mut reporter = UnixStream::connect(&path).expect("connect as reporter");
    write_message(
        &mut reporter,
        &ClientMessage::ReportEvent {
            event: AgentEvent {
                session_id: SessionId("session-1".to_string()),
                cwd: cwd.clone(),
                host_context: HostContext::Terminal,
                pid,
                status: AgentStatus::Running,
            },
        },
    )
    .expect("report session-1 running");
    drop(reporter);

    // A TUI client connects and subscribes before the session replacement,
    // exactly the scenario that used to leave a frozen duplicate behind.
    let (tx, rx) = mpsc::channel();
    spawn_client(path.clone(), tx);

    let mut app = App::new();
    match rx.recv_timeout(Duration::from_secs(2)) {
        Ok(ClientEvent::Snapshot(agents, test_runs, _logs)) => app.apply_snapshot(agents, test_runs),
        other => panic!("expected an initial snapshot, got {other:?}"),
    }

    // session-2 reuses the same pid (e.g. `/clear`), superseding session-1
    // per the registry's same-pid dedup rule.
    let mut reporter = UnixStream::connect(&path).expect("connect as reporter");
    write_message(
        &mut reporter,
        &ClientMessage::ReportEvent {
            event: AgentEvent {
                session_id: SessionId("session-2".to_string()),
                cwd: cwd.clone(),
                host_context: HostContext::Terminal,
                pid,
                status: AgentStatus::Running,
            },
        },
    )
    .expect("report session-2 running");
    drop(reporter);

    // Apply whatever events arrive - a LogAppended for the new run's
    // "started" entry, an AgentUpdate for session-2, and an AgentRemoved for
    // session-1, in any order - until the client has seen the removal.
    let mut saw_removal = false;
    for _ in 0..10 {
        match rx.recv_timeout(Duration::from_secs(2)) {
            Ok(ClientEvent::Update(agent)) => app.apply_update(agent),
            Ok(ClientEvent::LogAppended(entry)) => app.apply_log_appended(entry),
            Ok(ClientEvent::AgentRemoved(session_id)) => {
                app.remove_agent(&session_id);
                saw_removal = true;
                break;
            }
            other => panic!("expected an Update, LogAppended, or AgentRemoved event, got {other:?}"),
        }
    }
    assert!(saw_removal, "expected an AgentRemoved event for session-1");

    assert_eq!(
        app.directory_groups().iter().map(|g| g.agents.len()).sum::<usize>(),
        1,
        "session-1 must be dropped once session-2 takes over its pid"
    );

    app.open_details_modal();
    let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
    term.draw(|frame| { render(frame, &app); }).unwrap();
    let text = buffer_text(&term);

    // Both the modal's Agents pane and its Logs pane mention this pid (the
    // latter from "agent started" entries, which aren't part of this bug),
    // so scope the count to lines that also carry the "running" status
    // emoji (🔧) unique to a live agent's status cell - the Agents pane row
    // this fix must collapse to exactly one.
    let running_pid_rows = text
        .lines()
        .filter(|line| line.contains(&format!("pid {pid}")) && line.contains('🔧'))
        .count();
    assert_eq!(
        running_pid_rows, 1,
        "expected exactly one running-agent row for pid {pid} in the Agents pane, got:\n{text}"
    );
}
