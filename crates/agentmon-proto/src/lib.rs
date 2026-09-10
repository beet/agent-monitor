use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub mod framing;
pub use framing::{read_message, write_message};

/// Default per-user socket path: `~/Library/Application Support/agentmon/agentd.sock`.
///
/// Shared by the daemon (binds it) and every client (the TUI, the hook
/// reporter) so they agree on where to find each other without one
/// depending on the other's crate.
pub fn default_socket_path() -> PathBuf {
    let home = std::env::var_os("HOME").expect("HOME environment variable must be set");
    PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("agentmon")
        .join("agentd.sock")
}

/// Identifies a Claude Code session across hook events and client updates.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

/// Where a Claude Code session is hosted, per proposal.md's "identity" decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostContext {
    Nvim,
    Terminal,
    Desktop,
}

/// Lifecycle status of a tracked agent, per the agent-daemon and
/// agent-monitor-tui specs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Running,
    Idle,
    NeedsInput,
    Done,
    Stale,
    Declined,
}

/// Lifecycle status of a tracked test run, per the rspec-test-reporting and
/// agent-daemon specs. Distinct from `AgentStatus`: a test run has no
/// "declined" or "stale" concept - its last reported status simply stands
/// until superseded by the next run at the same working directory and pid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestRunStatus {
    Started,
    Passed,
    Failed,
}

/// The daemon's view of a tracked test run, as sent to clients. Identified by
/// `(cwd, pid)`, not by any Claude Code session id - a test run may be
/// launched outside any agent's process tree (see rspec-test-reporting spec).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestRunInfo {
    pub cwd: PathBuf,
    pub pid: u32,
    pub status: TestRunStatus,
    /// Unix epoch milliseconds of the last update to this test run.
    pub last_updated_ms: u64,
    /// Unix epoch milliseconds of when this run (the tracked process id)
    /// started - unlike `last_updated_ms`, this stays fixed across the
    /// started -> passed/failed lifecycle of the same process, and only
    /// resets when a different pid reports for the same directory.
    pub run_started_ms: u64,
}

/// A status event reported by a Claude Code hook to the daemon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentEvent {
    pub session_id: SessionId,
    pub cwd: PathBuf,
    pub host_context: HostContext,
    pub pid: u32,
    pub status: AgentStatus,
}

/// The daemon's view of a tracked agent, as sent to clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentInfo {
    pub session_id: SessionId,
    pub cwd: PathBuf,
    pub host_context: HostContext,
    pub pid: u32,
    pub status: AgentStatus,
    /// Unix epoch milliseconds of the last update to this agent.
    pub last_updated_ms: u64,
    /// Unix epoch milliseconds of when this agent most recently entered
    /// its current status, unlike `last_updated_ms` which also bumps on
    /// same-status events (e.g. each tool call while running).
    pub status_since_ms: u64,
}

/// The first message a connection sends, telling the daemon whether it is a
/// short-lived hook event report or a long-lived subscriber (e.g. the TUI).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    ReportEvent { event: AgentEvent },
    /// Reported by a test formatter (e.g. RSpec), not a Claude Code hook -
    /// carries no session id, since a test run has no reliable way to learn
    /// one (see rspec-test-reporting spec).
    ReportTestRun {
        cwd: PathBuf,
        pid: u32,
        status: TestRunStatus,
    },
    Subscribe,
}

