use std::collections::HashMap;
use std::path::{Path, PathBuf};

use agentmon_proto::{
    AgentInfo, ClientMessage, LogEntry, ReminderId, ReminderInfo, ReminderStatus, SessionId, TestRunInfo,
};

/// A paginated list's selection and scroll-window state - shared by the Logs
/// tab and the details modal's Logs pane so their `j`/`k`/`d`/`u` behavior
/// stays identical by construction. See the "Paginated lists support
/// keyboard navigation" requirement.
///
/// `page_size` (how many rows currently fit in the list's rendered area) is
/// supplied by callers rather than stored here, since it comes from the
/// terminal's actual rendered height and `App` stays terminal-independent -
/// see design.md's "`page_size` is a parameter" decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Paginator {
    pub selected: usize,
    pub top: usize,
}

impl Paginator {
    /// Moves the selection by `delta` lines, clamped to `[0, len - 1]`, and
    /// scrolls the window by the minimum amount needed to keep the new
    /// selection visible.
    pub fn move_by(&mut self, delta: isize, len: usize, page_size: usize) {
        self.selected = clamp_index(self.selected, delta, len);
        self.top = synced_top(self.top, self.selected, len, page_size);
    }

    /// Moves by a full page in `direction` (+1 down, -1 up), overlapping the
    /// previous page by exactly 1 row: the window's top row advances by
    /// `page_size - 1` rows and the selection snaps to that new top row,
    /// clamped so the window never scrolls past the list's first or last
    /// entry. A no-op if there's nothing to page (`len` or `page_size` is 0).
    pub fn page(&mut self, direction: isize, len: usize, page_size: usize) {
        if page_size == 0 || len == 0 {
            return;
        }
        let step = page_size.saturating_sub(1).max(1) as isize;
        let max_top = len.saturating_sub(page_size) as isize;
        let new_top = (self.top as isize + direction * step).clamp(0, max_top) as usize;
        self.top = new_top;
        self.selected = new_top;
    }

    /// Clamps the selection to `[0, len - 1]` - e.g. after a filter shrinks
    /// the visible list. Leaves `top` as-is; it self-heals on the next
    /// render via `display_top`, and on the next `move_by`/`page` call.
    pub fn clamp_selected(&mut self, len: usize) {
        self.selected = clamp_index(self.selected, 0, len);
    }

    /// Jumps to the list's first entry, scrolling the page so that entry is
    /// the top row shown - the `g` key, per the "Paginated lists support
    /// keyboard navigation" requirement. A no-op if `len` is 0.
    pub fn jump_to_start(&mut self, len: usize, _page_size: usize) {
        if len == 0 {
            return;
        }
        self.selected = 0;
        self.top = 0;
    }

    /// Jumps to the list's last entry, scrolling so the last page is
    /// showing - the `G` key. Uses the same `len.saturating_sub(page_size)`
    /// clamp `page` uses for its `max_top`, so the window never scrolls past
    /// the list's last entry. A no-op if `len` is 0.
    pub fn jump_to_end(&mut self, len: usize, page_size: usize) {
        if len == 0 {
            return;
        }
        self.selected = len - 1;
        self.top = len.saturating_sub(page_size);
    }

    /// Resets to the top of the list - used when the details modal (re)opens
    /// so its Logs pane always starts unscrolled, matching today's behavior.
    pub fn reset(&mut self) {
        self.selected = 0;
        self.top = 0;
    }

    /// The top-of-window row to render this frame. Non-mutating and
    /// recomputed fresh from the current `len`/`page_size` rather than
    /// trusting the stored `top`, so a stale window (e.g. right after a
    /// terminal resize changes `page_size`, before the next key press)
    /// self-heals for display instead of showing a truncated page.
    pub fn display_top(&self, len: usize, page_size: usize) -> usize {
        synced_top(self.top, self.display_selected(len), len, page_size)
    }

    /// The selection to render this frame, defensively clamped to `len` the
    /// same way `TableState`'s selection is clamped elsewhere in `ui.rs`.
    pub fn display_selected(&self, len: usize) -> usize {
        if len == 0 {
            0
        } else {
            self.selected.min(len - 1)
        }
    }
}

/// Scrolls `top` by the minimum amount needed to keep `selected` within
/// `[top, top + page_size)`, then clamps it so the window never runs past
/// the end of the list. Shared by `Paginator::move_by` (mutating) and
/// `Paginator::display_top` (a pure recomputation for rendering).
fn synced_top(top: usize, selected: usize, len: usize, page_size: usize) -> usize {
    if page_size == 0 {
        return 0;
    }
    let mut top = top;
    if selected < top {
        top = selected;
    } else if selected >= top + page_size {
        top = selected + 1 - page_size;
    }
    top.min(len.saturating_sub(page_size))
}

/// Whether a paginated list needs pagination controls (scrollbar, heading
/// hint) at all - true only once it holds more rows than fit on one page.
pub fn needs_pagination(len: usize, page_size: usize) -> bool {
    page_size > 0 && len > page_size
}

/// How many rows each paginated list actually rendered on the most recent
/// frame, as computed by `ui.rs` from the pane's real inner height. Threaded
/// into `handle_key` so paging math reflects what's really on screen rather
/// than a fixed guess - see design.md's "`page_size` is a parameter"
/// decision.
#[derive(Debug, Clone, Copy, Default)]
pub struct PageSizes {
    pub logs_tab: usize,
    pub modal_logs: usize,
    pub reminders_tab: usize,
    pub modal_reminders: usize,
}

/// Which tab is currently shown - see the "Tab navigation between Agents and
/// Logs" requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Agents,
    Logs,
    Reminders,
}

/// How the Logs tab orders its entries - see "Logs tab supports sorting and
/// filtering".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogSort {
    Recency,
    Project,
    Status,
}

/// How the Reminders tab orders its entries - see "Reminders tab supports
/// sorting and filtering". `Recency` uses each reminder's last *completed*
/// run (stop or finish), unlike the Updated column which also counts a bare
/// start - see `reminder_recency_key`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReminderSort {
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

/// Which of the details modal's two interactive panes (Logs, Reminders)
/// currently holds keyboard focus - see the "Project details modal"
/// requirement's `Tab`-toggled focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalFocus {
    Logs,
    Reminders,
}

/// Whether the open reminder form is creating a new reminder or editing an
/// existing one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReminderFormMode {
    Create,
    Edit(ReminderId),
}

/// Which field of the reminder form currently has input focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReminderFormField {
    Name,
    Duration,
}

/// The reminder creation/editing form's in-progress state - see "Reminder
/// creation and editing form". Only meaningful while the details modal is
/// open; `cwd` is the project the reminder belongs (or will belong) to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReminderForm {
    pub mode: ReminderFormMode,
    pub cwd: PathBuf,
    pub name: String,
    /// Raw digit-only text buffer; parsed to `u32` on submit.
    pub duration_minutes: String,
    pub field: ReminderFormField,
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
    /// This project's reminders, most recently updated first. Only attached
    /// to a group that already exists because of tracked agents or test
    /// runs - a project with reminders but neither does not get an Agents
    /// tab row of its own.
    pub reminders: Vec<ReminderInfo>,
}

