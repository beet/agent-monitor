use agentmon_proto::AgentInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    /// The connection dropped after being established and a reconnect
    /// attempt is in progress; the last-known agent list is kept on screen.
    Reconnecting,
    Unreachable(String),
}

/// Pure application state: what's on screen, decoupled from the terminal
/// and the socket connection so it can be tested without either.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    pub connection: ConnectionStatus,
    pub agents: Vec<AgentInfo>,
}

impl App {
    pub fn new() -> Self {
        App {
            connection: ConnectionStatus::Connecting,
            agents: Vec::new(),
        }
    }

    pub fn set_unreachable(&mut self, reason: String) {
        self.connection = ConnectionStatus::Unreachable(reason);
    }

    /// Marks the connection as actively retrying, keeping whatever agents
    /// are already on screen so the list doesn't flash empty mid-reconnect.
    pub fn set_reconnecting(&mut self) {
        self.connection = ConnectionStatus::Reconnecting;
    }

    pub fn apply_snapshot(&mut self, agents: Vec<AgentInfo>) {
        self.connection = ConnectionStatus::Connected;
        self.agents = agents;
    }

    /// Inserts a newly-seen agent, or updates one already shown - covers
    /// ordinary status changes as well as an agent being marked stale.
    pub fn apply_update(&mut self, agent: AgentInfo) {
        self.connection = ConnectionStatus::Connected;
        match self.agents.iter_mut().find(|a| a.session_id == agent.session_id) {
            Some(existing) => *existing = agent,
            None => self.agents.push(agent),
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentmon_proto::{AgentStatus, HostContext, SessionId};
    use std::path::PathBuf;

    fn agent(id: &str, status: AgentStatus) -> AgentInfo {
        AgentInfo {
            session_id: SessionId(id.to_string()),
            cwd: PathBuf::from("/tmp/project"),
            host_context: HostContext::Terminal,
            pid: 1,
            status,
            last_updated_ms: 0,
        }
    }

    #[test]
    fn apply_snapshot_replaces_the_agent_list_and_marks_connected() {
        let mut app = App::new();

        app.apply_snapshot(vec![agent("a", AgentStatus::Running)]);

        assert_eq!(app.connection, ConnectionStatus::Connected);
        assert_eq!(app.agents.len(), 1);
    }

    #[test]
    fn apply_update_adds_a_new_agent() {
        let mut app = App::new();

        app.apply_update(agent("a", AgentStatus::Running));

        assert_eq!(app.agents.len(), 1);
        assert_eq!(app.agents[0].status, AgentStatus::Running);
    }

    #[test]
    fn apply_update_updates_an_existing_agent_in_place() {
        let mut app = App::new();
        app.apply_update(agent("a", AgentStatus::Running));

        app.apply_update(agent("a", AgentStatus::Done));

        assert_eq!(app.agents.len(), 1, "must update in place, not duplicate");
        assert_eq!(app.agents[0].status, AgentStatus::Done);
    }

    #[test]
    fn apply_update_marking_an_agent_stale_is_reflected() {
        let mut app = App::new();
        app.apply_update(agent("a", AgentStatus::Running));

        app.apply_update(agent("a", AgentStatus::Stale));

        assert_eq!(app.agents[0].status, AgentStatus::Stale);
    }

}