/// A message sent from the daemon to a connected client (e.g. the TUI).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Full current state, sent once when a client connects.
    Snapshot {
        agents: Vec<AgentInfo>,
        test_runs: Vec<TestRunInfo>,
    },
    /// An incremental update to a single agent's state.
    AgentUpdate { agent: AgentInfo },
    /// An incremental update to a single test run's state.
    TestRunUpdate { test_run: TestRunInfo },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_test_run() -> TestRunInfo {
        TestRunInfo {
            cwd: PathBuf::from("/Users/beet/Documents/Projects/enclaudinate"),
            pid: 5150,
            status: TestRunStatus::Started,
            last_updated_ms: 1_700_000_000_000,
            run_started_ms: 1_700_000_000_000,
        }
    }

    fn sample_agent() -> AgentInfo {
        AgentInfo {
            session_id: SessionId("session-123".to_string()),
            cwd: PathBuf::from("/Users/beet/Documents/Projects/enclaudinate"),
            host_context: HostContext::Nvim,
            pid: 4242,
            status: AgentStatus::Running,
            last_updated_ms: 1_700_000_000_000,
            status_since_ms: 1_700_000_000_000,
        }
    }

    #[test]
    fn agent_event_round_trips_through_json() {
        let event = AgentEvent {
            session_id: SessionId("session-123".to_string()),
            cwd: PathBuf::from("/tmp/project"),
            host_context: HostContext::Terminal,
            pid: 99,
            status: AgentStatus::NeedsInput,
        };

        let json = serde_json::to_string(&event).unwrap();
        let decoded: AgentEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(event, decoded);
    }

    #[test]
    fn agent_info_round_trips_through_json() {
        let info = sample_agent();

        let json = serde_json::to_string(&info).unwrap();
        let decoded: AgentInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(info, decoded);
        assert!(
            json.contains("\"status_since_ms\":1700000000000"),
            "expected status_since_ms field in JSON, got: {json}"
        );
    }

    #[test]
    fn test_run_info_round_trips_through_json() {
        let info = sample_test_run();

        let json = serde_json::to_string(&info).unwrap();
        let decoded: TestRunInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(info, decoded);
        assert!(
            json.contains("\"run_started_ms\":1700000000000"),
            "expected run_started_ms field in JSON, got: {json}"
        );
    }

    #[test]
    fn server_message_snapshot_round_trips_through_json() {
        let message = ServerMessage::Snapshot {
            agents: vec![sample_agent()],
            test_runs: vec![sample_test_run()],
        };

        let json = serde_json::to_string(&message).unwrap();
        let decoded: ServerMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(message, decoded);
    }

    #[test]
    fn client_message_report_test_run_round_trips_through_json() {
        let message = ClientMessage::ReportTestRun {
            cwd: PathBuf::from("/tmp/project"),
            pid: 321,
            status: TestRunStatus::Failed,
        };

        let json = serde_json::to_string(&message).unwrap();
        let decoded: ClientMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(message, decoded);
    }

    #[test]
    fn server_message_test_run_update_round_trips_through_json() {
        let mut test_run = sample_test_run();
        test_run.status = TestRunStatus::Passed;
        let message = ServerMessage::TestRunUpdate { test_run };

        let json = serde_json::to_string(&message).unwrap();
        let decoded: ServerMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(message, decoded);
    }

    #[test]
    fn test_run_status_serializes_as_snake_case() {
        assert_eq!(
            serde_json::to_string(&TestRunStatus::Started).unwrap(),
            "\"started\""
        );
        assert_eq!(
            serde_json::to_string(&TestRunStatus::Passed).unwrap(),
            "\"passed\""
        );
        assert_eq!(
            serde_json::to_string(&TestRunStatus::Failed).unwrap(),
            "\"failed\""
        );
    }

    #[test]
    fn client_message_report_event_round_trips_through_json() {
        let message = ClientMessage::ReportEvent {
            event: AgentEvent {
                session_id: SessionId("session-123".to_string()),
                cwd: PathBuf::from("/tmp/project"),
                host_context: HostContext::Desktop,
                pid: 7,
                status: AgentStatus::Idle,
            },
        };

        let json = serde_json::to_string(&message).unwrap();
        let decoded: ClientMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(message, decoded);
    }

    #[test]
    fn client_message_subscribe_round_trips_through_json() {
        let message = ClientMessage::Subscribe;

        let json = serde_json::to_string(&message).unwrap();
        let decoded: ClientMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(message, decoded);
    }

    #[test]
    fn server_message_agent_update_round_trips_through_json() {
        let mut agent = sample_agent();
        agent.status = AgentStatus::Done;
        let message = ServerMessage::AgentUpdate { agent };

        let json = serde_json::to_string(&message).unwrap();
        let decoded: ServerMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(message, decoded);
    }

    #[test]
    fn host_context_and_status_serialize_as_snake_case() {
        assert_eq!(
            serde_json::to_string(&HostContext::Desktop).unwrap(),
            "\"desktop\""
        );
        assert_eq!(
            serde_json::to_string(&AgentStatus::NeedsInput).unwrap(),
            "\"needs_input\""
        );
        assert_eq!(
            serde_json::to_string(&AgentStatus::Declined).unwrap(),
            "\"declined\""
        );
    }
}