impl DirectoryGroup {
    /// The most recent `last_updated_ms` among this group's members, used to
    /// order groups relative to one another and, in the TUI, as the
    /// project's rendered UPDATED time. Reminders count too: a reminder's
    /// `last_updated_ms` already reflects its most recent started/stopped/
    /// finished event (or creation time if it has none), the same "last
    /// activity" meaning `last_updated_ms` carries for agents and test runs.
    pub fn most_recent_update_ms(&self) -> u64 {
        let agents_max = self.agents.iter().map(|a| a.last_updated_ms).max();
        let test_runs_max = self.test_runs.iter().map(|t| t.last_updated_ms).max();
        let reminders_max = self.reminders.iter().map(|r| r.last_updated_ms).max();
        agents_max
            .into_iter()
            .chain(test_runs_max)
            .chain(reminders_max)
            .max()
            .unwrap_or(0)
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
    /// Whether the Agents tab's `/`-search prompt is currently being edited -
    /// see "Agents tab supports incremental search by project name".
    pub agents_search_editing: bool,
    /// The in-progress search text while `agents_search_editing` is true.
    /// Seeded from `agents_search_applied` when editing begins, and discarded
    /// (without touching `agents_search_applied`) if editing is cancelled.
    pub agents_search_buffer: String,
    /// The committed Agents-tab search filter, applied as a case-insensitive
    /// substring match against each row's project name.
    pub agents_search_applied: Option<String>,
    pub logs: Vec<LogEntry>,
    pub logs_pagination: Paginator,
    pub logs_sort: LogSort,
    pub logs_filter_project: Option<String>,
    pub logs_filter_status: Option<String>,
    pub modal: Option<Modal>,
    /// The details modal's Logs pane's own selection/scroll state, separate
    /// from the Logs tab's - reset whenever `open_details_modal` runs.
    pub modal_logs_pagination: Paginator,
    pub reminders: Vec<ReminderInfo>,
    pub reminders_pagination: Paginator,
    pub reminders_sort: ReminderSort,
    pub reminders_filter_project: Option<String>,
    pub reminders_filter_status: Option<String>,
    /// Which of the details modal's Logs/Reminders panes currently holds
    /// keyboard focus.
    pub modal_focus: ModalFocus,
    /// The details modal's Reminders pane's own selection/scroll state,
    /// separate from the Reminders tab's - reset whenever
    /// `open_details_modal` runs.
    pub modal_reminders_pagination: Paginator,
    /// The reminder creation/editing form, when open.
    pub reminder_form: Option<ReminderForm>,
    /// The reminder the delete-confirmation dialog is asking about, when
    /// open.
    pub confirm_delete_reminder: Option<ReminderId>,
    /// Set when a `CreateReminder` command has been sent but the daemon
    /// hasn't yet confirmed it with a matching `ReminderUpdate` - the
    /// daemon assigns the id, so this is how `apply_reminder_update`
    /// recognizes "this is the one I just created" to highlight and
    /// auto-start it. Cleared once matched, or overwritten by a later
    /// create before this one is confirmed.
    pending_new_reminder: Option<(PathBuf, String)>,
}

impl App {
    pub fn new() -> Self {
        App {
            connection: ConnectionStatus::Connecting,
            agents: Vec::new(),
            test_runs: Vec::new(),
            active_tab: Tab::Agents,
            agents_selected: 0,
            agents_search_editing: false,
            agents_search_buffer: String::new(),
            agents_search_applied: None,
            logs: Vec::new(),
            logs_pagination: Paginator::default(),
            logs_sort: LogSort::Recency,
            logs_filter_project: None,
            logs_filter_status: None,
            modal: None,
            modal_logs_pagination: Paginator::default(),
            reminders: Vec::new(),
            reminders_pagination: Paginator::default(),
            reminders_sort: ReminderSort::Recency,
            reminders_filter_project: None,
            reminders_filter_status: None,
            modal_focus: ModalFocus::Logs,
            modal_reminders_pagination: Paginator::default(),
            reminder_form: None,
            confirm_delete_reminder: None,
            pending_new_reminder: None,
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
                    reminders: Vec::new(),
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
                    reminders: Vec::new(),
                })
                .test_runs
                .push(test_run.clone());
        }

        // Attaches to an already-existing group only - a project with
        // reminders but no tracked agent or test run does not get an Agents
        // tab row of its own, per the "Agents tab shows a Reminders column"
        // requirement.
        for reminder in &self.reminders {
            if let Some(group) = groups.get_mut(&reminder.cwd) {
                group.reminders.push(reminder.clone());
            }
        }

