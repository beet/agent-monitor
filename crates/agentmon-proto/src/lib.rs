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
}

/// The first message a connection sends, telling the daemon whether it is a
/// short-lived hook event report or a long-lived subscriber (e.g. the TUI).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    ReportEvent { event: AgentEvent },
    Subscribe,
}

/// A message sent from the daemon to a connected client (e.g. the TUI).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Full current state, sent once when a client connects.
    Snapshot { agents: Vec<AgentInfo> },
    /// An incremental update to a single agent's state.
    AgentUpdate { agent: AgentInfo },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_agent() -> AgentInfo {
        AgentInfo {
            session_id: SessionId("session-123".to_string()),
            cwd: PathBuf::from("/Users/beet/Documents/Projects/enclaudinate"),
            host_context: HostContext::Nvim,
            pid: 4242,
            status: AgentStatus::Running,
            last_updated_ms: 1_700_000_000_000,
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
    }

    #[test]
    fn server_message_snapshot_round_trips_through_json() {
        let message = ServerMessage::Snapshot {
            agents: vec![sample_agent()],
        };

        let json = serde_json::to_string(&message).unwrap();
        let decoded: ServerMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(message, decoded);
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
    }
}
