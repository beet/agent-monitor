use std::collections::HashMap;
use std::path::{Path, PathBuf};

use agentmon_proto::{AgentInfo, LogEntry, SessionId, TestRunInfo};

/// Number of lines a `d`/`u` page-down/page-up moves the Logs tab's
/// selection by. See the "Paginated lists support keyboard navigation"
/// requirement in agent-monitor-tui's spec.
pub const LOGS_PAGE_SIZE: usize = 10;

/// Which tab is currently shown - see the "Tab navigation between Agents and
/// Logs" requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Agents,
    Logs,
}

/// How the Logs tab orders its entries - see "Logs tab supports sorting and
/// filtering".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogSort {
    Recency,
    Project,
    Status,
}

/// A modal overlay drawn on top of the active tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modal {
    /// The working directory of the project whose details are shown.
    Details(PathBuf),
    Help,
}

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
    /// order groups relative to one another and, in the TUI, as the
    /// project's rendered UPDATED time.
    pub fn most_recent_update_ms(&self) -> u64 {
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
    pub active_tab: Tab,
    pub agents_selected: usize,
    pub logs: Vec<LogEntry>,
    pub logs_selected: usize,
    pub logs_sort: LogSort,
    pub logs_filter_project: Option<String>,
    pub logs_filter_status: Option<String>,
    pub modal: Option<Modal>,
}