        let mut groups: Vec<DirectoryGroup> = groups.into_values().collect();
        for group in &mut groups {
            group.agents.sort_by_key(|a| std::cmp::Reverse(a.last_updated_ms));
            group.test_runs.sort_by_key(|t| std::cmp::Reverse(t.last_updated_ms));
            group.reminders.sort_by_key(|r| std::cmp::Reverse(r.last_updated_ms));
        }
        groups.sort_by_key(|g| std::cmp::Reverse(g.most_recent_update_ms()));
        groups
    }

    /// Cycles through the Agents, Logs, and Reminders tabs in order.
    pub fn cycle_tab(&mut self) {
        self.active_tab = match self.active_tab {
            Tab::Agents => Tab::Logs,
            Tab::Logs => Tab::Reminders,
            Tab::Reminders => Tab::Agents,
        };
    }

    pub fn set_tab(&mut self, tab: Tab) {
        self.active_tab = tab;
    }

    /// The Agents tab's project rows after applying the current search
    /// filter (a case-insensitive substring match against each row's project
    /// name), preserving `directory_groups`' relative order - see "Agents tab
    /// supports incremental search by project name". While the search prompt
    /// is being edited, filters live against the in-progress edit buffer
    /// rather than the last-committed filter, so every keystroke narrows the
    /// list immediately, per that requirement's "updates immediately after
    /// every such change" behavior.
    pub fn visible_agent_groups(&self) -> Vec<DirectoryGroup> {
        let groups = self.directory_groups();
        let filter = if self.agents_search_editing {
            Some(self.agents_search_buffer.as_str())
        } else {
            self.agents_search_applied.as_deref()
        };
        match filter {
            None => groups,
            Some(filter) => {
                let filter = filter.to_lowercase();
                groups
                    .into_iter()
                    .filter(|g| project_name(&g.cwd).to_lowercase().contains(&filter))
                    .collect()
            }
        }
    }

    /// Moves the Agents tab's row selection by `delta`, clamped to the
    /// current number of visible (filtered) project rows.
    pub fn move_agents_selection(&mut self, delta: isize) {
        let len = self.visible_agent_groups().len();
        self.agents_selected = clamp_index(self.agents_selected, delta, len);
    }

    fn clamp_agents_selected(&mut self) {
        let len = self.visible_agent_groups().len();
        self.agents_selected = clamp_index(self.agents_selected, 0, len);
    }

    /// Enters the Agents tab's `/`-search-editing mode, seeding the edit
    /// buffer from whatever filter is already applied so refining or
    /// clearing it doesn't require retyping it.
    pub fn open_agents_search(&mut self) {
        self.agents_search_buffer = self.agents_search_applied.clone().unwrap_or_default();
        self.agents_search_editing = true;
    }

    /// Appends a character to the in-progress search buffer - the Agents
    /// tab's `/`-search-editing mode.
    pub fn push_agents_search_char(&mut self, c: char) {
        self.agents_search_buffer.push(c);
    }

    /// Removes the last character of the in-progress search buffer - the
    /// Agents tab's `/`-search-editing mode.
    pub fn pop_agents_search_char(&mut self) {
        self.agents_search_buffer.pop();
    }

    /// Commits the in-progress search buffer as the Agents tab's applied
    /// filter (or clears it, if the buffer is empty) and exits
    /// search-editing mode.
    pub fn commit_agents_search(&mut self) {
        self.agents_search_applied = if self.agents_search_buffer.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut self.agents_search_buffer))
        };
        self.agents_search_editing = false;
        self.clamp_agents_selected();
    }

    /// Discards the in-progress search buffer and exits search-editing mode,
    /// leaving whatever filter was applied before editing began untouched.
    pub fn cancel_agents_search(&mut self) {
        self.agents_search_editing = false;
    }

    /// Opens the details modal for the currently selected Agents-tab row, if
    /// any project rows are shown.
    /// Opens the details modal for whichever row is currently selected -
    /// the Agents tab's selected project, or the Logs tab's selected
    /// entry's project - so `Enter` reaches the same modal from either tab.
    pub fn open_details_modal(&mut self) {
        let cwd = match self.active_tab {
            Tab::Agents => self.visible_agent_groups().get(self.agents_selected).map(|group| group.cwd.clone()),
            Tab::Logs => self
                .visible_logs()
                .get(self.logs_pagination.selected)
                .map(|entry| entry.working_dir.clone()),
            Tab::Reminders => self
                .visible_reminders()
                .get(self.reminders_pagination.selected)
                .map(|r| r.cwd.clone()),
        };
        let Some(cwd) = cwd else { return };

        self.modal_logs_pagination.reset();
        self.modal_reminders_pagination.reset();

        if self.active_tab == Tab::Reminders {
            self.modal_focus = ModalFocus::Reminders;
            // Highlights the reminder that was selected in the Reminders
            // tab within the modal's own Reminders pane, per "Opening a
            // project's details from the Reminders tab".
            if let Some(selected_id) = self
                .visible_reminders()
                .get(self.reminders_pagination.selected)
                .map(|r| r.id.clone())
            {
                if let Some(index) = self.project_reminders(&cwd).iter().position(|r| r.id == selected_id) {
                    self.modal_reminders_pagination.selected = index;
                }
            }
        } else {
            self.modal_focus = ModalFocus::Logs;
        }

        self.modal = Some(Modal::Details(cwd));
    }

    pub fn open_help_modal(&mut self) {
        self.modal = Some(Modal::Help);
    }

    pub fn close_modal(&mut self) {
        self.modal = None;
        self.reminder_form = None;
        self.confirm_delete_reminder = None;
    }

    /// Toggles which of the details modal's Logs/Reminders panes currently
    /// holds keyboard focus - the `Tab` key while the modal is open.
    pub fn toggle_modal_focus(&mut self) {
        self.modal_focus = match self.modal_focus {
            ModalFocus::Logs => ModalFocus::Reminders,
            ModalFocus::Reminders => ModalFocus::Logs,
        };
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

    /// A project's activity log entries, most recent first - used by the
    /// details modal's Logs pane. Unlike `visible_logs`, this ignores the
    /// Logs tab's own sort/filter state, per the "Project details modal"
    /// requirement.
    pub fn project_logs(&self, cwd: &Path) -> Vec<&LogEntry> {
        let mut entries: Vec<&LogEntry> = self.logs.iter().filter(|e| e.working_dir == cwd).collect();
        entries.sort_by_key(|e| std::cmp::Reverse(e.occurred_at_ms));
        entries
    }

    /// The number of activity log entries shown in the details modal's Logs
    /// pane for whichever project it's currently open on - 0 if no details
    /// modal is open.
    fn modal_logs_len(&self) -> usize {
        match &self.modal {
            Some(Modal::Details(cwd)) => self.project_logs(cwd).len(),
            _ => 0,
        }
    }

    /// Moves the Logs tab's selection by `delta` lines, clamped to the
    /// current visible (filtered) entry count. `page_size` is however many
    /// rows the Logs tab actually rendered on the most recent frame.
    pub fn move_logs_selection(&mut self, delta: isize, page_size: usize) {
        let len = self.visible_logs().len();
        self.logs_pagination.move_by(delta, len, page_size);
    }

    /// Moves the Logs tab's selection by a full page in `direction` (+1 or
    /// -1), overlapping the previous page by exactly 1 row, per the
    /// "Paginated lists support keyboard navigation" requirement.
    pub fn page_logs(&mut self, direction: isize, page_size: usize) {
        let len = self.visible_logs().len();
        self.logs_pagination.page(direction, len, page_size);
    }

    /// Moves the details modal's Logs pane's own selection by `delta`
    /// lines - see `move_logs_selection`, scoped to the modal's pane
    /// instead of the Logs tab.
    pub fn move_modal_logs_selection(&mut self, delta: isize, page_size: usize) {
        let len = self.modal_logs_len();
        self.modal_logs_pagination.move_by(delta, len, page_size);
    }

    /// Moves the details modal's Logs pane's own selection by a full page -
    /// see `page_logs`, scoped to the modal's pane instead of the Logs tab.
    pub fn page_modal_logs(&mut self, direction: isize, page_size: usize) {
        let len = self.modal_logs_len();
        self.modal_logs_pagination.page(direction, len, page_size);
    }

    /// Jumps the Logs tab's selection to the list's first entry - the `g`
    /// key, per the "Paginated lists support keyboard navigation"
    /// requirement.
    pub fn jump_logs_to_start(&mut self, page_size: usize) {
        let len = self.visible_logs().len();
        self.logs_pagination.jump_to_start(len, page_size);
    }

    /// Jumps the Logs tab's selection to the list's last entry - the `G`
    /// key.
    pub fn jump_logs_to_end(&mut self, page_size: usize) {
        let len = self.visible_logs().len();
        self.logs_pagination.jump_to_end(len, page_size);
    }

    /// Jumps the details modal's Logs pane's own selection to the list's
    /// first entry - see `jump_logs_to_start`, scoped to the modal's pane.
    pub fn jump_modal_logs_to_start(&mut self, page_size: usize) {
        let len = self.modal_logs_len();
        self.modal_logs_pagination.jump_to_start(len, page_size);
    }

    /// Jumps the details modal's Logs pane's own selection to the list's
    /// last entry - see `jump_logs_to_end`, scoped to the modal's pane.
    pub fn jump_modal_logs_to_end(&mut self, page_size: usize) {
        let len = self.modal_logs_len();
        self.modal_logs_pagination.jump_to_end(len, page_size);
    }

    fn clamp_logs_selected(&mut self) {
        let len = self.visible_logs().len();
        self.logs_pagination.clamp_selected(len);
    }

    pub fn cycle_logs_sort(&mut self) {
        self.logs_sort = match self.logs_sort {
            LogSort::Recency => LogSort::Project,
            LogSort::Project => LogSort::Status,
            LogSort::Status => LogSort::Recency,
        };
        self.logs_pagination.reset();
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

    /// Replaces the tracked reminders with the daemon's current snapshot.
    pub fn apply_reminder_snapshot(&mut self, reminders: Vec<ReminderInfo>) {
        self.reminders = reminders;
        self.clamp_reminders_selected();
    }

    /// Inserts a newly-created reminder, or updates one already shown -
    /// covers edits, starts, stops, and completions. Returns the reminder's
    /// id if this update is the daemon's confirmation of a reminder just
    /// created via the form (see `pending_new_reminder`) - the caller
    /// (main.rs) then sends a `StartReminder` for it, so creating a
    /// reminder highlights and starts it in one motion rather than
    /// requiring a separate `s` afterward.
    pub fn apply_reminder_update(&mut self, reminder: ReminderInfo) -> Option<ReminderId> {
        let is_new = !self.reminders.iter().any(|r| r.id == reminder.id);
        let just_created = is_new
            && self.pending_new_reminder.as_ref() == Some(&(reminder.cwd.clone(), reminder.name.clone()));

        match self.reminders.iter_mut().find(|r| r.id == reminder.id) {
            Some(existing) => *existing = reminder.clone(),
            None => self.reminders.push(reminder.clone()),
        }
        self.clamp_reminders_selected();

        if !just_created {
            return None;
        }
        self.pending_new_reminder = None;
        if let Some(Modal::Details(cwd)) = &self.modal {
            if cwd == &reminder.cwd {
                if let Some(index) = self.project_reminders(cwd).iter().position(|r| r.id == reminder.id) {
                    self.modal_reminders_pagination.selected = index;
                    self.modal_focus = ModalFocus::Reminders;
                }
            }
        }
        Some(reminder.id)
    }

    /// Drops a deleted reminder from the tracked list. A no-op if it isn't
    /// currently tracked.
    pub fn remove_reminder(&mut self, id: &ReminderId) {
        self.reminders.retain(|r| &r.id != id);
        self.clamp_reminders_selected();
    }

    /// The Reminders tab's entries after applying the current filter and
    /// sort, recomputed on demand - see `visible_logs`.
    pub fn visible_reminders(&self) -> Vec<&ReminderInfo> {
        let mut entries: Vec<&ReminderInfo> = self
            .reminders
            .iter()
            .filter(|r| {
                self.reminders_filter_project
                    .as_deref()
                    .is_none_or(|p| project_name(&r.cwd) == p)
            })
            .filter(|r| {
                self.reminders_filter_status
                    .as_deref()
                    .is_none_or(|s| reminder_status_word(r.status) == s)
            })
            .collect();

        match self.reminders_sort {
            ReminderSort::Recency => entries.sort_by_key(|r| std::cmp::Reverse(reminder_recency_key(r))),
            ReminderSort::Project => entries.sort_by_key(|r| project_name(&r.cwd)),
            ReminderSort::Status => entries.sort_by_key(|r| reminder_status_word(r.status).to_string()),
        }

        entries
    }

    /// A project's reminders, most recently updated first - used by the
    /// details modal's Reminders pane. Unlike `visible_reminders`, this
    /// ignores the Reminders tab's own sort/filter state.
    pub fn project_reminders(&self, cwd: &Path) -> Vec<&ReminderInfo> {
        let mut entries: Vec<&ReminderInfo> = self.reminders.iter().filter(|r| r.cwd == cwd).collect();
        entries.sort_by_key(|r| std::cmp::Reverse(r.last_updated_ms));
        entries
    }

    /// The number of reminders shown in the details modal's Reminders pane
    /// for whichever project it's currently open on - 0 if no details modal
    /// is open.
    fn modal_reminders_len(&self) -> usize {
        match &self.modal {
            Some(Modal::Details(cwd)) => self.project_reminders(cwd).len(),
            _ => 0,
        }
    }

    fn clamp_reminders_selected(&mut self) {
        let len = self.visible_reminders().len();
        self.reminders_pagination.clamp_selected(len);
    }

    /// Moves the Reminders tab's selection by `delta` lines, clamped to the
    /// current visible (filtered) entry count.
    pub fn move_reminders_selection(&mut self, delta: isize, page_size: usize) {
        let len = self.visible_reminders().len();
        self.reminders_pagination.move_by(delta, len, page_size);
    }

    /// Moves the Reminders tab's selection by a full page - see `page_logs`.
    pub fn page_reminders(&mut self, direction: isize, page_size: usize) {
        let len = self.visible_reminders().len();
        self.reminders_pagination.page(direction, len, page_size);
    }

    /// Jumps the Reminders tab's selection to the list's first entry.
    pub fn jump_reminders_to_start(&mut self, page_size: usize) {
        let len = self.visible_reminders().len();
        self.reminders_pagination.jump_to_start(len, page_size);
    }

    /// Jumps the Reminders tab's selection to the list's last entry.
    pub fn jump_reminders_to_end(&mut self, page_size: usize) {
        let len = self.visible_reminders().len();
        self.reminders_pagination.jump_to_end(len, page_size);
    }

    /// Moves the details modal's Reminders pane's own selection by `delta`
    /// lines - see `move_modal_logs_selection`, scoped to the Reminders pane.
    pub fn move_modal_reminders_selection(&mut self, delta: isize, page_size: usize) {
        let len = self.modal_reminders_len();
        self.modal_reminders_pagination.move_by(delta, len, page_size);
    }

    /// Moves the details modal's Reminders pane's own selection by a full
    /// page - see `page_modal_logs`.
    pub fn page_modal_reminders(&mut self, direction: isize, page_size: usize) {
        let len = self.modal_reminders_len();
        self.modal_reminders_pagination.page(direction, len, page_size);
    }

    /// Jumps the details modal's Reminders pane's own selection to its first
    /// entry.
    pub fn jump_modal_reminders_to_start(&mut self, page_size: usize) {
        let len = self.modal_reminders_len();
        self.modal_reminders_pagination.jump_to_start(len, page_size);
    }

    /// Jumps the details modal's Reminders pane's own selection to its last
    /// entry.
    pub fn jump_modal_reminders_to_end(&mut self, page_size: usize) {
        let len = self.modal_reminders_len();
        self.modal_reminders_pagination.jump_to_end(len, page_size);
    }

    pub fn cycle_reminders_sort(&mut self) {
        self.reminders_sort = match self.reminders_sort {
            ReminderSort::Recency => ReminderSort::Project,
            ReminderSort::Project => ReminderSort::Status,
            ReminderSort::Status => ReminderSort::Recency,
        };
        self.reminders_pagination.reset();
    }

    /// Cycles the Project filter through every distinct project present
    /// among tracked reminders, alphabetically, then back to no filter.
    pub fn cycle_reminders_project_filter(&mut self) {
        let mut projects: Vec<String> = self.reminders.iter().map(|r| project_name(&r.cwd)).collect();
        projects.sort();
        projects.dedup();
        self.reminders_filter_project = next_in_cycle(self.reminders_filter_project.as_deref(), &projects);
        self.clamp_reminders_selected();
    }

    /// Cycles the Status filter through every distinct status present among
    /// tracked reminders, then back to no filter. Bound to `f` rather than
    /// `s` on the Reminders tab, since `s` starts/stops the highlighted
    /// reminder there.
    pub fn cycle_reminders_status_filter(&mut self) {
        let mut statuses: Vec<String> = self.reminders.iter().map(|r| reminder_status_word(r.status).to_string()).collect();
        statuses.sort();
        statuses.dedup();
        self.reminders_filter_status = next_in_cycle(self.reminders_filter_status.as_deref(), &statuses);
        self.clamp_reminders_selected();
    }

    pub fn clear_reminders_filters(&mut self) {
        self.reminders_filter_project = None;
        self.reminders_filter_status = None;
        self.clamp_reminders_selected();
    }

    /// The `ReminderId` currently highlighted in the Reminders tab, if any.
    pub fn selected_reminder_in_tab(&self) -> Option<ReminderId> {
        self.visible_reminders()
            .get(self.reminders_pagination.selected)
            .map(|r| r.id.clone())
    }

    /// The `ReminderId` currently highlighted in the details modal's
    /// Reminders pane, if the modal is open and it holds any reminders.
    pub fn selected_reminder_in_modal(&self) -> Option<ReminderId> {
        let Some(Modal::Details(cwd)) = &self.modal else { return None };
        self.project_reminders(cwd)
            .get(self.modal_reminders_pagination.selected)
            .map(|r| r.id.clone())
    }

    /// The `ClientMessage` to send to start `id` if it isn't running, or
    /// stop it if it is - the toggle both the Reminders tab and the modal's
    /// Reminders pane use for `Enter`/`s`. `None` if `id` isn't tracked.
    pub fn toggle_reminder_command(&self, id: &ReminderId) -> Option<ClientMessage> {
        let reminder = self.reminders.iter().find(|r| &r.id == id)?;
        Some(if reminder.status == ReminderStatus::Running {
            ClientMessage::StopReminder { id: id.clone() }
        } else {
            ClientMessage::StartReminder { id: id.clone() }
        })
    }

    /// Opens the reminder form empty, for creating a new reminder scoped to
    /// the details modal's current project. A no-op if the details modal
    /// isn't open.
    pub fn open_reminder_create_form(&mut self) {
        let Some(Modal::Details(cwd)) = &self.modal else { return };
        self.reminder_form = Some(ReminderForm {
            mode: ReminderFormMode::Create,
            cwd: cwd.clone(),
            name: String::new(),
            duration_minutes: String::new(),
            field: ReminderFormField::Name,
        });
    }

    /// Opens the reminder form pre-populated with the details modal's
    /// currently highlighted reminder. A no-op if the modal isn't open or
    /// holds no reminders.
    pub fn open_reminder_edit_form(&mut self) {
        let Some(Modal::Details(cwd)) = &self.modal else { return };
        let cwd = cwd.clone();
        let Some(reminder) = self
            .project_reminders(&cwd)
            .get(self.modal_reminders_pagination.selected)
            .map(|r| (*r).clone())
        else {
            return;
        };
        self.reminder_form = Some(ReminderForm {
            mode: ReminderFormMode::Edit(reminder.id),
            cwd,
            name: reminder.name,
            duration_minutes: reminder.duration_minutes.to_string(),
            field: ReminderFormField::Name,
        });
    }

    pub fn close_reminder_form(&mut self) {
        self.reminder_form = None;
    }

    /// Moves input focus between the form's Name and Duration fields - the
    /// `Tab` key while the form is open.
    pub fn toggle_reminder_form_field(&mut self) {
        if let Some(form) = &mut self.reminder_form {
            form.field = match form.field {
                ReminderFormField::Name => ReminderFormField::Duration,
                ReminderFormField::Duration => ReminderFormField::Name,
            };
        }
    }

    /// Appends a typed character to whichever field currently has focus.
    /// The Duration field only accepts digits.
    pub fn reminder_form_input_char(&mut self, c: char) {
        if let Some(form) = &mut self.reminder_form {
            match form.field {
                ReminderFormField::Name => form.name.push(c),
                ReminderFormField::Duration => {
                    if c.is_ascii_digit() {
                        form.duration_minutes.push(c);
                    }
                }
            }
        }
    }

    /// Removes the last character from whichever field currently has focus.
    pub fn reminder_form_backspace(&mut self) {
        if let Some(form) = &mut self.reminder_form {
            match form.field {
                ReminderFormField::Name => {
                    form.name.pop();
                }
                ReminderFormField::Duration => {
                    form.duration_minutes.pop();
                }
            }
        }
    }

    /// Closes the open reminder form and returns the `ClientMessage` to send
    /// (create or edit) - the `Enter` key while the form is open. `None` if
    /// no form is open. A new reminder is remembered as `pending_new_reminder`
    /// so `apply_reminder_update` can recognize the daemon's confirmation of
    /// it and highlight/auto-start it - see that method's doc comment.
    pub fn submit_reminder_form(&mut self) -> Option<ClientMessage> {
        let form = self.reminder_form.take()?;
        let duration_minutes: u32 = form.duration_minutes.parse().unwrap_or(0);
        Some(match form.mode {
            ReminderFormMode::Create => {
                self.pending_new_reminder = Some((form.cwd.clone(), form.name.clone()));
                ClientMessage::CreateReminder {
                    cwd: form.cwd,
                    name: form.name,
                    duration_minutes,
                }
            }
            ReminderFormMode::Edit(id) => ClientMessage::UpdateReminder {
                id,
                name: form.name,
                duration_minutes,
            },
        })
    }

    /// Opens the delete-confirmation dialog for the details modal's
    /// currently highlighted reminder. A no-op if the modal isn't open or
    /// holds no reminders.
    pub fn open_delete_reminder_confirm(&mut self) {
        let Some(Modal::Details(cwd)) = &self.modal else { return };
        let cwd = cwd.clone();
        if let Some(reminder) = self.project_reminders(&cwd).get(self.modal_reminders_pagination.selected) {
            self.confirm_delete_reminder = Some(reminder.id.clone());
        }
    }

    pub fn close_delete_reminder_confirm(&mut self) {
        self.confirm_delete_reminder = None;
    }

    /// Closes the delete-confirmation dialog and returns the `ClientMessage`
    /// to send - the `Enter` key while it is open. `None` if no dialog is
    /// open.
    pub fn confirm_delete_reminder_command(&mut self) -> Option<ClientMessage> {
        let id = self.confirm_delete_reminder.take()?;
        Some(ClientMessage::DeleteReminder { id })
    }
}

/// The status word a reminder's status maps to for filtering/sorting -
/// matches the wire's snake_case spelling (see `ReminderStatus`).
fn reminder_status_word(status: ReminderStatus) -> &'static str {
    match status {
        ReminderStatus::NotYetStarted => "not_yet_started",
        ReminderStatus::Running => "running",
        ReminderStatus::Done => "done",
    }
}

