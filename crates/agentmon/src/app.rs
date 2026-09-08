use std::collections::HashMap;
use std::path::PathBuf;

use agentmon_proto::{AgentInfo, TestRunInfo};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    /// The connection dropped after being established and a reconnect
    /// attempt is in progress; the last-known agent list is kept on screen.
    Reconnecting,
    Unreachable(String),
}

/// Every tracked agent and test run sharing one exact working directory,
/// ready for display - see the "Live agent list" requirement in
/// agent-monitor-tui's spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryGroup {
    pub cwd: PathBuf,
    pub agents: Vec<AgentInfo>,
    pub test_runs: Vec<TestRunInfo>,
}

impl DirectoryGroup {
    /// The most recent `last_updated_ms` among this group's members, used to
    /// order groups relative to one another.
    fn most_recent_update_ms(&self) -> u64 {
        let agents_max = self.agents.iter().map(|a| a.last_updated_ms).max();
        let test_runs_max = self.test_runs.iter().map(|t| t.last_updated_ms).max();
        agents_max.into_iter().chain(test_runs_max).max().unwrap_or(0)
    }
}

/// Pure application state: what's on screen, decoupled from the terminal
/// and the socket connection so it can be tested without either.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    pub connection: ConnectionStatus,
    pub agents: Vec<AgentInfo>,
    pub test_runs: Vec<TestRunInfo>,
}

impl App {
    pub fn new() -> Self {
        App {
            connection: ConnectionStatus::Connecting,
            agents: Vec::new(),
            test_runs: Vec::new(),
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

    pub fn apply_snapshot(&mut self, agents: Vec<AgentInfo>, test_runs: Vec<TestRunInfo>) {
        self.connection = ConnectionStatus::Connected;
        self.agents = agents;
        self.test_runs = test_runs;
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

    /// Inserts a newly-seen test run, or updates one already shown, keyed by
    /// `(cwd, pid)` - the same identity `agentd` uses.
    pub fn apply_test_run_update(&mut self, test_run: TestRunInfo) {
        self.connection = ConnectionStatus::Connected;
        match self
            .test_runs
            .iter_mut()
            .find(|t| t.cwd == test_run.cwd && t.pid == test_run.pid)
        {
            Some(existing) => *existing = test_run,
            None => self.test_runs.push(test_run),
        }
    }

    /// Groups tracked agents and test runs by exact working-directory match,
    /// ordered most-recently-updated group first. Within a group, agents are
    /// listed before test runs, each ordered by their own last-updated time,
    /// most recent first.
    pub fn directory_groups(&self) -> Vec<DirectoryGroup> {
        let mut groups: HashMap<PathBuf, DirectoryGroup> = HashMap::new();

        for agent in &self.agents {
            groups
                .entry(agent.cwd.clone())
                .or_insert_with(|| DirectoryGroup {
                    cwd: agent.cwd.clone(),
                    agents: Vec::new(),
                    test_runs: Vec::new(),
                })
                .agents
                .push(agent.clone());
        }

        for test_run in &self.test_runs {
            groups
                .entry(test_run.cwd.clone())
                .or_insert_with(|| DirectoryGroup {
                    cwd: test_run.cwd.clone(),
                    agents: Vec::new(),
                    test_runs: Vec::new(),
                })
                .test_runs
                .push(test_run.clone());
        }

        let mut groups: Vec<DirectoryGroup> = groups.into_values().collect();
        for group in &mut groups {
            group.agents.sort_by_key(|a| std::cmp::Reverse(a.last_updated_ms));
            group.test_runs.sort_by_key(|t| std::cmp::Reverse(t.last_updated_ms));
        }
        groups.sort_by_key(|g| std::cmp::Reverse(g.most_recent_update_ms()));
        groups
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
        agent_in("/tmp/project", id, status)
    }

    fn agent_in(cwd: &str, id: &str, status: AgentStatus) -> AgentInfo {
        AgentInfo {
            session_id: SessionId(id.to_string()),
            cwd: PathBuf::from(cwd),
            host_context: HostContext::Terminal,
            pid: 1,
            status,
            last_updated_ms: 0,
            status_since_ms: 0,
        }
    }

    fn test_run_in(cwd: &str, pid: u32, status: agentmon_proto::TestRunStatus, last_updated_ms: u64) -> TestRunInfo {
        TestRunInfo {
            cwd: PathBuf::from(cwd),
            pid,
            status,
            last_updated_ms,
        }
    }

    #[test]
    fn apply_snapshot_replaces_the_agent_list_and_marks_connected() {
        let mut app = App::new();

        app.apply_snapshot(vec![agent("a", AgentStatus::Running)], Vec::new());

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

    #[test]
    fn apply_test_run_update_adds_a_new_test_run() {
        let mut app = App::new();

        app.apply_test_run_update(test_run_in("/tmp/project", 1, agentmon_proto::TestRunStatus::Started, 0));

        assert_eq!(app.test_runs.len(), 1);
    }

    #[test]
    fn apply_test_run_update_updates_the_same_cwd_and_pid_in_place() {
        let mut app = App::new();
        app.apply_test_run_update(test_run_in("/tmp/project", 1, agentmon_proto::TestRunStatus::Started, 0));

        app.apply_test_run_update(test_run_in("/tmp/project", 1, agentmon_proto::TestRunStatus::Failed, 100));

        assert_eq!(app.test_runs.len(), 1, "must update in place, not duplicate");
        assert_eq!(app.test_runs[0].status, agentmon_proto::TestRunStatus::Failed);
    }

    #[test]
    fn directory_groups_groups_two_agents_sharing_a_cwd() {
        let mut app = App::new();
        app.apply_snapshot(
            vec![
                agent_in("/tmp/project", "a", AgentStatus::Running),
                agent_in("/tmp/project", "b", AgentStatus::Running),
            ],
            Vec::new(),
        );

        let groups = app.directory_groups();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].agents.len(), 2);
    }

    #[test]
    fn directory_groups_includes_a_test_run_only_group() {
        let mut app = App::new();
        app.apply_test_run_update(test_run_in("/tmp/project", 1, agentmon_proto::TestRunStatus::Started, 0));

        let groups = app.directory_groups();

        assert_eq!(groups.len(), 1);
        assert!(groups[0].agents.is_empty());
        assert_eq!(groups[0].test_runs.len(), 1);
    }

    #[test]
    fn directory_groups_are_sorted_by_most_recent_member() {
        let mut app = App::new();
        app.apply_snapshot(
            vec![AgentInfo {
                last_updated_ms: 1_000,
                ..agent_in("/tmp/older", "a", AgentStatus::Running)
            }],
            vec![test_run_in("/tmp/newer", 1, agentmon_proto::TestRunStatus::Started, 2_000)],
        );

        let groups = app.directory_groups();

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].cwd, PathBuf::from("/tmp/newer"));
        assert_eq!(groups[1].cwd, PathBuf::from("/tmp/older"));
    }

    #[test]
    fn directory_groups_orders_agents_before_test_runs_within_a_group() {
        let mut app = App::new();
        app.apply_snapshot(
            vec![AgentInfo {
                last_updated_ms: 1_000,
                ..agent_in("/tmp/project", "a", AgentStatus::Running)
            }],
            vec![test_run_in("/tmp/project", 1, agentmon_proto::TestRunStatus::Started, 2_000)],
        );

        let groups = app.directory_groups();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].agents.len(), 1);
        assert_eq!(groups[0].test_runs.len(), 1);
    }
}