impl App {
    pub fn new() -> Self {
        App {
            connection: ConnectionStatus::Connecting,
            agents: Vec::new(),
            test_runs: Vec::new(),
            active_tab: Tab::Agents,
            agents_selected: 0,
            logs: Vec::new(),
            logs_selected: 0,
            logs_sort: LogSort::Recency,
            logs_filter_project: None,
            logs_filter_status: None,
            modal: None,
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
        self.clamp_agents_selected();
    }

    /// Inserts a newly-seen agent, or updates one already shown - covers
    /// ordinary status changes as well as an agent being marked stale.
    pub fn apply_update(&mut self, agent: AgentInfo) {
        self.connection = ConnectionStatus::Connected;
        match self.agents.iter_mut().find(|a| a.session_id == agent.session_id) {
            Some(existing) => *existing = agent,
            None => self.agents.push(agent),
        }
        self.clamp_agents_selected();
    }

    /// Drops a retired session id from the tracked agents, per the "Live
    /// agent list" requirement's handling of a daemon-pushed removal - e.g.
    /// a same-pid `/clear` superseding this session. A no-op if the session
    /// id isn't currently tracked (it may have already been replaced by a
    /// later update that arrived first).
    pub fn remove_agent(&mut self, session_id: &SessionId) {
        self.agents.retain(|a| &a.session_id != session_id);
        self.clamp_agents_selected();
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
        self.clamp_agents_selected();
    }

    /// Replaces the activity log with the daemon's current snapshot, per the
    /// activity-log spec's "Clients receive the current log and live
    /// updates" requirement.
    pub fn apply_log_snapshot(&mut self, logs: Vec<LogEntry>) {
        self.logs = logs;
        self.clamp_logs_selected();
    }

    /// Appends a newly-pushed activity log entry.
    pub fn apply_log_appended(&mut self, entry: LogEntry) {
        self.logs.push(entry);
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

    /// Cycles between the Agents and Logs tabs.
    pub fn cycle_tab(&mut self) {
        self.active_tab = match self.active_tab {
            Tab::Agents => Tab::Logs,
            Tab::Logs => Tab::Agents,
        };
    }

    pub fn set_tab(&mut self, tab: Tab) {
        self.active_tab = tab;
    }

    /// Moves the Agents tab's row selection by `delta`, clamped to the
    /// current number of project rows.
    pub fn move_agents_selection(&mut self, delta: isize) {
        let len = self.directory_groups().len();
        self.agents_selected = clamp_index(self.agents_selected, delta, len);
    }

    fn clamp_agents_selected(&mut self) {
        let len = self.directory_groups().len();
        self.agents_selected = clamp_index(self.agents_selected, 0, len);
    }

    /// Opens the details modal for the currently selected Agents-tab row, if
    /// any project rows are shown.
    /// Opens the details modal for whichever row is currently selected -
    /// the Agents tab's selected project, or the Logs tab's selected
    /// entry's project - so `Enter` reaches the same modal from either tab.
    pub fn open_details_modal(&mut self) {
        let cwd = match self.active_tab {
            Tab::Agents => self.directory_groups().get(self.agents_selected).map(|group| group.cwd.clone()),
            Tab::Logs => self.visible_logs().get(self.logs_selected).map(|entry| entry.working_dir.clone()),
        };
        if let Some(cwd) = cwd {
            self.modal = Some(Modal::Details(cwd));
        }
    }

    pub fn open_help_modal(&mut self) {
        self.modal = Some(Modal::Help);
    }

    pub fn close_modal(&mut self) {
        self.modal = None;
    }

    /// The activity log's entries after applying the current filter and
    /// sort, recomputed on demand rather than cached, since 500 entries is
    /// cheap to filter/sort on every render (see design.md).
    pub fn visible_logs(&self) -> Vec<&LogEntry> {
        let mut entries: Vec<&LogEntry> = self
            .logs
            .iter()
            .filter(|e| {
                self.logs_filter_project
                    .as_deref()
                    .is_none_or(|p| project_name(&e.working_dir) == p)
            })
            .filter(|e| self.logs_filter_status.as_deref().is_none_or(|s| e.status == s))
            .collect();

        match self.logs_sort {
            LogSort::Recency => entries.sort_by_key(|e| std::cmp::Reverse(e.occurred_at_ms)),
            LogSort::Project => entries.sort_by_key(|e| project_name(&e.working_dir)),
            LogSort::Status => entries.sort_by(|a, b| a.status.cmp(&b.status)),
        }

        entries
    }

    /// Moves the Logs tab's selection by `delta` lines, clamped to the
    /// current visible (filtered) entry count.
    pub fn move_logs_selection(&mut self, delta: isize) {
        let len = self.visible_logs().len();
        self.logs_selected = clamp_index(self.logs_selected, delta, len);
    }

    /// Moves the Logs tab's selection by a full page in `direction` (+1 or
    /// -1), per the "Paginated lists support keyboard navigation"
    /// requirement.
    pub fn page_logs(&mut self, direction: isize) {
        self.move_logs_selection(direction * LOGS_PAGE_SIZE as isize);
    }

    fn clamp_logs_selected(&mut self) {
        let len = self.visible_logs().len();
        self.logs_selected = clamp_index(self.logs_selected, 0, len);
    }

    pub fn cycle_logs_sort(&mut self) {
        self.logs_sort = match self.logs_sort {
            LogSort::Recency => LogSort::Project,
            LogSort::Project => LogSort::Status,
            LogSort::Status => LogSort::Recency,
        };
        self.logs_selected = 0;
    }

    /// Cycles the Project filter through every distinct project present in
    /// the full (unfiltered) log, in alphabetical order, then back to no
    /// filter.
    pub fn cycle_logs_project_filter(&mut self) {
        let mut projects: Vec<String> = self.logs.iter().map(|e| project_name(&e.working_dir)).collect();
        projects.sort();
        projects.dedup();
        self.logs_filter_project = next_in_cycle(self.logs_filter_project.as_deref(), &projects);
        self.clamp_logs_selected();
    }

    /// Cycles the Status filter through every distinct status present in the
    /// full (unfiltered) log, in alphabetical order, then back to no filter.
    pub fn cycle_logs_status_filter(&mut self) {
        let mut statuses: Vec<String> = self.logs.iter().map(|e| e.status.clone()).collect();
        statuses.sort();
        statuses.dedup();
        self.logs_filter_status = next_in_cycle(self.logs_filter_status.as_deref(), &statuses);
        self.clamp_logs_selected();
    }

    pub fn clear_logs_filters(&mut self) {
        self.logs_filter_project = None;
        self.logs_filter_status = None;
        self.clamp_logs_selected();
    }
}

/// Moves `current` by `delta`, clamped to `[0, len - 1]` (or `0` if `len` is
/// `0`). Shared by both the Agents-tab and Logs-tab selection cursors.
fn clamp_index(current: usize, delta: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (current as isize + delta).clamp(0, len as isize - 1) as usize
}

/// Advances `current` to the next entry in `options` (in order), or to `None`
/// once the last option has been passed - used to cycle a filter through
/// "every value, then off" with a single key.
fn next_in_cycle(current: Option<&str>, options: &[String]) -> Option<String> {
    match current {
        None => options.first().cloned(),
        Some(current) => match options.iter().position(|o| o == current) {
            Some(i) if i + 1 < options.len() => Some(options[i + 1].clone()),
            _ => None,
        },
    }
}

fn project_name(cwd: &Path) -> String {
    cwd.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| cwd.display().to_string())
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
            run_started_ms: 0,
        }
    }

    fn test_run_in(cwd: &str, pid: u32, status: agentmon_proto::TestRunStatus, last_updated_ms: u64) -> TestRunInfo {
        TestRunInfo {
            cwd: PathBuf::from(cwd),
            pid,
            status,
            last_updated_ms,
            run_started_ms: last_updated_ms,
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
    fn remove_agent_drops_a_retired_session_id() {
        let mut app = App::new();
        app.apply_update(agent("session-1", AgentStatus::Running));
        app.apply_update(agent("session-2", AgentStatus::Running));

        app.remove_agent(&SessionId("session-1".to_string()));

        assert_eq!(app.agents.len(), 1);
        assert_eq!(app.agents[0].session_id, SessionId("session-2".to_string()));
    }

    #[test]
    fn remove_agent_is_a_no_op_for_an_unknown_session_id() {
        let mut app = App::new();
        app.apply_update(agent("session-1", AgentStatus::Running));

        app.remove_agent(&SessionId("session-2".to_string()));

        assert_eq!(app.agents.len(), 1, "removing an untracked session id must not affect other agents");
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

    fn log_in(cwd: &str, status: &str, occurred_at_ms: u64) -> LogEntry {
        LogEntry {
            working_dir: PathBuf::from(cwd),
            category: agentmon_proto::LogCategory::Agent,
            status: status.to_string(),
            occurred_at_ms,
            pid: Some(1),
        }
    }

    #[test]
    fn starts_on_the_agents_tab() {
        assert_eq!(App::new().active_tab, Tab::Agents);
    }

    #[test]
    fn cycle_tab_toggles_between_agents_and_logs() {
        let mut app = App::new();
        app.cycle_tab();
        assert_eq!(app.active_tab, Tab::Logs);
        app.cycle_tab();
        assert_eq!(app.active_tab, Tab::Agents);
    }

    #[test]
    fn set_tab_jumps_directly_even_if_already_active() {
        let mut app = App::new();
        app.set_tab(Tab::Agents);
        assert_eq!(app.active_tab, Tab::Agents);
        app.set_tab(Tab::Logs);
        assert_eq!(app.active_tab, Tab::Logs);
    }

    #[test]
    fn agents_selection_moves_down_and_up_clamped() {
        let mut app = App::new();
        app.apply_snapshot(
            vec![
                agent_in("/tmp/a", "a", AgentStatus::Running),
                agent_in("/tmp/b", "b", AgentStatus::Running),
            ],
            Vec::new(),
        );

        assert_eq!(app.agents_selected, 0);
        app.move_agents_selection(1);
        assert_eq!(app.agents_selected, 1);
        app.move_agents_selection(1);
        assert_eq!(app.agents_selected, 1, "must clamp at the last row");
        app.move_agents_selection(-1);
        assert_eq!(app.agents_selected, 0);
        app.move_agents_selection(-1);
        assert_eq!(app.agents_selected, 0, "must clamp at the first row");
    }

    #[test]
    fn agents_selection_is_clamped_when_rows_shrink() {
        let mut app = App::new();
        app.apply_snapshot(
            vec![
                agent_in("/tmp/a", "a", AgentStatus::Running),
                agent_in("/tmp/b", "b", AgentStatus::Running),
            ],
            Vec::new(),
        );
        app.move_agents_selection(1);
        assert_eq!(app.agents_selected, 1);

        app.apply_snapshot(vec![agent_in("/tmp/a", "a", AgentStatus::Running)], Vec::new());

        assert_eq!(app.agents_selected, 0, "selection must clamp once a row disappears");
    }

    #[test]
    fn opening_details_modal_captures_the_selected_projects_cwd() {
        let mut app = App::new();
        // Distinct last_updated_ms values give directory_groups() a
        // deterministic order (most recent first) to select against.
        app.apply_snapshot(
            vec![
                AgentInfo {
                    last_updated_ms: 2_000,
                    ..agent_in("/tmp/a", "a", AgentStatus::Running)
                },
                AgentInfo {
                    last_updated_ms: 1_000,
                    ..agent_in("/tmp/b", "b", AgentStatus::Running)
                },
            ],
            Vec::new(),
        );
        app.move_agents_selection(1);

        app.open_details_modal();

        assert_eq!(app.modal, Some(Modal::Details(PathBuf::from("/tmp/b"))));
    }

    #[test]
    fn opening_details_modal_with_no_rows_does_nothing() {
        let mut app = App::new();

        app.open_details_modal();

        assert_eq!(app.modal, None);
    }

    #[test]
    fn opening_details_modal_on_the_logs_tab_uses_the_selected_entrys_project() {
        let mut app = App::new();
        app.set_tab(Tab::Logs);
        app.apply_log_snapshot(vec![log_in("/tmp/a", "done", 1), log_in("/tmp/b", "started", 2)]);
        app.move_logs_selection(1); // select "/tmp/a", the older (second) entry once sorted by recency

        app.open_details_modal();

        assert_eq!(app.modal, Some(Modal::Details(PathBuf::from("/tmp/a"))));
    }

    #[test]
    fn opening_details_modal_on_the_logs_tab_with_no_entries_does_nothing() {
        let mut app = App::new();
        app.set_tab(Tab::Logs);

        app.open_details_modal();

        assert_eq!(app.modal, None);
    }

    #[test]
    fn help_modal_opens_and_closes() {
        let mut app = App::new();

        app.open_help_modal();
        assert_eq!(app.modal, Some(Modal::Help));

        app.close_modal();
        assert_eq!(app.modal, None);
    }

    #[test]
    fn closing_the_modal_does_not_touch_agent_or_test_run_state() {
        let mut app = App::new();
        app.apply_snapshot(vec![agent_in("/tmp/a", "a", AgentStatus::Running)], Vec::new());
        app.open_details_modal();

        app.close_modal();

        assert_eq!(app.agents.len(), 1, "closing the modal must not affect tracked agents");
    }

    #[test]
    fn apply_log_snapshot_replaces_the_log() {
        let mut app = App::new();

        app.apply_log_snapshot(vec![log_in("/tmp/a", "done", 1)]);

        assert_eq!(app.logs.len(), 1);
    }

    #[test]
    fn apply_log_appended_adds_to_the_log() {
        let mut app = App::new();
        app.apply_log_snapshot(vec![log_in("/tmp/a", "done", 1)]);

        app.apply_log_appended(log_in("/tmp/b", "started", 2));

        assert_eq!(app.logs.len(), 2);
    }

    #[test]
    fn visible_logs_default_sorted_most_recent_first() {
        let mut app = App::new();
        app.apply_log_snapshot(vec![
            log_in("/tmp/a", "done", 1),
            log_in("/tmp/b", "started", 2),
        ]);

        let visible = app.visible_logs();

        assert_eq!(visible[0].working_dir, PathBuf::from("/tmp/b"));
        assert_eq!(visible[1].working_dir, PathBuf::from("/tmp/a"));
    }

    #[test]
    fn cycle_logs_sort_moves_through_recency_project_status() {
        let mut app = App::new();
        assert_eq!(app.logs_sort, LogSort::Recency);
        app.cycle_logs_sort();
        assert_eq!(app.logs_sort, LogSort::Project);
        app.cycle_logs_sort();
        assert_eq!(app.logs_sort, LogSort::Status);
        app.cycle_logs_sort();
        assert_eq!(app.logs_sort, LogSort::Recency);
    }

    #[test]
    fn sorting_by_project_orders_alphabetically() {
        let mut app = App::new();
        app.apply_log_snapshot(vec![
            log_in("/tmp/zeta", "done", 1),
            log_in("/tmp/alpha", "done", 2),
        ]);
        app.cycle_logs_sort();

        let visible = app.visible_logs();

        assert_eq!(visible[0].working_dir, PathBuf::from("/tmp/alpha"));
        assert_eq!(visible[1].working_dir, PathBuf::from("/tmp/zeta"));
    }

    #[test]
    fn sorting_by_status_orders_alphabetically() {
        let mut app = App::new();
        app.apply_log_snapshot(vec![
            log_in("/tmp/a", "started", 1),
            log_in("/tmp/b", "done", 2),
        ]);
        app.cycle_logs_sort();
        app.cycle_logs_sort();

        let visible = app.visible_logs();

        assert_eq!(visible[0].status, "done");
        assert_eq!(visible[1].status, "started");
    }

    #[test]
    fn cycle_logs_project_filter_cycles_through_projects_then_clears() {
        let mut app = App::new();
        app.apply_log_snapshot(vec![log_in("/tmp/a", "done", 1), log_in("/tmp/b", "done", 2)]);

        app.cycle_logs_project_filter();
        assert_eq!(app.logs_filter_project.as_deref(), Some("a"));
        assert_eq!(app.visible_logs().len(), 1);

        app.cycle_logs_project_filter();
        assert_eq!(app.logs_filter_project.as_deref(), Some("b"));

        app.cycle_logs_project_filter();
        assert_eq!(app.logs_filter_project, None, "must cycle back to no filter");
        assert_eq!(app.visible_logs().len(), 2);
    }

    #[test]
    fn cycle_logs_status_filter_cycles_through_statuses_then_clears() {
        let mut app = App::new();
        app.apply_log_snapshot(vec![
            log_in("/tmp/a", "done", 1),
            log_in("/tmp/a", "needs_input", 2),
        ]);

        app.cycle_logs_status_filter();
        assert_eq!(app.logs_filter_status.as_deref(), Some("done"));
        assert_eq!(app.visible_logs().len(), 1);

        app.cycle_logs_status_filter();
        assert_eq!(app.logs_filter_status.as_deref(), Some("needs_input"));

        app.cycle_logs_status_filter();
        assert_eq!(app.logs_filter_status, None);
    }

    #[test]
    fn clear_logs_filters_resets_both() {
        let mut app = App::new();
        app.apply_log_snapshot(vec![log_in("/tmp/a", "done", 1)]);
        app.cycle_logs_project_filter();
        app.cycle_logs_status_filter();

        app.clear_logs_filters();

        assert_eq!(app.logs_filter_project, None);
        assert_eq!(app.logs_filter_status, None);
    }

    #[test]
    fn move_logs_selection_moves_one_line_at_a_time_clamped() {
        let mut app = App::new();
        app.apply_log_snapshot(vec![log_in("/tmp/a", "done", 1), log_in("/tmp/b", "done", 2)]);

        app.move_logs_selection(1);
        assert_eq!(app.logs_selected, 1);
        app.move_logs_selection(1);
        assert_eq!(app.logs_selected, 1, "must clamp at the last entry");
        app.move_logs_selection(-1);
        assert_eq!(app.logs_selected, 0);
        app.move_logs_selection(-1);
        assert_eq!(app.logs_selected, 0, "must clamp at the first entry");
    }

    #[test]
    fn page_logs_moves_by_a_full_page_clamped() {
        let mut app = App::new();
        let entries: Vec<LogEntry> = (0..(LOGS_PAGE_SIZE * 3) as u64)
            .map(|i| log_in("/tmp/a", "done", i))
            .collect();
        app.apply_log_snapshot(entries);

        app.page_logs(1);
        assert_eq!(app.logs_selected, LOGS_PAGE_SIZE);
        app.page_logs(1);
        assert_eq!(app.logs_selected, LOGS_PAGE_SIZE * 2);
        app.page_logs(1);
        assert_eq!(
            app.logs_selected,
            LOGS_PAGE_SIZE * 3 - 1,
            "must clamp at the last entry rather than overshoot"
        );
        app.page_logs(-1);
        assert_eq!(app.logs_selected, LOGS_PAGE_SIZE * 2 - 1);
    }

    #[test]
    fn logs_selection_is_clamped_when_a_filter_shrinks_the_visible_list() {
        let mut app = App::new();
        app.apply_log_snapshot(vec![log_in("/tmp/a", "done", 1), log_in("/tmp/b", "done", 2)]);
        app.move_logs_selection(1);
        assert_eq!(app.logs_selected, 1);

        app.cycle_logs_project_filter();

        assert_eq!(app.logs_selected, 0, "selection must clamp once the filtered list shrinks");
    }
}