/// The Reminders tab's default sort key: a reminder's last *completed* run
/// (stopped or finished, i.e. `Done`), falling back to its creation time when
/// it has none yet - distinct from the Updated column, which also counts a
/// bare start. See design.md's "Recency sort fallback" decision.
fn reminder_recency_key(reminder: &ReminderInfo) -> u64 {
    if reminder.status == ReminderStatus::Done {
        reminder.last_updated_ms
    } else {
        reminder.created_at_ms
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

        app.apply_test_run_update(test_run_in("/tmp/project", 1, agentmon_proto::TestRunStatus::Running, 0));

        assert_eq!(app.test_runs.len(), 1);
    }

    #[test]
    fn apply_test_run_update_updates_the_same_cwd_and_pid_in_place() {
        let mut app = App::new();
        app.apply_test_run_update(test_run_in("/tmp/project", 1, agentmon_proto::TestRunStatus::Running, 0));

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
        app.apply_test_run_update(test_run_in("/tmp/project", 1, agentmon_proto::TestRunStatus::Running, 0));

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
            vec![test_run_in("/tmp/newer", 1, agentmon_proto::TestRunStatus::Running, 2_000)],
        );

        let groups = app.directory_groups();

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].cwd, PathBuf::from("/tmp/newer"));
        assert_eq!(groups[1].cwd, PathBuf::from("/tmp/older"));
    }

    #[test]
    fn a_projects_reminder_activity_counts_toward_its_recency_ordering() {
        let mut app = App::new();
        app.apply_snapshot(
            vec![
                AgentInfo {
                    last_updated_ms: 1_000,
                    ..agent_in("/tmp/recent-agent", "a", AgentStatus::Running)
                },
                AgentInfo {
                    last_updated_ms: 1_000,
                    ..agent_in("/tmp/recent-reminder", "b", AgentStatus::Running)
                },
            ],
            Vec::new(),
        );
        // "/tmp/recent-reminder" has an older agent update (1_000) but a
        // much more recent reminder event (5_000) - that reminder activity
        // must be enough to sort it ahead of "/tmp/recent-agent".
        app.apply_reminder_update(reminder_in("/tmp/recent-reminder", "r1", "Check the build", ReminderStatus::Done, 0, Some(0), 5_000));

        let groups = app.directory_groups();

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].cwd, PathBuf::from("/tmp/recent-reminder"), "the project with the more recent reminder activity should sort first");
        assert_eq!(groups[1].cwd, PathBuf::from("/tmp/recent-agent"));
    }

    #[test]
    fn a_projects_updated_column_reflects_its_reminder_activity() {
        let mut app = App::new();
        app.apply_snapshot(vec![agent_in("/tmp/project", "a", AgentStatus::Running)], Vec::new());
        app.apply_reminder_update(reminder_in("/tmp/project", "r1", "Check the build", ReminderStatus::Done, 0, Some(0), 9_999));

        let groups = app.directory_groups();

        assert_eq!(groups[0].most_recent_update_ms(), 9_999);
    }

    #[test]
    fn directory_groups_orders_agents_before_test_runs_within_a_group() {
        let mut app = App::new();
        app.apply_snapshot(
            vec![AgentInfo {
                last_updated_ms: 1_000,
                ..agent_in("/tmp/project", "a", AgentStatus::Running)
            }],
            vec![test_run_in("/tmp/project", 1, agentmon_proto::TestRunStatus::Running, 2_000)],
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
            reminder_name: None,
            reminder_due_at_ms: None,
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
        assert_eq!(app.active_tab, Tab::Reminders);
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
    fn visible_agent_groups_filters_by_a_case_insensitive_substring_of_project_name() {
        let mut app = App::new();
        app.apply_snapshot(
            vec![
                agent_in("/tmp/LAB-1234", "a", AgentStatus::Running),
                agent_in("/tmp/other-project", "b", AgentStatus::Running),
            ],
            Vec::new(),
        );

        app.agents_search_applied = Some("1234".to_string());
        let groups = app.visible_agent_groups();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].cwd, PathBuf::from("/tmp/LAB-1234"));

        app.agents_search_applied = Some("lab".to_string());
        assert_eq!(app.visible_agent_groups().len(), 1, "match must be case-insensitive");

        app.agents_search_applied = Some("no-such-project".to_string());
        assert!(app.visible_agent_groups().is_empty(), "a non-matching filter shows no rows");

        app.agents_search_applied = None;
        assert_eq!(app.visible_agent_groups().len(), 2, "no filter shows every row");
    }

    #[test]
    fn visible_agent_groups_preserves_the_unfiltered_relative_order() {
        let mut app = App::new();
        app.apply_snapshot(
            vec![
                AgentInfo {
                    last_updated_ms: 1_000,
                    ..agent_in("/tmp/project-a", "a", AgentStatus::Running)
                },
                AgentInfo {
                    last_updated_ms: 2_000,
                    ..agent_in("/tmp/project-b", "b", AgentStatus::Running)
                },
            ],
            Vec::new(),
        );

        app.agents_search_applied = Some("project".to_string());
        let groups = app.visible_agent_groups();

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].cwd, PathBuf::from("/tmp/project-b"), "most recently updated stays first");
        assert_eq!(groups[1].cwd, PathBuf::from("/tmp/project-a"));
    }

    #[test]
    fn visible_agent_groups_filters_live_against_the_in_progress_buffer_while_editing() {
        let mut app = App::new();
        app.apply_snapshot(
            vec![
                agent_in("/tmp/LAB-1234", "a", AgentStatus::Running),
                agent_in("/tmp/other-project", "b", AgentStatus::Running),
            ],
            Vec::new(),
        );

        app.open_agents_search();
        app.push_agents_search_char('1');
        app.push_agents_search_char('2');
        app.push_agents_search_char('3');
        app.push_agents_search_char('4');

        let groups = app.visible_agent_groups();
        assert_eq!(
            groups.len(),
            1,
            "typing must filter the list immediately, before Enter commits it"
        );
        assert_eq!(groups[0].cwd, PathBuf::from("/tmp/LAB-1234"));
    }

    #[test]
    fn agents_selection_is_clamped_when_a_search_filter_shrinks_the_visible_rows() {
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

        app.agents_search_buffer = "a".to_string();
        app.commit_agents_search();

        assert_eq!(app.agents_selected, 0, "selection must clamp once the filter drops the selected row");
    }

    #[test]
    fn open_agents_search_seeds_the_buffer_from_the_applied_filter() {
        let mut app = App::new();
        app.agents_search_applied = Some("lab".to_string());

        app.open_agents_search();

        assert!(app.agents_search_editing);
        assert_eq!(app.agents_search_buffer, "lab");
    }

    #[test]
    fn open_agents_search_seeds_an_empty_buffer_when_no_filter_is_applied() {
        let mut app = App::new();

        app.open_agents_search();

        assert!(app.agents_search_editing);
        assert_eq!(app.agents_search_buffer, "");
    }

    #[test]
    fn push_and_pop_agents_search_char_edit_the_buffer() {
        let mut app = App::new();
        app.open_agents_search();

        app.push_agents_search_char('a');
        app.push_agents_search_char('b');
        assert_eq!(app.agents_search_buffer, "ab");

        app.pop_agents_search_char();
        assert_eq!(app.agents_search_buffer, "a");
    }

    #[test]
    fn commit_agents_search_applies_a_non_empty_buffer_and_exits_editing() {
        let mut app = App::new();
        app.open_agents_search();
        app.push_agents_search_char('x');

        app.commit_agents_search();

        assert_eq!(app.agents_search_applied.as_deref(), Some("x"));
        assert!(!app.agents_search_editing);
    }

    #[test]
    fn commit_agents_search_with_an_empty_buffer_clears_the_applied_filter() {
        let mut app = App::new();
        app.agents_search_applied = Some("x".to_string());
        app.open_agents_search();
        app.pop_agents_search_char();

        app.commit_agents_search();

        assert_eq!(app.agents_search_applied, None);
        assert!(!app.agents_search_editing);
    }

    #[test]
    fn cancel_agents_search_restores_the_prior_applied_filter() {
        let mut app = App::new();
        app.agents_search_applied = Some("x".to_string());
        app.open_agents_search();
        app.push_agents_search_char('y');

        app.cancel_agents_search();

        assert_eq!(
            app.agents_search_applied.as_deref(),
            Some("x"),
            "cancelling must restore the filter that was applied before editing began"
        );
        assert!(!app.agents_search_editing);
    }

    #[test]
    fn cancel_agents_search_with_no_prior_filter_leaves_none_applied() {
        let mut app = App::new();
        app.open_agents_search();
        app.push_agents_search_char('y');

        app.cancel_agents_search();

        assert_eq!(app.agents_search_applied, None);
        assert!(!app.agents_search_editing);
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
        app.move_logs_selection(1, 10); // select "/tmp/a", the older (second) entry once sorted by recency

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
    fn paginator_move_by_moves_one_line_at_a_time_clamped() {
        let mut p = Paginator::default();

        p.move_by(1, 2, 10);
        assert_eq!(p.selected, 1);
        p.move_by(1, 2, 10);
        assert_eq!(p.selected, 1, "must clamp at the last entry");
        p.move_by(-1, 2, 10);
        assert_eq!(p.selected, 0);
        p.move_by(-1, 2, 10);
        assert_eq!(p.selected, 0, "must clamp at the first entry");
    }

    #[test]
    fn paginator_page_moves_by_a_full_page_with_a_1_row_overlap() {
        let mut p = Paginator::default();
        let (len, page_size) = (30, 10);

        p.page(1, len, page_size);
        assert_eq!(p.top, 9, "advances by page_size - 1, overlapping the previous page by 1 row");
        assert_eq!(p.selected, 9, "selection snaps to the new page's top row");

        p.page(1, len, page_size);
        assert_eq!(p.top, 18);
        assert_eq!(p.selected, 18);

        p.page(-1, len, page_size);
        assert_eq!(p.top, 9);
        assert_eq!(p.selected, 9);
    }

    #[test]
    fn paginator_page_clamps_at_the_first_and_last_page_instead_of_overshooting() {
        let mut p = Paginator::default();
        let (len, page_size) = (15, 10);

        p.page(-1, len, page_size);
        assert_eq!((p.top, p.selected), (0, 0), "already on the first page");

        p.page(1, len, page_size);
        assert_eq!((p.top, p.selected), (5, 5), "last page still shows a full page's worth of rows");
        p.page(1, len, page_size);
        assert_eq!((p.top, p.selected), (5, 5), "must not overshoot past the last page");
    }

    #[test]
    fn paginator_jump_to_start_and_end_are_no_ops_on_an_empty_list() {
        let mut p = Paginator::default();

        p.jump_to_end(0, 10);
        assert_eq!((p.top, p.selected), (0, 0));
        p.jump_to_start(0, 10);
        assert_eq!((p.top, p.selected), (0, 0));
    }

    #[test]
    fn paginator_jump_to_start_and_end_on_a_single_page_list() {
        let mut p = Paginator::default();
        let (len, page_size) = (5, 10);

        p.jump_to_end(len, page_size);
        assert_eq!((p.top, p.selected), (0, 4), "selects the last row without scrolling past a single page");

        p.jump_to_start(len, page_size);
        assert_eq!((p.top, p.selected), (0, 0));
    }

    #[test]
    fn paginator_jump_to_start_and_end_on_a_multi_page_list() {
        let mut p = Paginator::default();
        let (len, page_size) = (30, 10);

        p.jump_to_end(len, page_size);
        assert_eq!((p.top, p.selected), (20, 29), "scrolls so the last page is showing, last entry selected");

        p.jump_to_start(len, page_size);
        assert_eq!((p.top, p.selected), (0, 0), "scrolls back to the first page, first entry selected");
    }

    #[test]
    fn paginator_jump_to_end_is_idempotent() {
        let mut p = Paginator::default();
        let (len, page_size) = (30, 10);

        p.jump_to_end(len, page_size);
        let after_first = (p.top, p.selected);
        p.jump_to_end(len, page_size);
        assert_eq!((p.top, p.selected), after_first, "repeated jumps to the end must not change state further");
    }

    #[test]
    fn paginator_jump_to_start_is_idempotent() {
        let mut p = Paginator::default();
        let (len, page_size) = (30, 10);
        p.jump_to_end(len, page_size);

        p.jump_to_start(len, page_size);
        let after_first = (p.top, p.selected);
        p.jump_to_start(len, page_size);
        assert_eq!((p.top, p.selected), after_first, "repeated jumps to the start must not change state further");
    }

    #[test]
    fn jump_logs_to_start_and_end_move_the_logs_tabs_selection() {
        let mut app = App::new();
        let page_size = 10;
        let entries: Vec<LogEntry> = (0..(page_size * 3) as u64).map(|i| log_in("/tmp/a", "done", i)).collect();
        app.apply_log_snapshot(entries);

        app.jump_logs_to_end(page_size);
        assert_eq!(app.logs_pagination.selected, page_size * 3 - 1);

        app.jump_logs_to_start(page_size);
        assert_eq!(app.logs_pagination.selected, 0);
    }

    #[test]
    fn jump_modal_logs_to_start_and_end_move_the_modals_own_selection() {
        let mut app = App::new();
        app.apply_snapshot(vec![agent("a", AgentStatus::Running)], Vec::new());
        let page_size = 10;
        let entries: Vec<LogEntry> = (0..(page_size * 3) as u64).map(|i| log_in("/tmp/project", "done", i)).collect();
        app.apply_log_snapshot(entries);
        app.open_details_modal();

        app.jump_modal_logs_to_end(page_size);
        assert_eq!(app.modal_logs_pagination.selected, page_size * 3 - 1);
        assert_eq!(app.logs_pagination.selected, 0, "the Logs tab underneath must not react");

        app.jump_modal_logs_to_start(page_size);
        assert_eq!(app.modal_logs_pagination.selected, 0);
    }

    #[test]
    fn move_logs_selection_moves_one_line_at_a_time_clamped() {
        let mut app = App::new();
        app.apply_log_snapshot(vec![log_in("/tmp/a", "done", 1), log_in("/tmp/b", "done", 2)]);

        app.move_logs_selection(1, 10);
        assert_eq!(app.logs_pagination.selected, 1);
        app.move_logs_selection(1, 10);
        assert_eq!(app.logs_pagination.selected, 1, "must clamp at the last entry");
        app.move_logs_selection(-1, 10);
        assert_eq!(app.logs_pagination.selected, 0);
        app.move_logs_selection(-1, 10);
        assert_eq!(app.logs_pagination.selected, 0, "must clamp at the first entry");
    }

    #[test]
    fn page_logs_moves_by_a_full_page_with_a_1_row_overlap_clamped() {
        let mut app = App::new();
        let page_size = 10;
        let entries: Vec<LogEntry> = (0..(page_size * 3) as u64)
            .map(|i| log_in("/tmp/a", "done", i))
            .collect();
        app.apply_log_snapshot(entries);

        app.page_logs(1, page_size);
        assert_eq!(app.logs_pagination.selected, page_size - 1);
        app.page_logs(1, page_size);
        assert_eq!(app.logs_pagination.selected, (page_size - 1) * 2);
        app.page_logs(1, page_size);
        assert_eq!(
            app.logs_pagination.selected,
            page_size * 2, // max_top = len - page_size = 30 - 10 = 20
            "must clamp at the last page rather than overshoot"
        );
        app.page_logs(-1, page_size);
        assert_eq!(app.logs_pagination.selected, page_size * 2 - (page_size - 1));
    }

    #[test]
    fn logs_selection_is_clamped_when_a_filter_shrinks_the_visible_list() {
        let mut app = App::new();
        app.apply_log_snapshot(vec![log_in("/tmp/a", "done", 1), log_in("/tmp/b", "done", 2)]);
        app.move_logs_selection(1, 10);
        assert_eq!(app.logs_pagination.selected, 1);

        app.cycle_logs_project_filter();

        assert_eq!(app.logs_pagination.selected, 0, "selection must clamp once the filtered list shrinks");
    }

    #[test]
    fn opening_the_details_modal_resets_its_logs_panes_pagination() {
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
        app.apply_log_snapshot(vec![log_in("/tmp/a", "done", 1), log_in("/tmp/a", "started", 2)]);
        app.open_details_modal(); // opens on "/tmp/a", the more recently updated project
        app.move_modal_logs_selection(1, 10);
        assert_eq!(app.modal_logs_pagination.selected, 1);
        app.close_modal();

        app.move_agents_selection(1); // select "/tmp/b"
        app.open_details_modal();

        assert_eq!(
            app.modal_logs_pagination.selected, 0,
            "reopening the modal must reset its Logs pane's pagination"
        );
        assert_eq!(app.modal_logs_pagination.top, 0);
    }

    fn reminder_in(
        cwd: &str,
        id: &str,
        name: &str,
        status: ReminderStatus,
        created_at_ms: u64,
        run_started_ms: Option<u64>,
        last_updated_ms: u64,
    ) -> ReminderInfo {
        ReminderInfo {
            id: ReminderId(id.to_string()),
            cwd: PathBuf::from(cwd),
            name: name.to_string(),
            duration_minutes: 10,
            status,
            created_at_ms,
            run_started_ms,
            last_updated_ms,
        }
    }

    #[test]
    fn apply_reminder_snapshot_replaces_the_reminder_list() {
        let mut app = App::new();

        app.apply_reminder_snapshot(vec![reminder_in(
            "/tmp/a",
            "r1",
            "Check the build",
            ReminderStatus::NotYetStarted,
            0,
            None,
            0,
        )]);

        assert_eq!(app.reminders.len(), 1);
    }

    #[test]
    fn apply_reminder_update_adds_then_updates_in_place() {
        let mut app = App::new();
        let reminder = reminder_in("/tmp/a", "r1", "Check the build", ReminderStatus::NotYetStarted, 0, None, 0);
        app.apply_reminder_update(reminder.clone());
        assert_eq!(app.reminders.len(), 1);

        let updated = ReminderInfo { status: ReminderStatus::Running, ..reminder };
        app.apply_reminder_update(updated);

        assert_eq!(app.reminders.len(), 1, "must update in place, not duplicate");
        assert_eq!(app.reminders[0].status, ReminderStatus::Running);
    }

    #[test]
    fn remove_reminder_drops_it() {
        let mut app = App::new();
        app.apply_reminder_update(reminder_in("/tmp/a", "r1", "Check the build", ReminderStatus::NotYetStarted, 0, None, 0));

        app.remove_reminder(&ReminderId("r1".to_string()));

        assert!(app.reminders.is_empty());
    }

    #[test]
    fn remove_reminder_is_a_no_op_for_an_unknown_id() {
        let mut app = App::new();
        app.apply_reminder_update(reminder_in("/tmp/a", "r1", "Check the build", ReminderStatus::NotYetStarted, 0, None, 0));

        app.remove_reminder(&ReminderId("unknown".to_string()));

        assert_eq!(app.reminders.len(), 1);
    }

    #[test]
    fn visible_reminders_default_sorted_by_last_completed_run() {
        let mut app = App::new();
        app.apply_reminder_snapshot(vec![
            reminder_in("/tmp/a", "older", "Older", ReminderStatus::Done, 0, Some(0), 1_000),
            reminder_in("/tmp/b", "newer", "Newer", ReminderStatus::Done, 0, Some(0), 2_000),
        ]);

        let visible = app.visible_reminders();

        assert_eq!(visible[0].id, ReminderId("newer".to_string()));
        assert_eq!(visible[1].id, ReminderId("older".to_string()));
    }

    #[test]
    fn visible_reminders_recency_falls_back_to_creation_time_when_never_completed() {
        let mut app = App::new();
        app.apply_reminder_snapshot(vec![
            // Running, not yet completed - sorts by creation time, not last_updated_ms.
            reminder_in("/tmp/a", "running", "Running", ReminderStatus::Running, 500, Some(500), 999_999),
            reminder_in("/tmp/b", "done", "Done", ReminderStatus::Done, 0, Some(0), 1_000),
        ]);

        let visible = app.visible_reminders();

        assert_eq!(
            visible[0].id,
            ReminderId("done".to_string()),
            "a done reminder's completed-run time should outrank a running reminder's creation time here"
        );
    }

    #[test]
    fn cycle_reminders_sort_moves_through_recency_project_status() {
        let mut app = App::new();
        assert_eq!(app.reminders_sort, ReminderSort::Recency);
        app.cycle_reminders_sort();
        assert_eq!(app.reminders_sort, ReminderSort::Project);
        app.cycle_reminders_sort();
        assert_eq!(app.reminders_sort, ReminderSort::Status);
        app.cycle_reminders_sort();
        assert_eq!(app.reminders_sort, ReminderSort::Recency);
    }

    #[test]
    fn reminders_sorting_by_project_orders_alphabetically() {
        let mut app = App::new();
        app.apply_reminder_snapshot(vec![
            reminder_in("/tmp/zeta", "z", "Z", ReminderStatus::NotYetStarted, 0, None, 0),
            reminder_in("/tmp/alpha", "a", "A", ReminderStatus::NotYetStarted, 0, None, 0),
        ]);
        app.cycle_reminders_sort();

        let visible = app.visible_reminders();

        assert_eq!(visible[0].cwd, PathBuf::from("/tmp/alpha"));
        assert_eq!(visible[1].cwd, PathBuf::from("/tmp/zeta"));
    }

    #[test]
    fn reminders_sorting_by_status_orders_alphabetically() {
        let mut app = App::new();
        app.apply_reminder_snapshot(vec![
            reminder_in("/tmp/a", "a", "A", ReminderStatus::Running, 0, Some(0), 0),
            reminder_in("/tmp/b", "b", "B", ReminderStatus::Done, 0, Some(0), 0),
        ]);
        app.cycle_reminders_sort();
        app.cycle_reminders_sort();

        let visible = app.visible_reminders();

        assert_eq!(visible[0].status, ReminderStatus::Done);
        assert_eq!(visible[1].status, ReminderStatus::Running);
    }

    #[test]
    fn cycle_reminders_project_filter_cycles_through_projects_then_clears() {
        let mut app = App::new();
        app.apply_reminder_snapshot(vec![
            reminder_in("/tmp/a", "a", "A", ReminderStatus::NotYetStarted, 0, None, 0),
            reminder_in("/tmp/b", "b", "B", ReminderStatus::NotYetStarted, 0, None, 0),
        ]);

        app.cycle_reminders_project_filter();
        assert_eq!(app.reminders_filter_project.as_deref(), Some("a"));
        assert_eq!(app.visible_reminders().len(), 1);

        app.cycle_reminders_project_filter();
        assert_eq!(app.reminders_filter_project.as_deref(), Some("b"));

        app.cycle_reminders_project_filter();
        assert_eq!(app.reminders_filter_project, None, "must cycle back to no filter");
    }

    #[test]
    fn cycle_reminders_status_filter_cycles_through_statuses_then_clears() {
        let mut app = App::new();
        app.apply_reminder_snapshot(vec![
            reminder_in("/tmp/a", "a", "A", ReminderStatus::Running, 0, Some(0), 0),
            reminder_in("/tmp/a", "b", "B", ReminderStatus::Done, 0, Some(0), 0),
        ]);

        app.cycle_reminders_status_filter();
        assert_eq!(app.reminders_filter_status.as_deref(), Some("done"));
        assert_eq!(app.visible_reminders().len(), 1);

        app.cycle_reminders_status_filter();
        assert_eq!(app.reminders_filter_status.as_deref(), Some("running"));

        app.cycle_reminders_status_filter();
        assert_eq!(app.reminders_filter_status, None);
    }

    #[test]
    fn clear_reminders_filters_resets_both() {
        let mut app = App::new();
        app.apply_reminder_snapshot(vec![reminder_in(
            "/tmp/a",
            "a",
            "A",
            ReminderStatus::NotYetStarted,
            0,
            None,
            0,
        )]);
        app.cycle_reminders_project_filter();
        app.cycle_reminders_status_filter();

        app.clear_reminders_filters();

        assert_eq!(app.reminders_filter_project, None);
        assert_eq!(app.reminders_filter_status, None);
    }

    #[test]
    fn directory_groups_attaches_reminders_only_to_an_existing_group() {
        let mut app = App::new();
        app.apply_snapshot(vec![agent_in("/tmp/project", "a", AgentStatus::Running)], Vec::new());
        app.apply_reminder_snapshot(vec![
            reminder_in("/tmp/project", "r1", "In project", ReminderStatus::NotYetStarted, 0, None, 0),
            reminder_in("/tmp/reminder-only", "r2", "No agent here", ReminderStatus::NotYetStarted, 0, None, 0),
        ]);

        let groups = app.directory_groups();

        assert_eq!(groups.len(), 1, "a reminder-only project must not get its own Agents tab row");
        assert_eq!(groups[0].reminders.len(), 1);
        assert_eq!(groups[0].reminders[0].id, ReminderId("r1".to_string()));
    }

    #[test]
    fn directory_groups_orders_a_projects_reminders_most_recently_updated_first() {
        let mut app = App::new();
        app.apply_snapshot(vec![agent_in("/tmp/project", "a", AgentStatus::Running)], Vec::new());
        app.apply_reminder_snapshot(vec![
            reminder_in("/tmp/project", "older", "Older", ReminderStatus::Done, 0, Some(0), 1_000),
            reminder_in("/tmp/project", "newer", "Newer", ReminderStatus::Done, 0, Some(0), 2_000),
        ]);

        let groups = app.directory_groups();

        assert_eq!(groups[0].reminders[0].id, ReminderId("newer".to_string()));
        assert_eq!(groups[0].reminders[1].id, ReminderId("older".to_string()));
    }

    #[test]
    fn opening_details_modal_from_agents_tab_defaults_focus_to_logs() {
        let mut app = App::new();
        app.apply_snapshot(vec![agent_in("/tmp/a", "a", AgentStatus::Running)], Vec::new());

        app.open_details_modal();

        assert_eq!(app.modal_focus, ModalFocus::Logs);
    }

    #[test]
    fn opening_details_modal_from_reminders_tab_focuses_reminders_and_highlights_it() {
        let mut app = App::new();
        app.apply_reminder_snapshot(vec![
            reminder_in("/tmp/a", "r1", "First", ReminderStatus::NotYetStarted, 0, None, 0),
            reminder_in("/tmp/a", "r2", "Second", ReminderStatus::NotYetStarted, 1_000, None, 1_000),
        ]);
        app.set_tab(Tab::Reminders);
        app.move_reminders_selection(1, 10); // select the second (older, since default sort falls back to creation time)

        app.open_details_modal();

        assert_eq!(app.modal, Some(Modal::Details(PathBuf::from("/tmp/a"))));
        assert_eq!(app.modal_focus, ModalFocus::Reminders);
        let selected_id = app.selected_reminder_in_modal();
        assert!(selected_id.is_some(), "the invoking reminder should be highlighted in the modal's pane");
    }

    #[test]
    fn toggle_modal_focus_switches_between_logs_and_reminders() {
        let mut app = App::new();
        app.apply_snapshot(vec![agent_in("/tmp/a", "a", AgentStatus::Running)], Vec::new());
        app.open_details_modal();
        assert_eq!(app.modal_focus, ModalFocus::Logs);

        app.toggle_modal_focus();
        assert_eq!(app.modal_focus, ModalFocus::Reminders);

        app.toggle_modal_focus();
        assert_eq!(app.modal_focus, ModalFocus::Logs);
    }

    #[test]
    fn close_modal_also_closes_the_reminder_form_and_delete_confirm() {
        let mut app = App::new();
        app.apply_snapshot(vec![agent_in("/tmp/a", "a", AgentStatus::Running)], Vec::new());
        app.open_details_modal();
        app.open_reminder_create_form();
        assert!(app.reminder_form.is_some());

        app.close_modal();

        assert!(app.reminder_form.is_none());
        assert!(app.confirm_delete_reminder.is_none());
    }

    #[test]
    fn open_reminder_create_form_opens_empty_scoped_to_the_modals_project() {
        let mut app = App::new();
        app.apply_snapshot(vec![agent_in("/tmp/a", "a", AgentStatus::Running)], Vec::new());
        app.open_details_modal();

        app.open_reminder_create_form();

        let form = app.reminder_form.as_ref().unwrap();
        assert_eq!(form.mode, ReminderFormMode::Create);
        assert_eq!(form.cwd, PathBuf::from("/tmp/a"));
        assert_eq!(form.name, "");
        assert_eq!(form.duration_minutes, "");
    }

    #[test]
    fn open_reminder_edit_form_pre_populates_the_highlighted_reminder() {
        let mut app = App::new();
        app.apply_snapshot(vec![agent_in("/tmp/a", "a", AgentStatus::Running)], Vec::new());
        app.apply_reminder_update(ReminderInfo {
            duration_minutes: 25,
            ..reminder_in("/tmp/a", "r1", "Check the build", ReminderStatus::NotYetStarted, 0, None, 0)
        });
        app.open_details_modal();

        app.open_reminder_edit_form();

        let form = app.reminder_form.as_ref().unwrap();
        assert_eq!(form.mode, ReminderFormMode::Edit(ReminderId("r1".to_string())));
        assert_eq!(form.name, "Check the build");
        assert_eq!(form.duration_minutes, "25");
    }

    #[test]
    fn reminder_form_field_input_and_backspace_edit_the_active_field() {
        let mut app = App::new();
        app.apply_snapshot(vec![agent_in("/tmp/a", "a", AgentStatus::Running)], Vec::new());
        app.open_details_modal();
        app.open_reminder_create_form();

        app.reminder_form_input_char('H');
        app.reminder_form_input_char('i');
        assert_eq!(app.reminder_form.as_ref().unwrap().name, "Hi");

        app.toggle_reminder_form_field();
        app.reminder_form_input_char('5');
        app.reminder_form_input_char('x'); // non-digit, ignored on the Duration field
        assert_eq!(app.reminder_form.as_ref().unwrap().duration_minutes, "5");

        app.reminder_form_backspace();
        assert_eq!(app.reminder_form.as_ref().unwrap().duration_minutes, "");
    }

    #[test]
    fn submit_reminder_form_for_create_builds_a_create_reminder_message() {
        let mut app = App::new();
        app.apply_snapshot(vec![agent_in("/tmp/a", "a", AgentStatus::Running)], Vec::new());
        app.open_details_modal();
        app.open_reminder_create_form();
        app.reminder_form_input_char('X');
        app.toggle_reminder_form_field();
        app.reminder_form_input_char('1');
        app.reminder_form_input_char('0');

        let message = app.submit_reminder_form();

        assert_eq!(
            message,
            Some(ClientMessage::CreateReminder {
                cwd: PathBuf::from("/tmp/a"),
                name: "X".to_string(),
                duration_minutes: 10,
            })
        );
        assert!(app.reminder_form.is_none(), "submitting must close the form");
    }

    #[test]
    fn confirming_a_just_created_reminder_highlights_and_returns_its_id_to_start() {
        let mut app = App::new();
        app.apply_snapshot(vec![agent_in("/tmp/a", "a", AgentStatus::Running)], Vec::new());
        app.open_details_modal();
        app.open_reminder_create_form();
        app.reminder_form_input_char('X');
        app.submit_reminder_form();
        // A pre-existing, unrelated reminder in the same project shouldn't
        // interfere with selecting the new one below.
        app.apply_reminder_update(reminder_in("/tmp/a", "older", "Older", ReminderStatus::NotYetStarted, 0, None, 0));

        // The daemon's confirmation, naming the id it assigned.
        let confirmed = reminder_in("/tmp/a", "new-id", "X", ReminderStatus::NotYetStarted, 0, None, 0);
        let started_id = app.apply_reminder_update(confirmed);

        assert_eq!(started_id, Some(ReminderId("new-id".to_string())), "should signal the new reminder needs starting");
        assert_eq!(app.modal_focus, ModalFocus::Reminders, "should focus the Reminders pane to show it");
        let selected_id = app.selected_reminder_in_modal();
        assert_eq!(selected_id, Some(ReminderId("new-id".to_string())), "the new reminder should be highlighted");
    }

    #[test]
    fn a_reminder_update_unrelated_to_any_pending_create_does_not_autostart() {
        let mut app = App::new();
        app.apply_snapshot(vec![agent_in("/tmp/a", "a", AgentStatus::Running)], Vec::new());
        app.open_details_modal();

        // No form was ever opened/submitted - this is just an ordinary
        // incoming update (e.g. another client started a reminder).
        let id = app.apply_reminder_update(reminder_in("/tmp/a", "r1", "Some reminder", ReminderStatus::Running, 0, Some(0), 0));

        assert_eq!(id, None);
    }

    #[test]
    fn editing_a_reminder_never_triggers_autostart() {
        let mut app = App::new();
        app.apply_snapshot(vec![agent_in("/tmp/a", "a", AgentStatus::Running)], Vec::new());
        app.apply_reminder_update(reminder_in("/tmp/a", "r1", "A", ReminderStatus::NotYetStarted, 0, None, 0));
        app.open_details_modal();
        app.open_reminder_edit_form();
        app.reminder_form_input_char('X'); // "AX"
        app.submit_reminder_form();

        let id = app.apply_reminder_update(reminder_in("/tmp/a", "r1", "AX", ReminderStatus::NotYetStarted, 0, None, 0));

        assert_eq!(id, None, "editing an existing reminder must never be mistaken for a new one to auto-start");
    }

    #[test]
    fn a_second_create_before_the_first_confirms_replaces_the_pending_marker() {
        let mut app = App::new();
        app.apply_snapshot(vec![agent_in("/tmp/a", "a", AgentStatus::Running)], Vec::new());
        app.open_details_modal();
        app.open_reminder_create_form();
        app.reminder_form_input_char('A');
        app.submit_reminder_form();
        app.open_reminder_create_form();
        app.reminder_form_input_char('B');
        app.submit_reminder_form();

        // The (late) confirmation for "A" arrives after "B" was already
        // submitted - it must not be mistaken for the pending create.
        let id = app.apply_reminder_update(reminder_in("/tmp/a", "a-id", "A", ReminderStatus::NotYetStarted, 0, None, 0));
        assert_eq!(id, None, "a stale confirmation for the superseded create must not auto-start");

        let id = app.apply_reminder_update(reminder_in("/tmp/a", "b-id", "B", ReminderStatus::NotYetStarted, 0, None, 0));
        assert_eq!(id, Some(ReminderId("b-id".to_string())), "the most recent create should still auto-start");
    }

    #[test]
    fn close_reminder_form_discards_without_submitting() {
        let mut app = App::new();
        app.apply_snapshot(vec![agent_in("/tmp/a", "a", AgentStatus::Running)], Vec::new());
        app.open_details_modal();
        app.open_reminder_create_form();
        app.reminder_form_input_char('X');

        app.close_reminder_form();

        assert!(app.reminder_form.is_none());
    }

    #[test]
    fn toggle_reminder_command_starts_a_not_running_reminder_and_stops_a_running_one() {
        let mut app = App::new();
        app.apply_reminder_update(reminder_in("/tmp/a", "r1", "A", ReminderStatus::NotYetStarted, 0, None, 0));

        assert_eq!(
            app.toggle_reminder_command(&ReminderId("r1".to_string())),
            Some(ClientMessage::StartReminder { id: ReminderId("r1".to_string()) })
        );

        app.apply_reminder_update(reminder_in("/tmp/a", "r1", "A", ReminderStatus::Running, 0, Some(0), 0));

        assert_eq!(
            app.toggle_reminder_command(&ReminderId("r1".to_string())),
            Some(ClientMessage::StopReminder { id: ReminderId("r1".to_string()) })
        );
    }

    #[test]
    fn open_delete_reminder_confirm_and_confirm_build_a_delete_message() {
        let mut app = App::new();
        app.apply_snapshot(vec![agent_in("/tmp/a", "a", AgentStatus::Running)], Vec::new());
        app.apply_reminder_update(reminder_in("/tmp/a", "r1", "A", ReminderStatus::NotYetStarted, 0, None, 0));
        app.open_details_modal();

        app.open_delete_reminder_confirm();
        assert_eq!(app.confirm_delete_reminder, Some(ReminderId("r1".to_string())));

        let message = app.confirm_delete_reminder_command();

        assert_eq!(message, Some(ClientMessage::DeleteReminder { id: ReminderId("r1".to_string()) }));
        assert!(app.confirm_delete_reminder.is_none(), "confirming must close the dialog");
    }

    #[test]
    fn close_delete_reminder_confirm_dismisses_without_a_message() {
        let mut app = App::new();
        app.apply_snapshot(vec![agent_in("/tmp/a", "a", AgentStatus::Running)], Vec::new());
        app.apply_reminder_update(reminder_in("/tmp/a", "r1", "A", ReminderStatus::NotYetStarted, 0, None, 0));
        app.open_details_modal();
        app.open_delete_reminder_confirm();

        app.close_delete_reminder_confirm();

        assert!(app.confirm_delete_reminder.is_none());
    }
}
