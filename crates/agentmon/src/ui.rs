use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Table, TableState};
use ratatui::Frame;

use std::path::Path;

use agentmon_proto::{AgentInfo, AgentStatus, LogEntry, TestRunInfo, TestRunStatus};

use crate::app::{App, ConnectionStatus, DirectoryGroup, LogSort, Modal, Tab};

/// Background fill for the selected row in a table. A named ANSI color (not
/// `Rgb`/`Indexed`) so it - like the status colors elsewhere in this file -
/// is controlled by the terminal's own color scheme rather than a fixed
/// literal value.
const SELECTED_ROW_BG: Color = Color::Blue;

/// Foreground for the selected row's text, forced to a single readable
/// color rather than left as each status's own foreground. Applied after
/// the row's cells are rendered (see the two `row_highlight_style` uses
/// below), so it overrides `status_label_and_style`/
/// `test_run_status_cell_text_and_style` colors on the selected row instead
/// of leaving them to collide with `SELECTED_ROW_BG` (e.g. "running"'s blue
/// text would otherwise vanish against a blue background).
const SELECTED_ROW_FG: Color = Color::White;

/// The order distinct agent statuses appear in a project's AGENTS cell - a
/// fixed order so the same set of statuses always renders the same way,
/// independent of the order agents happen to be tracked in.
const AGENT_STATUS_ORDER: [AgentStatus; 6] = [
    AgentStatus::Running,
    AgentStatus::Idle,
    AgentStatus::NeedsInput,
    AgentStatus::Done,
    AgentStatus::Stale,
    AgentStatus::Declined,
];

pub fn render(frame: &mut Frame, app: &App) {
    match &app.connection {
        ConnectionStatus::Connecting => render_message(frame, "Connecting to agentd..."),
        ConnectionStatus::Unreachable(reason) => render_message(
            frame,
            &format!("agentd is not running.\n\nStart it with: agentd\n\n({reason})"),
        ),
        ConnectionStatus::Connected => render_body(frame, app, None),
        ConnectionStatus::Reconnecting => render_body(frame, app, Some("reconnecting to agentd...")),
    }
}

fn render_body(frame: &mut Frame, app: &App, banner: Option<&str>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(frame.area());

    render_tab_bar(frame, app, chunks[0]);

    match app.active_tab {
        Tab::Agents => render_agent_table(frame, app, chunks[1], banner),
        Tab::Logs => render_logs_tab(frame, app, chunks[1], banner),
    }

    if let Some(modal) = &app.modal {
        render_modal(frame, app, modal);
    }
}

/// Renders an explicit tab bar so the Agents/Logs split - and that `Tab`
/// cycles between them - is visually obvious, rather than relying on the
/// table's own border title to convey which tab is active. A bottom-only
/// border separates it from the content below without boxing it in on the
/// other three sides.
fn render_tab_bar(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::BOTTOM);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(8)])
        .split(inner);

    let tab_style = |tab: Tab| {
        if app.active_tab == tab {
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::new()
        }
    };
    let tabs_line = Line::from(vec![
        Span::styled("Agents [a]", tab_style(Tab::Agents)),
        Span::raw(" | "),
        Span::styled("Logs [l]", tab_style(Tab::Logs)),
        Span::styled("  (tab)", Style::new().fg(Color::DarkGray)),
    ]);
    frame.render_widget(Paragraph::new(tabs_line), columns[0]);

    let title = Paragraph::new(Span::styled("agentmon", Style::new().add_modifier(Modifier::BOLD)))
        .alignment(ratatui::layout::Alignment::Right);
    frame.render_widget(title, columns[1]);
}

fn render_message(frame: &mut Frame, message: &str) {
    let block = Block::default()
        .title("agentmon")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let paragraph = Paragraph::new(message).block(block);
    frame.render_widget(paragraph, frame.area());
}

fn render_agent_table(frame: &mut Frame, app: &App, area: Rect, banner: Option<&str>) {
    let header = Row::new(["PROJECT", "AGENTS", "TESTS", "UPDATED"]).style(Style::new().bold());

    let now = now_ms();
    let groups = app.directory_groups();
    let rows = groups.iter().map(|group| {
        let project = project_name(&group.cwd);
        Row::new([
            Cell::from(project),
            Cell::from(agents_status_line(group, now, true)),
            Cell::from(tests_status_line(group, now, true)),
            Cell::from(format_last_updated(group.most_recent_update_ms())),
        ])
    });

    // Agents gets a bit more than Tests (it can hold several joined status
    // segments where Tests holds at most one), and together they still
    // receive the majority of space beyond PROJECT/UPDATED (5 fill units
    // vs PROJECT's 2).
    let widths = [
        Constraint::Fill(2),
        Constraint::Fill(3),
        Constraint::Fill(2),
        Constraint::Length(19),
    ];

    let title = match banner {
        Some(banner) => format!("Agents - {banner}"),
        None => "Agents".to_string(),
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let inner = block.inner(area);

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(Style::new().bg(SELECTED_ROW_BG).fg(SELECTED_ROW_FG))
        .highlight_symbol("> ")
        .block(block);

    let selected = if groups.is_empty() { None } else { Some(app.agents_selected.min(groups.len() - 1)) };
    let mut state = TableState::default().with_selected(selected);
    frame.render_stateful_widget(table, area, &mut state);

    // "No agents tracked yet" lives in the empty table body, not the title,
    // matching the Logs tab's convention for its own empty states.
    if groups.is_empty() {
        let message_area = Rect {
            x: inner.x,
            y: inner.y + 1,
            width: inner.width,
            height: inner.height.saturating_sub(1),
        };
        frame.render_widget(
            Paragraph::new("No agents tracked yet").style(Style::new().fg(Color::DarkGray)),
            message_area,
        );
    }
}

/// Renders the Logs tab: every activity log entry the daemon has sent,
/// aggregated across projects, filtered/sorted per `App`'s current state -
/// see the "Logs tab shows an aggregated, paginated activity list" and
/// "Logs tab supports sorting and filtering" requirements.
fn render_logs_tab(frame: &mut Frame, app: &App, area: Rect, banner: Option<&str>) {
    let header = Row::new(["TIME", "PROJECT", "CATEGORY", "STATUS"]).style(Style::new().bold());

    let entries = app.visible_logs();
    let rows = entries.iter().map(|entry| {
        let (status_text, status_style) = log_entry_status_line(&app.logs, entry);
        Row::new([
            Cell::from(format_last_updated(entry.occurred_at_ms)),
            Cell::from(project_name(&entry.working_dir)),
            Cell::from(log_category_label(entry.category)),
            Cell::from(Span::styled(status_text, status_style)),
        ])
    });

    let widths = [
        Constraint::Length(19),
        Constraint::Fill(2),
        Constraint::Length(10),
        Constraint::Fill(1),
    ];

    let controls = logs_controls_hint(app);
    let title = match banner {
        Some(banner) => format!("Logs - {banner}  |  {controls}"),
        None => format!("Logs - {controls}"),
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let inner = block.inner(area);

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(Style::new().bg(SELECTED_ROW_BG).fg(SELECTED_ROW_FG))
        .highlight_symbol("> ")
        .block(block);

    let selected = if entries.is_empty() {
        None
    } else {
        Some(app.logs_selected.min(entries.len() - 1))
    };
    let mut state = TableState::default().with_selected(selected);
    frame.render_stateful_widget(table, area, &mut state);

    // Both the "nothing logged yet" and "filter matches nothing" empty
    // states are shown in the body beneath the header, not the title - the
    // title's job is the always-visible sort and filter controls, not
    // transient result state.
    if app.logs.is_empty() || entries.is_empty() {
        let message = if app.logs.is_empty() {
            "No activity logged yet"
        } else {
            "No activity matches the current filter"
        };
        let message_area = Rect {
            x: inner.x,
            y: inner.y + 1,
            width: inner.width,
            height: inner.height.saturating_sub(1),
        };
        frame.render_widget(
            Paragraph::new(message).style(Style::new().fg(Color::DarkGray)),
            message_area,
        );
    }
}

fn log_sort_label(sort: LogSort) -> &'static str {
    match sort {
        LogSort::Recency => "recency",
        LogSort::Project => "project",
        LogSort::Status => "status",
    }
}

fn log_category_label(category: agentmon_proto::LogCategory) -> &'static str {
    match category {
        agentmon_proto::LogCategory::Agent => "agent",
        agentmon_proto::LogCategory::TestRun => "test-run",
    }
}

/// Maps a log entry's category and (loosely-typed, wire-format) status
/// string back to the same emoji-marked label and color used for that
/// status in the Agents tab, so the Logs tab's STATUS column is visually
/// consistent with it rather than showing a plain, unstyled string. Always
/// includes the category-word prefix ("agent"/"tests") - every caller of
/// this function is a cross-category view (the Logs tab, or the details
/// modal's Logs pane, which mirrors it), never a category-scoped pane.
fn log_status_cell_text_and_style(category: agentmon_proto::LogCategory, status: &str) -> (String, Style) {
    match category {
        agentmon_proto::LogCategory::Agent => match status {
            "started" => agent_started_text_and_style(),
            "done" => agent_status_text_and_style(AgentStatus::Done, true),
            "needs_input" => agent_status_text_and_style(AgentStatus::NeedsInput, true),
            other => (other.to_string(), Style::new()),
        },
        agentmon_proto::LogCategory::TestRun => match status {
            "started" => test_run_started_text_and_style(),
            "passed" => test_run_status_cell_text_and_style(TestRunStatus::Passed, true),
            "failed" => test_run_status_cell_text_and_style(TestRunStatus::Failed, true),
            other => (other.to_string(), Style::new()),
        },
    }
}

/// For a completed run/task log entry - a "passed"/"failed" test-run entry
/// or a "done" agent entry - the elapsed time since the most recent
/// preceding "started" entry of the same category for the same project. Log
/// entries carry no run-start timestamp of their own (unlike the live
/// `AgentInfo`/`TestRunInfo` the Agents tab reads), so the pairing is
/// reconstructed from the log's own history.
fn log_completion_duration_ms(all_logs: &[LogEntry], entry: &LogEntry) -> Option<u64> {
    let is_completion = matches!(
        (entry.category, entry.status.as_str()),
        (agentmon_proto::LogCategory::TestRun, "passed")
            | (agentmon_proto::LogCategory::TestRun, "failed")
            | (agentmon_proto::LogCategory::Agent, "done")
    );
    if !is_completion {
        return None;
    }
    all_logs
        .iter()
        .filter(|e| e.category == entry.category)
        .filter(|e| e.working_dir == entry.working_dir)
        .filter(|e| e.status == "started")
        .filter(|e| e.occurred_at_ms <= entry.occurred_at_ms)
        .max_by_key(|e| e.occurred_at_ms)
        .map(|started| entry.occurred_at_ms.saturating_sub(started.occurred_at_ms))
}

/// Builds a log entry's styled status text, appending an elapsed duration
/// for a completed run/task so its total time is visible the same way it
/// already is on the Agents tab - see `log_completion_duration_ms`.
fn log_entry_status_line(all_logs: &[LogEntry], entry: &LogEntry) -> (String, Style) {
    let (mut text, style) = log_status_cell_text_and_style(entry.category, &entry.status);
    if let Some(duration_ms) = log_completion_duration_ms(all_logs, entry) {
        text.push(' ');
        text.push_str(&format_running_duration(0, duration_ms));
    }
    (text, style)
}

/// Appends `entry`'s reporting process id to an already-built status line,
/// for agent-category entries only - used by the details modal's Logs pane,
/// which shows pid where the top-level Logs tab does not.
fn log_entry_line_with_pid(all_logs: &[LogEntry], entry: &LogEntry) -> (String, Style) {
    let (mut text, style) = log_entry_status_line(all_logs, entry);
    if entry.category == agentmon_proto::LogCategory::Agent {
        if let Some(pid) = entry.pid {
            text.push_str(&format!("  pid {pid}"));
        }
    }
    (text, style)
}

/// Builds the always-visible sort/filter control hint shown in the Logs
/// tab's title, e.g. `Sort [o]: recency  |  Filter: none.  Project [p]
/// Status [s]  Clear [c]` - the keybinding hints stay present whether or not
/// a filter is currently applied, so the user always knows how to reach
/// them.
fn logs_controls_hint(app: &App) -> String {
    format!(
        "Sort [o]: {}  |  Filter: {}.  Project [p]  Status [s]  Clear [c]",
        log_sort_label(app.logs_sort),
        logs_filter_state(app),
    )
}

fn logs_filter_state(app: &App) -> String {
    match (&app.logs_filter_project, &app.logs_filter_status) {
        (None, None) => "none".to_string(),
        (Some(project), None) => format!("project={project}"),
        (None, Some(status)) => format!("status={status}"),
        (Some(project), Some(status)) => format!("project={project}, status={status}"),
    }
}

/// Draws `modal` centered on top of whatever tab is currently shown.
fn render_modal(frame: &mut Frame, app: &App, modal: &Modal) {
    match modal {
        Modal::Help => render_help_modal(frame),
        Modal::Details(cwd) => render_details_modal(frame, app, cwd),
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

/// Renders the selected project's details modal: three panes covering its
/// registered agents, its last test run (if any), and its recent activity
/// log entries - see the "Project details modal" requirement.
fn render_details_modal(frame: &mut Frame, app: &App, cwd: &Path) {
    let area = centered_rect(80, 80, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(format!("Details - {}", project_name(cwd)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Agents and Tests share the top third, side by side; Logs - typically
    // the longest-running list - gets the remaining two-thirds beneath them.
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)])
        .split(inner);
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
        .split(rows[0]);
    let (agents_area, tests_area, logs_area) = (top[0], top[1], rows[1]);

    let group = app.directory_groups().into_iter().find(|g| g.cwd == cwd);
    let now = now_ms();

    let agents_lines: Vec<Line> = match &group {
        Some(g) if !g.agents.is_empty() => g
            .agents
            .iter()
            .map(|a| {
                let (text, style) = status_cell_text_and_style(a, now, false);
                Line::from(vec![
                    Span::styled(text, style),
                    Span::raw(format!("  pid {}", a.pid)),
                ])
            })
            .collect(),
        _ => vec![Line::from("No agents")],
    };
    frame.render_widget(
        Paragraph::new(agents_lines).block(
            Block::default()
                .title("Agents")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        ),
        agents_area,
    );

    let tests_lines: Vec<Line> = match group.as_ref().and_then(|g| g.test_runs.first()) {
        Some(test_run) => {
            let (label, style) = test_run_status_cell_text_and_style(test_run.status, false);
            let text = format!("{label} {}", format_test_run_duration(test_run, now));
            vec![Line::from(Span::styled(text, style))]
        }
        None => vec![Line::from("No test run")],
    };
    frame.render_widget(
        Paragraph::new(tests_lines).block(
            Block::default()
                .title("Tests")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        ),
        tests_area,
    );

    let mut project_logs: Vec<&LogEntry> = app.logs.iter().filter(|e| e.working_dir == cwd).collect();
    project_logs.sort_by_key(|e| std::cmp::Reverse(e.occurred_at_ms));
    let logs_lines: Vec<Line> = if project_logs.is_empty() {
        vec![Line::from("No activity")]
    } else {
        project_logs
            .iter()
            .map(|entry| {
                let (status_text, style) = log_entry_line_with_pid(&app.logs, entry);
                Line::from(vec![
                    Span::raw(format!("{} ", format_last_updated(entry.occurred_at_ms))),
                    Span::styled(status_text, style),
                ])
            })
            .collect()
    };
    frame.render_widget(
        Paragraph::new(logs_lines).block(
            Block::default()
                .title("Logs")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        ),
        logs_area,
    );
}

/// Renders the keyboard-shortcuts help overlay - see the "Keyboard shortcuts
/// help modal" requirement.
fn render_help_modal(frame: &mut Frame) {
    let area = centered_rect(60, 60, frame.area());
    frame.render_widget(Clear, area);

    let text = [
        "Tab       switch tabs",
        "A / L     jump to Agents / Logs tab",
        "j / down  move selection down",
        "k / up    move selection up",
        "d / PgDn  page down (Logs tab)",
        "u / PgUp  page up (Logs tab)",
        "Enter     open project details (Agents or Logs tab)",
        "o         cycle log sort (Logs tab)",
        "p         cycle project filter (Logs tab)",
        "s         cycle status filter (Logs tab)",
        "c         clear log filters (Logs tab)",
        "?         toggle this help",
        "Esc       close modal",
        "q         quit",
    ]
    .join("\n");

    frame.render_widget(
        Paragraph::new(text).block(
            Block::default()
                .title("Keyboard Shortcuts")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        ),
        area,
    );
}

/// Formats a unix-epoch-milliseconds timestamp in the system's local
/// timezone, without a UTC offset or timezone abbreviation.
fn format_last_updated(last_updated_ms: u64) -> String {
    let datetime = chrono::DateTime::from_timestamp_millis(last_updated_ms as i64)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp_millis(0).unwrap());
    datetime
        .with_timezone(&chrono::Local)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Formats the elapsed time since `status_since_ms` as a compact counter:
/// `9s` under a minute, `2m14s` under an hour, `1h03m` at an hour or more
/// (seconds are dropped once hours are shown, since they stop being useful).
fn format_running_duration(status_since_ms: u64, now_ms: u64) -> String {
    let elapsed_secs = now_ms.saturating_sub(status_since_ms) / 1000;
    let hours = elapsed_secs / 3600;
    let minutes = (elapsed_secs % 3600) / 60;
    let seconds = elapsed_secs % 60;

    if hours > 0 {
        format!("{hours}h{minutes:02}m")
    } else if minutes > 0 {
        format!("{minutes}m{seconds:02}s")
    } else {
        format!("{seconds}s")
    }
}

fn project_name(cwd: &Path) -> String {
    cwd.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| cwd.display().to_string())
}

/// Builds a project's AGENTS cell: one styled segment per distinct agent
/// status present (in `AGENT_STATUS_ORDER`, so the same set of statuses
/// always renders in the same order), joined by " · " so no status is
/// hidden behind another, per the "Status is visually distinguishable"
/// requirement.
fn agents_status_line(group: &DirectoryGroup, now_ms: u64, with_category_prefix: bool) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();

    for &status in AGENT_STATUS_ORDER.iter() {
        let matching: Vec<&AgentInfo> = group.agents.iter().filter(|a| a.status == status).collect();
        let Some(&first) = matching.first() else {
            continue;
        };
        if !spans.is_empty() {
            spans.push(Span::raw(" · "));
        }
        // A duration is only attached when exactly one agent holds this
        // status - with two or more, there's no single elapsed time that
        // isn't arbitrary to pick, so the segment shows just the label.
        let (text, style) = if matches!(status, AgentStatus::Running | AgentStatus::Done) && matching.len() == 1 {
            status_cell_text_and_style(first, now_ms, with_category_prefix)
        } else {
            agent_status_text_and_style(status, with_category_prefix)
        };
        spans.push(Span::styled(text, style));
    }

    Line::from(spans)
}

/// Builds a project's TESTS cell: its test run's status and duration, or
/// empty if the project has no tracked test run.
fn tests_status_line(group: &DirectoryGroup, now_ms: u64, with_category_prefix: bool) -> Line<'static> {
    match group.test_runs.first() {
        Some(test_run) => {
            let (label, style) = test_run_status_cell_text_and_style(test_run.status, with_category_prefix);
            let duration = format_test_run_duration(test_run, now_ms);
            Line::from(Span::styled(format!("{label} {duration}"), style))
        }
        None => Line::from(""),
    }
}

/// A test run's duration: live and counting up while "running" (computed
/// against the current time, like a running agent's), and the fixed total
/// elapsed time once "passed" or "failed" (computed against its own
/// last-updated time instead, so it stops advancing once the run is done).
fn format_test_run_duration(test_run: &TestRunInfo, now_ms: u64) -> String {
    match test_run.status {
        TestRunStatus::Running => format_running_duration(test_run.run_started_ms, now_ms),
        TestRunStatus::Passed | TestRunStatus::Failed => {
            format_running_duration(test_run.run_started_ms, test_run.last_updated_ms)
        }
    }
}

/// The STATUS cell's text and style for one agent: the running status gets
/// an appended live elapsed-duration counter and the done status gets an
/// appended fixed total-duration counter (see `format_running_duration`);
/// every other status renders as just its label.
fn status_cell_text_and_style(agent: &AgentInfo, now_ms: u64, with_category_prefix: bool) -> (String, Style) {
    let (mut text, style) = agent_status_text_and_style(agent.status, with_category_prefix);
    match agent.status {
        AgentStatus::Running => {
            text.push(' ');
            text.push_str(&format_running_duration(agent.status_since_ms, now_ms));
        }
        AgentStatus::Done => {
            text.push(' ');
            text.push_str(&format_running_duration(agent.run_started_ms, agent.status_since_ms));
        }
        _ => {}
    }
    (text, style)
}

/// Every status gets both a distinct label and a distinct style, so the
/// distinction survives even in a plain-text rendering (as asserted by
/// tests) and not only through color. Bare - carries no category-word
/// prefix; see `agent_status_text_and_style` for that.
fn status_label_and_style(status: AgentStatus) -> (&'static str, Style) {
    match status {
        AgentStatus::Running => ("🔧 running", Style::new().fg(Color::Blue)),
        AgentStatus::Idle => ("💤 idle", Style::new().fg(Color::Gray)),
        AgentStatus::NeedsInput => (
            "🔔 NEEDS INPUT",
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
        AgentStatus::Done => ("✅ done", Style::new().fg(Color::Green)),
        AgentStatus::Stale => (
            // 🕸️ (U+1F578 + VS16) rendered half-width in some terminal
            // fonts, corrupting the selected-row highlight; 👻 (U+1F47B) has
            // default emoji presentation with no variation selector needed,
            // so it doesn't depend on a terminal correctly honoring VS16.
            "👻 stale",
            Style::new()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM),
        ),
        AgentStatus::Declined => ("🚫 declined", Style::new().fg(Color::Red)),
    }
}

/// Inserts `category_word` right after `label`'s leading emoji and before
/// its status word (e.g. `("🔧 running", "agent")` -> `"🔧 agent running"`),
/// per the "Status labels are prefixed by category outside dedicated panes"
/// requirement.
fn with_category_word(label: &str, category_word: &str) -> String {
    match label.split_once(' ') {
        Some((emoji, rest)) => format!("{emoji} {category_word} {rest}"),
        None => format!("{label} {category_word}"),
    }
}

/// An agent status's label and style, with the "agent" category-word prefix
/// applied when `with_category_prefix` is set - true for the Agents tab and
/// the Logs tab/pane, false for the details modal's Agents pane.
fn agent_status_text_and_style(status: AgentStatus, with_category_prefix: bool) -> (String, Style) {
    let (label, style) = status_label_and_style(status);
    let text = if with_category_prefix {
        with_category_word(label, "agent")
    } else {
        label.to_string()
    };
    (text, style)
}

/// The label and style for a log-only "agent started" entry - not a real
/// `AgentStatus` variant (see the activity-log spec's "started" capture),
/// so it has no bare/pane form: it only ever appears in a cross-category
/// view (the Logs tab or the details modal's Logs pane), never in the
/// Agents tab's AGENTS column or the modal's Agents pane.
fn agent_started_text_and_style() -> (String, Style) {
    ("⏳ agent started".to_string(), Style::new().fg(Color::Blue))
}

/// The Logs tab/pane's one-time "a test run began" log entry, independent of
/// `TestRunStatus` - mirrors `agent_started_text_and_style` exactly: this
/// label always carries the "tests" category-word prefix, so it has no
/// bare/pane form; it only ever appears in a cross-category view, never in
/// the Agents tab's TESTS column or the modal's Tests pane (which show the
/// live `TestRunStatus::Running` status as "running" instead).
fn test_run_started_text_and_style() -> (String, Style) {
    ("⏳ tests started".to_string(), Style::new().fg(Color::Blue))
}

/// The STATUS cell's text and style for a test run, with the "tests"
/// category-word prefix applied when `with_category_prefix` is set - true
/// for the Agents tab and the Logs tab/pane, false for the details modal's
/// Tests pane.
fn test_run_status_cell_text_and_style(status: TestRunStatus, with_category_prefix: bool) -> (String, Style) {
    let (label, style) = match status {
        TestRunStatus::Running => ("⏳ running", Style::new().fg(Color::Blue)),
        TestRunStatus::Passed => ("✅ passed", Style::new().fg(Color::Green)),
        TestRunStatus::Failed => (
            "❌ failed",
            Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
    };
    let text = if with_category_prefix {
        with_category_word(label, "tests")
    } else {
        label.to_string()
    };
    (text, style)
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentmon_proto::{HostContext, SessionId};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::path::PathBuf;

    fn terminal() -> Terminal<TestBackend> {
        Terminal::new(TestBackend::new(100, 10)).unwrap()
    }

    fn agent_in(cwd: &str, id: &str, status: AgentStatus, host: HostContext, pid: u32, last_updated_ms: u64) -> AgentInfo {
        AgentInfo {
            session_id: SessionId(id.to_string()),
            cwd: PathBuf::from(cwd),
            host_context: host,
            pid,
            status,
            last_updated_ms,
            status_since_ms: last_updated_ms,
            run_started_ms: last_updated_ms,
        }
    }

    fn agent(id: &str, status: AgentStatus, host: HostContext, pid: u32, last_updated_ms: u64) -> AgentInfo {
        agent_in("/Users/beet/project", id, status, host, pid, last_updated_ms)
    }

    fn agent_with_status_since(
        id: &str,
        status: AgentStatus,
        host: HostContext,
        pid: u32,
        last_updated_ms: u64,
        status_since_ms: u64,
    ) -> AgentInfo {
        AgentInfo {
            status_since_ms,
            ..agent(id, status, host, pid, last_updated_ms)
        }
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

    /// Finds the first cell in the buffer (scanning top-to-bottom,
    /// left-to-right) whose symbol matches `needle`, so style-comparison
    /// tests don't need to hardcode a row index that shifts whenever the
    /// surrounding layout changes (e.g. the tab bar's height).
    fn find_cell<'a>(
        buffer: &'a ratatui::buffer::Buffer,
        needle: &str,
    ) -> Option<&'a ratatui::buffer::Cell> {
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                if buffer[(x, y)].symbol() == needle {
                    return Some(&buffer[(x, y)]);
                }
            }
        }
        None
    }

    /// Finds the top-left position of the first row containing `needle` as a
    /// contiguous substring, so tests can check layout (e.g. "this text is
    /// right of that text, on the same row") without relying on a single
    /// ambiguous character.
    fn find_text(buffer: &ratatui::buffer::Buffer, needle: &str) -> Option<(u16, u16)> {
        find_text_from(buffer, needle, 0)
    }

    /// Like `find_text`, but only considers rows at or after `min_y` - useful
    /// when `needle` also appears elsewhere on screen (e.g. a pane title
    /// that happens to match the tab bar's label) and the test only cares
    /// about the occurrence within a specific area.
    fn find_text_from(buffer: &ratatui::buffer::Buffer, needle: &str, min_y: u16) -> Option<(u16, u16)> {
        for y in min_y..buffer.area.height {
            let mut row = String::new();
            for x in 0..buffer.area.width {
                row.push_str(buffer[(x, y)].symbol());
            }
            if let Some(byte_idx) = row.find(needle) {
                let x = row[..byte_idx].chars().count() as u16;
                return Some((x, y));
            }
        }
        None
    }

    #[test]
    fn unreachable_daemon_renders_a_clear_message() {
        let mut term = terminal();
        let mut app = App::new();
        app.set_unreachable("connection refused".to_string());

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("agentd is not running"), "got:\n{text}");
        assert!(text.contains("Start it with: agentd"), "got:\n{text}");
    }

    #[test]
    fn agents_tab_shows_a_gray_placeholder_in_the_table_body_when_empty() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());

        term.draw(|frame| render(frame, &app)).unwrap();

        let buffer = term.backend().buffer();
        let text = buffer_text(&term);
        assert!(
            !text.contains("no agents tracked yet"),
            "the empty-agents message must not appear in the title, got:\n{text}"
        );
        let (msg_x, msg_y) = find_text(buffer, "No agents tracked yet").expect("got:\n{text}");
        assert_eq!(
            buffer[(msg_x, msg_y)].fg,
            Color::DarkGray,
            "the empty-agents message should be shown in gray in the table body"
        );
    }

    #[test]
    fn seeded_agents_are_rendered_in_the_table() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(vec![agent("s", AgentStatus::Running, HostContext::Nvim, 4242, 0)], Vec::new());

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("project"), "expected project name, got:\n{text}");
        assert!(text.contains("running"), "expected status, got:\n{text}");
    }

    #[test]
    fn a_running_status_stays_legible_on_the_default_selected_row() {
        let mut term = terminal();
        let mut app = App::new();
        // A single agent's row is the only one shown, so it's the row
        // selected by default - before the user has pressed any navigation
        // key - exercising the same highlighted-on-startup case a freshly
        // started agent hits.
        app.apply_snapshot(vec![agent("s", AgentStatus::Running, HostContext::Nvim, 4242, 0)], Vec::new());

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("🔧"), "expected the running emoji to render, got:\n{text}");
        assert!(text.contains("running"), "expected the running label to render, got:\n{text}");

        let buffer = term.backend().buffer();
        let (x, y) = find_text(buffer, "running").expect("running status text should be rendered");
        let running_cell = &buffer[(x, y)];
        assert_eq!(
            running_cell.bg,
            SELECTED_ROW_BG,
            "the only row should be highlighted as selected by default"
        );
        assert_eq!(
            running_cell.fg,
            SELECTED_ROW_FG,
            "the selected row's text must be forced to the selected-row foreground, \
             overriding the running status's own color, so it stays visible against \
             the selected-row background"
        );
    }

    #[test]
    fn borders_are_rounded_everywhere_except_the_tab_bars_underline() {
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(
            vec![agent("a", AgentStatus::Running, HostContext::Terminal, 4242, 0)],
            Vec::new(),
        );

        // Agents tab.
        term.draw(|frame| render(frame, &app)).unwrap();
        let text = buffer_text(&term);
        assert!(text.contains('╭'), "Agents tab should use rounded corners, got:\n{text}");
        assert!(!text.contains('┌'), "no sharp corners should remain, got:\n{text}");

        // Logs tab.
        app.set_tab(Tab::Logs);
        term.draw(|frame| render(frame, &app)).unwrap();
        let text = buffer_text(&term);
        assert!(text.contains('╭'), "Logs tab should use rounded corners, got:\n{text}");

        // Details modal (Agents/Tests/Logs panes, plus the outer frame).
        app.set_tab(Tab::Agents);
        app.open_details_modal();
        term.draw(|frame| render(frame, &app)).unwrap();
        let text = buffer_text(&term);
        let rounded_corners = text.matches('╭').count();
        assert!(
            rounded_corners >= 4,
            "expected at least 4 rounded top-left corners (outer frame + 3 panes), got {rounded_corners} in:\n{text}"
        );
        assert!(!text.contains('┌'), "no sharp corners should remain in the modal, got:\n{text}");

        // Help modal.
        app.close_modal();
        app.open_help_modal();
        term.draw(|frame| render(frame, &app)).unwrap();
        let text = buffer_text(&term);
        assert!(text.contains('╭'), "help modal should use rounded corners, got:\n{text}");

        // The tab bar's bottom-only underline has no corners to round - it
        // must still just be a plain horizontal line.
        assert!(
            find_text(term.backend().buffer(), "─").is_some(),
            "tab bar underline should still be a plain horizontal line"
        );
    }

    #[test]
    fn the_table_has_no_host_or_pid_columns() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(vec![agent("s", AgentStatus::Running, HostContext::Nvim, 4242, 0)], Vec::new());

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("PROJECT"), "got:\n{text}");
        assert!(text.contains("AGENTS"), "got:\n{text}");
        assert!(text.contains("TESTS"), "got:\n{text}");
        assert!(text.contains("UPDATED"), "got:\n{text}");
        assert!(!text.contains("HOST"), "HOST column should be removed, got:\n{text}");
        assert!(!text.contains("PID"), "PID column should be removed, got:\n{text}");
        assert!(!text.contains("nvim"), "host label should not be rendered, got:\n{text}");
        assert!(!text.contains("4242"), "pid should not be rendered, got:\n{text}");
    }

    #[test]
    fn a_long_project_name_uses_more_than_20_characters_on_a_wide_terminal() {
        let mut term = Terminal::new(TestBackend::new(200, 10)).unwrap();
        let mut app = App::new();
        let long_name = "this-is-a-very-long-project-directory-name";
        app.apply_snapshot(
            vec![agent_in(
                &format!("/tmp/{long_name}"),
                "s",
                AgentStatus::Running,
                HostContext::Terminal,
                1,
                0,
            )],
            Vec::new(),
        );

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(
            text.contains(long_name),
            "expected the full project name to render on a wide terminal, got:\n{text}"
        );
    }

    #[test]
    fn a_long_project_name_is_clipped_on_a_narrow_terminal() {
        let mut term = Terminal::new(TestBackend::new(40, 10)).unwrap();
        let mut app = App::new();
        let long_name = "this-is-a-very-long-project-directory-name";
        app.apply_snapshot(
            vec![agent_in(
                &format!("/tmp/{long_name}"),
                "s",
                AgentStatus::Running,
                HostContext::Terminal,
                1,
                0,
            )],
            Vec::new(),
        );

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(
            !text.contains(long_name),
            "expected the name not to fit in full on a narrow terminal, got:\n{text}"
        );
    }

    #[test]
    fn needs_input_is_visually_distinguished_from_other_statuses() {
        // Wider than the default test terminal: this row combines two
        // agent statuses (with the "agent " category-word prefix) in one
        // AGENTS cell, which needs more room than a single status does.
        let mut term = Terminal::new(TestBackend::new(140, 10)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(
            vec![
                // A more recently updated agent in another project sorts first and
                // becomes the default selection instead, so the row under test here
                // isn't repainted with the selected-row foreground override.
                agent_in("/Users/beet/other-project", "c", AgentStatus::Idle, HostContext::Terminal, 4244, 5000),
                // status_since_ms close to "now" keeps the running duration
                // short, so it plus the "agent "/"NEEDS INPUT" segment still
                // fits the AGENTS column's test-terminal width.
                agent_with_status_since("a", AgentStatus::Running, HostContext::Terminal, 4242, 0, now_ms()),
                agent("b", AgentStatus::NeedsInput, HostContext::Terminal, 4243, 0),
            ],
            Vec::new(),
        );

        term.draw(|frame| render(frame, &app)).unwrap();

        // Distinguished by label text - both statuses combine into the same
        // project's single row...
        let text = buffer_text(&term);
        assert!(text.contains("NEEDS INPUT"));
        assert!(text.contains("running"));

        // ...and by style: locate the "NEEDS INPUT" span and confirm its
        // foreground color differs from the "running" span's. Matched by
        // whole word (not a single ambiguous character) so the match can't
        // land on an unrelated cell, such as the "r" in "other-project".
        let buffer = term.backend().buffer();
        let (ni_x, ni_y) = find_text(buffer, "NEEDS INPUT").expect("NEEDS INPUT should be rendered");
        let (r_x, r_y) = find_text(buffer, "running").expect("running should be rendered");
        assert_ne!(
            buffer[(ni_x, ni_y)].fg,
            buffer[(r_x, r_y)].fg,
            "needs-input styling must differ from running styling"
        );
    }

    #[test]
    fn projects_are_rendered_most_recently_updated_first() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(
            vec![
                agent_in("/tmp/older-project", "older", AgentStatus::Running, HostContext::Terminal, 1111, 1_000),
                agent_in("/tmp/newer-project", "newer", AgentStatus::Running, HostContext::Terminal, 2222, 2_000),
            ],
            Vec::new(),
        );

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        let newer_pos = text.find("newer-project").expect("newer project should be rendered");
        let older_pos = text.find("older-project").expect("older project should be rendered");
        assert!(
            newer_pos < older_pos,
            "more recently updated project should render first:\n{text}"
        );
    }

    #[test]
    fn an_update_moves_the_updated_project_to_the_top() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(
            vec![
                agent_in("/tmp/project-a", "a", AgentStatus::Running, HostContext::Terminal, 1111, 1_000),
                agent_in("/tmp/project-b", "b", AgentStatus::Running, HostContext::Terminal, 2222, 2_000),
            ],
            Vec::new(),
        );
        // "project-a" starts below "project-b"; a fresh update should move it
        // back to the top.
        app.apply_update(agent_in(
            "/tmp/project-a",
            "a",
            AgentStatus::Running,
            HostContext::Terminal,
            1111,
            3_000,
        ));

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        let a_pos = text.find("project-a").expect("project-a should be rendered");
        let b_pos = text.find("project-b").expect("project-b should be rendered");
        assert!(
            a_pos < b_pos,
            "the just-updated project should move to the top of the table:\n{text}"
        );
    }

    #[test]
    fn last_updated_column_shows_local_time() {
        let mut term = terminal();
        let mut app = App::new();
        let last_updated_ms: u64 = 1_700_000_000_000;
        app.apply_snapshot(
            vec![agent(
                "s",
                AgentStatus::Running,
                HostContext::Terminal,
                4242,
                last_updated_ms,
            )],
            Vec::new(),
        );

        term.draw(|frame| render(frame, &app)).unwrap();

        // Computed independently at test time (not hardcoded) since the
        // expected string depends on the machine's local timezone.
        let expected = chrono::DateTime::from_timestamp_millis(last_updated_ms as i64)
            .unwrap()
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        let text = buffer_text(&term);
        assert!(text.contains("UPDATED"), "expected an UPDATED column header, got:\n{text}");
        assert!(
            text.contains(&expected),
            "expected local timestamp {expected:?}, got:\n{text}"
        );
    }

    #[test]
    fn format_running_duration_under_a_minute() {
        assert_eq!(format_running_duration(0, 9_000), "9s");
    }

    #[test]
    fn format_running_duration_under_an_hour() {
        assert_eq!(format_running_duration(0, 134_000), "2m14s");
    }

    #[test]
    fn format_running_duration_an_hour_or_more() {
        assert_eq!(format_running_duration(0, 3_780_000), "1h03m");
    }

    #[test]
    fn a_running_agent_shows_its_elapsed_duration() {
        let running = agent_with_status_since(
            "s",
            AgentStatus::Running,
            HostContext::Terminal,
            4242,
            1_000_000,
            866_000,
        );

        let (text, _) = status_cell_text_and_style(&running, 1_000_000, false);

        assert_eq!(text, "🔧 running 2m14s");
    }

    #[test]
    fn a_non_running_agent_shows_no_duration() {
        let idle = agent("s", AgentStatus::Idle, HostContext::Terminal, 4242, 0);

        let (text, _) = status_cell_text_and_style(&idle, 1_000_000, false);

        assert_eq!(text, "💤 idle");
    }

    #[test]
    fn a_declined_agent_shows_no_duration() {
        let declined = agent("s", AgentStatus::Declined, HostContext::Terminal, 4242, 0);

        let (text, _) = status_cell_text_and_style(&declined, 1_000_000, false);

        assert_eq!(text, "🚫 declined");
    }

    #[test]
    fn a_done_agent_shows_its_total_duration() {
        let done = AgentInfo {
            run_started_ms: 866_000,
            status_since_ms: 1_000_000,
            ..agent("s", AgentStatus::Done, HostContext::Terminal, 4242, 1_000_000)
        };

        let (text, _) = status_cell_text_and_style(&done, 999_999_999, false);

        assert_eq!(
            text, "✅ done 2m14s",
            "done's duration must be fixed (status_since - run_started), not computed against `now`"
        );
    }

    #[test]
    fn status_cell_text_and_style_applies_the_category_prefix_when_requested() {
        let running = agent("s", AgentStatus::Running, HostContext::Terminal, 4242, 0);

        let (text, _) = status_cell_text_and_style(&running, 0, true);

        assert_eq!(text, "🔧 agent running 0s");
    }

    #[test]
    fn declined_is_visually_distinguished_from_other_statuses() {
        // Wider than the default test terminal, for the same reason as
        // `needs_input_is_visually_distinguished_from_other_statuses`.
        let mut term = Terminal::new(TestBackend::new(140, 10)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(
            vec![
                // A more recently updated agent in another project sorts first and
                // becomes the default selection instead, so the row under test here
                // isn't repainted with the selected-row foreground override.
                agent_in("/Users/beet/other-project", "c", AgentStatus::Idle, HostContext::Terminal, 4244, 5000),
                agent_with_status_since("a", AgentStatus::Running, HostContext::Terminal, 4242, 0, now_ms()),
                agent("b", AgentStatus::Declined, HostContext::Terminal, 4243, 0),
            ],
            Vec::new(),
        );

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("declined"), "expected declined status, got:\n{text}");
        assert!(text.contains("running"), "expected running status, got:\n{text}");

        // Matched by whole word (not a single ambiguous character) so the
        // match can't land on an unrelated cell, such as the "d" in "idle".
        let buffer = term.backend().buffer();
        let (d_x, d_y) = find_text(buffer, "declined").expect("declined should be rendered");
        let (r_x, r_y) = find_text(buffer, "running").expect("running should be rendered");
        assert_ne!(
            buffer[(d_x, d_y)].fg,
            buffer[(r_x, r_y)].fg,
            "declined styling must differ from running styling"
        );
    }

    #[test]
    fn a_running_agent_row_includes_a_duration_in_the_rendered_table() {
        let mut term = terminal();
        let mut app = App::new();
        let now = now_ms();
        app.apply_snapshot(
            vec![agent_with_status_since(
                "s",
                AgentStatus::Running,
                HostContext::Terminal,
                4242,
                now,
                now - 134_000,
            )],
            Vec::new(),
        );

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(
            text.contains("2m1"),
            "expected a running duration around 2m14s, got:\n{text}"
        );
    }

    #[test]
    fn two_agents_with_the_same_status_produce_one_status_segment() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(
            vec![
                agent("a", AgentStatus::Running, HostContext::Terminal, 4242, 0),
                agent("b", AgentStatus::Running, HostContext::Terminal, 4243, 0),
            ],
            Vec::new(),
        );

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert_eq!(
            text.matches("running").count(),
            1,
            "two agents sharing a status must collapse to a single segment, got:\n{text}"
        );
    }

    #[test]
    fn two_agents_with_different_statuses_show_both_in_one_row() {
        // Wider than the default test terminal, for the same reason as
        // `needs_input_is_visually_distinguished_from_other_statuses`.
        let mut term = Terminal::new(TestBackend::new(140, 10)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(
            vec![
                agent_with_status_since("a", AgentStatus::Running, HostContext::Terminal, 4242, 0, now_ms()),
                agent("b", AgentStatus::NeedsInput, HostContext::Terminal, 4243, 0),
            ],
            Vec::new(),
        );

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("running"), "got:\n{text}");
        assert!(text.contains("NEEDS INPUT"), "got:\n{text}");
        assert!(
            text.contains(" · "),
            "expected the distinct statuses joined by a separator, got:\n{text}"
        );
    }

    fn test_run(pid: u32, status: TestRunStatus, last_updated_ms: u64) -> agentmon_proto::TestRunInfo {
        test_run_with_start(pid, status, last_updated_ms, last_updated_ms)
    }

    fn test_run_with_start(
        pid: u32,
        status: TestRunStatus,
        run_started_ms: u64,
        last_updated_ms: u64,
    ) -> agentmon_proto::TestRunInfo {
        agentmon_proto::TestRunInfo {
            cwd: PathBuf::from("/Users/beet/project"),
            pid,
            status,
            last_updated_ms,
            run_started_ms,
        }
    }

    #[test]
    fn a_directory_with_no_tracked_agent_still_shows_its_test_run() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_test_run_update(test_run(999, TestRunStatus::Running, 0));

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("project"), "expected project name, got:\n{text}");
        assert!(text.contains("tests running"), "expected test-run status, got:\n{text}");
    }

    #[test]
    fn a_test_run_appears_alongside_its_directorys_agent() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(
            vec![agent("a", AgentStatus::Running, HostContext::Terminal, 4242, 0)],
            vec![test_run(999, TestRunStatus::Failed, 0)],
        );

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("running"), "expected the agent row, got:\n{text}");
        assert!(text.contains("tests failed"), "expected the test-run row, got:\n{text}");
    }

    #[test]
    fn each_test_run_status_has_a_distinct_emoji_marker() {
        assert_eq!(
            test_run_status_cell_text_and_style(TestRunStatus::Running, true).0,
            "⏳ tests running"
        );
        assert_eq!(
            test_run_status_cell_text_and_style(TestRunStatus::Passed, true).0,
            "✅ tests passed"
        );
        assert_eq!(
            test_run_status_cell_text_and_style(TestRunStatus::Failed, true).0,
            "❌ tests failed"
        );
    }

    #[test]
    fn test_run_status_cell_omits_the_category_prefix_when_not_requested() {
        assert_eq!(
            test_run_status_cell_text_and_style(TestRunStatus::Running, false).0,
            "⏳ running"
        );
        assert_eq!(
            test_run_status_cell_text_and_style(TestRunStatus::Passed, false).0,
            "✅ passed"
        );
        assert_eq!(
            test_run_status_cell_text_and_style(TestRunStatus::Failed, false).0,
            "❌ failed"
        );
    }

    #[test]
    fn a_failing_test_run_is_visually_distinguished() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(
            vec![agent("a", AgentStatus::Running, HostContext::Terminal, 4242, 0)],
            vec![test_run(999, TestRunStatus::Failed, 0)],
        );

        term.draw(|frame| render(frame, &app)).unwrap();

        let (_, failed_style) = test_run_status_cell_text_and_style(TestRunStatus::Failed, true);
        let (_, running_style) = status_label_and_style(AgentStatus::Running);
        assert_ne!(
            failed_style.fg, running_style.fg,
            "a failed test run's styling must differ from a running agent's"
        );
    }

    #[test]
    fn an_agent_status_and_test_run_status_are_shown_together() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(
            vec![agent("a", AgentStatus::Running, HostContext::Terminal, 4242, 0)],
            vec![test_run(999, TestRunStatus::Failed, 0)],
        );

        term.draw(|frame| render(frame, &app)).unwrap();

        let buffer = term.backend().buffer();
        let text = buffer_text(&term);
        assert!(text.contains("running"), "got:\n{text}");
        assert!(text.contains("tests failed"), "got:\n{text}");
        // Each status now lives in its own column (AGENTS vs TESTS) rather
        // than sharing one cell joined by a separator.
        let (running_x, running_y) = find_text(buffer, "running").expect("running should be rendered");
        let (failed_x, failed_y) = find_text(buffer, "tests failed").expect("tests failed should be rendered");
        assert_eq!(running_y, failed_y, "both statuses belong to the same project row");
        assert!(
            failed_x > running_x,
            "the Tests column's status must render to the right of the Agents column's"
        );
    }

    #[test]
    fn format_test_run_duration_for_a_running_run_is_live() {
        let running = test_run_with_start(1, TestRunStatus::Running, 0, 0);

        assert_eq!(format_test_run_duration(&running, 9_000), "9s");
    }

    #[test]
    fn format_test_run_duration_for_a_finished_run_is_fixed_regardless_of_now() {
        let passed = test_run_with_start(1, TestRunStatus::Passed, 0, 134_000);

        assert_eq!(format_test_run_duration(&passed, 999_999_999), "2m14s");
    }

    #[test]
    fn a_running_test_run_shows_a_live_elapsed_duration() {
        let mut term = terminal();
        let mut app = App::new();
        let now = now_ms();
        app.apply_test_run_update(test_run_with_start(999, TestRunStatus::Running, now - 134_000, now));

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(
            text.contains("2m1"),
            "expected a live duration around 2m14s, got:\n{text}"
        );
    }

    #[test]
    fn a_passed_test_run_shows_its_total_duration() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_test_run_update(test_run_with_start(999, TestRunStatus::Passed, 0, 134_000));

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("tests passed"), "got:\n{text}");
        assert!(text.contains("2m14s"), "expected the total run duration, got:\n{text}");
    }

    #[test]
    fn a_failed_test_run_shows_its_total_duration() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_test_run_update(test_run_with_start(999, TestRunStatus::Failed, 0, 45_000));

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("tests failed"), "got:\n{text}");
        assert!(text.contains("45s"), "expected the total run duration, got:\n{text}");
    }

    #[test]
    fn updated_column_shows_the_most_recent_member_timestamp() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(
            vec![agent("a", AgentStatus::Running, HostContext::Terminal, 4242, 1_000)],
            vec![test_run(999, TestRunStatus::Failed, 2_000)],
        );

        term.draw(|frame| render(frame, &app)).unwrap();

        let expected = chrono::DateTime::from_timestamp_millis(2_000)
            .unwrap()
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        let text = buffer_text(&term);
        assert!(
            text.contains(&expected),
            "expected the more recent member's timestamp {expected:?}, got:\n{text}"
        );
    }

    fn log_entry(cwd: &str, category: agentmon_proto::LogCategory, status: &str, occurred_at_ms: u64) -> LogEntry {
        LogEntry {
            working_dir: PathBuf::from(cwd),
            category,
            status: status.to_string(),
            occurred_at_ms,
            pid: Some(1),
        }
    }

    #[test]
    fn logs_tab_lists_entries_across_projects() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.set_tab(crate::app::Tab::Logs);
        app.apply_log_snapshot(vec![
            log_entry("/tmp/project-a", agentmon_proto::LogCategory::Agent, "done", 1_000),
            log_entry("/tmp/project-b", agentmon_proto::LogCategory::TestRun, "failed", 2_000),
        ]);

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("project-a"), "got:\n{text}");
        assert!(text.contains("project-b"), "got:\n{text}");
        assert!(text.contains("done"), "got:\n{text}");
        assert!(text.contains("failed"), "got:\n{text}");
    }

    #[test]
    fn log_status_cell_uses_the_same_emoji_markers_as_the_agents_tab() {
        assert_eq!(
            log_status_cell_text_and_style(agentmon_proto::LogCategory::Agent, "started").0,
            "⏳ agent started"
        );
        assert_eq!(
            log_status_cell_text_and_style(agentmon_proto::LogCategory::Agent, "done").0,
            "✅ agent done"
        );
        assert_eq!(
            log_status_cell_text_and_style(agentmon_proto::LogCategory::Agent, "needs_input").0,
            "🔔 agent NEEDS INPUT"
        );
        assert_eq!(
            log_status_cell_text_and_style(agentmon_proto::LogCategory::TestRun, "started").0,
            "⏳ tests started"
        );
        assert_eq!(
            log_status_cell_text_and_style(agentmon_proto::LogCategory::TestRun, "passed").0,
            "✅ tests passed"
        );
        assert_eq!(
            log_status_cell_text_and_style(agentmon_proto::LogCategory::TestRun, "failed").0,
            "❌ tests failed"
        );
    }

    #[test]
    fn log_status_cell_style_matches_the_agents_tab() {
        assert_eq!(
            log_status_cell_text_and_style(agentmon_proto::LogCategory::Agent, "done").1,
            status_label_and_style(AgentStatus::Done).1
        );
        assert_eq!(
            log_status_cell_text_and_style(agentmon_proto::LogCategory::TestRun, "failed").1,
            test_run_status_cell_text_and_style(TestRunStatus::Failed, true).1
        );
    }

    #[test]
    fn logs_tab_status_column_renders_the_emoji_marked_labels() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.set_tab(crate::app::Tab::Logs);
        app.apply_log_snapshot(vec![
            log_entry("/tmp/project-a", agentmon_proto::LogCategory::Agent, "needs_input", 1_000),
            log_entry("/tmp/project-b", agentmon_proto::LogCategory::TestRun, "failed", 2_000),
        ]);

        term.draw(|frame| render(frame, &app)).unwrap();

        // Emoji are double-width in the terminal buffer, so check the glyph
        // and its trailing label as separate substrings rather than one
        // contiguous string spanning the wide-character boundary.
        let text = buffer_text(&term);
        assert!(text.contains('🔔'), "got:\n{text}");
        assert!(text.contains("NEEDS INPUT"), "got:\n{text}");
        assert!(text.contains('❌'), "got:\n{text}");
        assert!(text.contains("tests failed"), "got:\n{text}");
    }

    #[test]
    fn logs_tab_shows_a_completed_test_runs_duration() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.set_tab(crate::app::Tab::Logs);
        app.apply_log_snapshot(vec![
            log_entry("/tmp/project-a", agentmon_proto::LogCategory::TestRun, "started", 0),
            log_entry("/tmp/project-a", agentmon_proto::LogCategory::TestRun, "passed", 134_000),
        ]);

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(
            text.contains("2m14s"),
            "expected the completed test run's total duration, got:\n{text}"
        );
    }

    #[test]
    fn logs_tab_shows_a_completed_agent_tasks_duration() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.set_tab(crate::app::Tab::Logs);
        app.apply_log_snapshot(vec![
            log_entry("/tmp/project-a", agentmon_proto::LogCategory::Agent, "started", 0),
            log_entry("/tmp/project-a", agentmon_proto::LogCategory::Agent, "done", 134_000),
        ]);

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(
            text.contains("2m14s"),
            "expected the completed agent task's total duration, got:\n{text}"
        );
    }

    #[test]
    fn logs_tab_renders_an_agent_started_entry() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.set_tab(crate::app::Tab::Logs);
        app.apply_log_snapshot(vec![log_entry(
            "/tmp/project-a",
            agentmon_proto::LogCategory::Agent,
            "started",
            0,
        )]);

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains('⏳'), "got:\n{text}");
        assert!(text.contains("agent started"), "got:\n{text}");
    }

    #[test]
    fn the_agents_column_never_renders_a_started_label() {
        // "started" is a one-time log event, not an ongoing AgentStatus, so
        // it must never appear as a live status in the Agents tab/pane -
        // only the six real AgentStatus labels can.
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(
            vec![
                agent("a", AgentStatus::Running, HostContext::Terminal, 1, 0),
                agent_in("/tmp/b", "b", AgentStatus::Idle, HostContext::Terminal, 2, 0),
                agent_in("/tmp/c", "c", AgentStatus::NeedsInput, HostContext::Terminal, 3, 0),
                agent_in("/tmp/d", "d", AgentStatus::Done, HostContext::Terminal, 4, 0),
                agent_in("/tmp/e", "e", AgentStatus::Stale, HostContext::Terminal, 5, 0),
                agent_in("/tmp/f", "f", AgentStatus::Declined, HostContext::Terminal, 6, 0),
            ],
            Vec::new(),
        );
        // Also log a "started" entry, so it exists in the log but must not
        // leak into the Agents tab's column.
        app.apply_log_snapshot(vec![log_entry(
            "/tmp/project",
            agentmon_proto::LogCategory::Agent,
            "started",
            0,
        )]);

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(
            !text.contains("started"),
            "the Agents tab must never render a \"started\" label, got:\n{text}"
        );
    }

    #[test]
    fn details_modal_logs_pane_renders_an_agent_started_entry() {
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.apply_log_snapshot(vec![log_entry(
            "/Users/beet/project",
            agentmon_proto::LogCategory::Agent,
            "started",
            0,
        )]);
        app.modal = Some(crate::app::Modal::Details(PathBuf::from("/Users/beet/project")));

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains('⏳'), "got:\n{text}");
        assert!(text.contains("agent started"), "got:\n{text}");
    }

    #[test]
    fn logs_tab_shows_no_duration_for_a_started_run_with_no_completion_yet() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.set_tab(crate::app::Tab::Logs);
        app.apply_log_snapshot(vec![log_entry(
            "/tmp/project-a",
            agentmon_proto::LogCategory::TestRun,
            "started",
            0,
        )]);

        term.draw(|frame| render(frame, &app)).unwrap();

        assert_eq!(
            log_completion_duration_ms(&app.logs, &app.logs[0]),
            None,
            "a lone started entry has no completion to compute a duration from"
        );
    }

    #[test]
    fn logs_tab_status_column_is_colored_by_status() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.set_tab(crate::app::Tab::Logs);
        app.apply_log_snapshot(vec![
            log_entry("/tmp/project-a", agentmon_proto::LogCategory::Agent, "done", 1_000),
            log_entry("/tmp/project-b", agentmon_proto::LogCategory::TestRun, "failed", 2_000),
            // Most recent of the three, so it sorts first under the default
            // recency sort and absorbs the default selection instead of
            // "done" or "failed" - leaving their rows' colors un-overridden
            // by the selected-row foreground.
            log_entry("/tmp/project-c", agentmon_proto::LogCategory::Agent, "idle", 3_000),
        ]);

        term.draw(|frame| render(frame, &app)).unwrap();

        let buffer = term.backend().buffer();
        let (done_x, done_y) = find_text(buffer, "done").expect("done status should be rendered");
        let (failed_x, failed_y) = find_text(buffer, "tests failed").expect("failed status should be rendered");

        let (_, done_style) = status_label_and_style(AgentStatus::Done);
        let (_, failed_style) = test_run_status_cell_text_and_style(TestRunStatus::Failed, true);
        assert_eq!(buffer[(done_x, done_y)].fg, done_style.fg.unwrap_or_default());
        assert_eq!(buffer[(failed_x, failed_y)].fg, failed_style.fg.unwrap_or_default());
        assert_ne!(
            buffer[(done_x, done_y)].fg,
            buffer[(failed_x, failed_y)].fg,
            "done and failed should be colored differently"
        );
    }

    #[test]
    fn logs_tab_shows_a_gray_placeholder_in_the_table_body_when_empty() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.set_tab(crate::app::Tab::Logs);

        term.draw(|frame| render(frame, &app)).unwrap();

        let buffer = term.backend().buffer();
        let text = buffer_text(&term);
        assert!(
            !text.contains("no activity logged yet"),
            "the empty-log message must not appear in the title, got:\n{text}"
        );
        assert!(
            text.contains("Sort [o]"),
            "sort control hint must be shown even with no activity, got:\n{text}"
        );
        assert!(
            text.contains("Filter: none"),
            "filter control hint must be shown even with no activity, got:\n{text}"
        );
        let (msg_x, msg_y) = find_text(buffer, "No activity logged yet").expect("got:\n{text}");
        assert_eq!(
            buffer[(msg_x, msg_y)].fg,
            Color::DarkGray,
            "the empty-log message should be shown in gray in the table body"
        );
    }

    #[test]
    fn logs_tab_shows_sort_and_filter_shortcut_hints_with_no_filter_applied() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.set_tab(crate::app::Tab::Logs);
        app.apply_log_snapshot(vec![log_entry(
            "/tmp/project-a",
            agentmon_proto::LogCategory::Agent,
            "done",
            1_000,
        )]);

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("Sort [o]: recency"), "got:\n{text}");
        assert!(text.contains("Filter: none."), "got:\n{text}");
        assert!(text.contains("Project [p]"), "got:\n{text}");
        assert!(text.contains("Status [s]"), "got:\n{text}");
        assert!(text.contains("Clear [c]"), "got:\n{text}");
    }

    #[test]
    fn logs_tab_retains_shortcut_hints_with_a_filter_applied() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.set_tab(crate::app::Tab::Logs);
        app.apply_log_snapshot(vec![log_entry(
            "/tmp/project-a",
            agentmon_proto::LogCategory::Agent,
            "done",
            1_000,
        )]);
        app.cycle_logs_project_filter();
        app.cycle_logs_sort();

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("Sort [o]: project"), "got:\n{text}");
        assert!(text.contains("Filter: project=project-a."), "got:\n{text}");
        assert!(
            text.contains("Project [p]") && text.contains("Status [s]") && text.contains("Clear [c]"),
            "shortcut hints must remain visible once a filter is applied, got:\n{text}"
        );
    }

    #[test]
    fn a_filter_matching_nothing_shows_a_gray_placeholder_in_the_table_body_not_the_title() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.set_tab(crate::app::Tab::Logs);
        app.apply_log_snapshot(vec![
            log_entry("/tmp/alpha", agentmon_proto::LogCategory::Agent, "done", 1_000),
            log_entry("/tmp/beta", agentmon_proto::LogCategory::Agent, "needs_input", 2_000),
        ]);
        // "alpha" only ever has status "done" - combining it with the
        // "needs_input" status filter matches nothing.
        app.cycle_logs_project_filter(); // -> "alpha" (first alphabetically)
        app.cycle_logs_status_filter(); // -> "done"
        app.cycle_logs_status_filter(); // -> "needs_input"

        term.draw(|frame| render(frame, &app)).unwrap();

        let buffer = term.backend().buffer();
        let text = buffer_text(&term);
        assert!(
            !text.contains("no activity matches"),
            "the empty-filter message must not appear in the title anymore, got:\n{text}"
        );
        let (msg_x, msg_y) =
            find_text(buffer, "No activity matches the current filter").expect("got:\n{text}");
        assert_eq!(
            buffer[(msg_x, msg_y)].fg,
            Color::DarkGray,
            "the empty-filter message should be shown in gray"
        );
    }

    #[test]
    fn details_modal_shows_agents_tests_and_logs_panes() {
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(
            vec![agent("a", AgentStatus::Running, HostContext::Terminal, 4242, 0)],
            vec![test_run(999, TestRunStatus::Failed, 0)],
        );
        app.apply_log_snapshot(vec![log_entry(
            "/Users/beet/project",
            agentmon_proto::LogCategory::TestRun,
            "failed",
            0,
        )]);
        app.open_details_modal();

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("Agents"), "got:\n{text}");
        assert!(text.contains("Tests"), "got:\n{text}");
        assert!(text.contains("Logs"), "got:\n{text}");
        assert!(text.contains("running"), "expected the agent's status, got:\n{text}");
        assert!(text.contains("tests failed"), "expected the test run's status, got:\n{text}");
    }

    #[test]
    fn details_modal_reflects_new_events_while_open() {
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(
            vec![agent("a", AgentStatus::Running, HostContext::Terminal, 4242, 0)],
            Vec::new(),
        );
        app.open_details_modal();
        term.draw(|frame| render(frame, &app)).unwrap();
        assert!(buffer_text(&term).contains("running"), "sanity check before the update");

        // Simulate an event arriving from the daemon while the modal stays
        // open - the same `apply_update`/`apply_log_appended` calls
        // `main.rs`'s event loop makes unconditionally, regardless of modal
        // state.
        app.apply_update(agent("a", AgentStatus::NeedsInput, HostContext::Terminal, 4242, 1_000));
        app.apply_log_appended(log_entry(
            "/Users/beet/project",
            agentmon_proto::LogCategory::Agent,
            "needs_input",
            1_000,
        ));

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(
            text.contains("NEEDS INPUT"),
            "the modal's Agents pane should reflect the new status without closing/reopening, got:\n{text}"
        );
        assert_eq!(
            text.matches("NEEDS INPUT").count(),
            2,
            "expected NEEDS INPUT once in the Agents pane and once in the Logs pane for the new entry, got:\n{text}"
        );
    }

    #[test]
    fn details_modal_puts_agents_and_tests_side_by_side_above_logs() {
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(
            vec![agent("a", AgentStatus::Running, HostContext::Terminal, 4242, 0)],
            vec![test_run(999, TestRunStatus::Failed, 0)],
        );
        app.open_details_modal();

        term.draw(|frame| render(frame, &app)).unwrap();

        let buffer = term.backend().buffer();
        // "Agents" also appears as the tab bar's label and the background
        // Agents-tab table's own title - restrict the search to rows below
        // the modal's own "Details - ..." title to find its Agents pane.
        let (_, details_y) = find_text(buffer, "Details -").expect("modal title should be rendered");
        let (agents_x, agents_y) = find_text_from(buffer, "Agents", details_y + 1)
            .expect("Agents pane title should be rendered");
        let (tests_x, tests_y) = find_text_from(buffer, "Tests", details_y + 1)
            .expect("Tests pane title should be rendered");
        let (_, logs_y) = find_text_from(buffer, "Logs", details_y + 1)
            .expect("Logs pane title should be rendered");

        assert_eq!(agents_y, tests_y, "Agents and Tests panes must be on the same row");
        assert!(tests_x > agents_x, "Tests pane must be to the right of Agents");
        assert!(logs_y > agents_y, "Logs pane must be below Agents/Tests");
    }

    #[test]
    fn details_modal_colors_agent_and_test_statuses_like_the_agents_tab() {
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(
            vec![agent("a", AgentStatus::NeedsInput, HostContext::Terminal, 4242, 0)],
            vec![test_run(999, TestRunStatus::Failed, 0)],
        );
        app.open_details_modal();

        term.draw(|frame| render(frame, &app)).unwrap();

        let buffer = term.backend().buffer();
        let (x, y) = find_text(buffer, "NEEDS INPUT").expect("agent status should be rendered");
        let (_, expected_style) = status_label_and_style(AgentStatus::NeedsInput);
        assert_eq!(buffer[(x, y)].fg, expected_style.fg.unwrap_or_default());

        // The modal's Tests pane omits the "tests" category-word prefix
        // (unlike the top-level Agents tab it mirrors styling from).
        let (x, y) = find_text(buffer, "failed").expect("test run status should be rendered");
        let (_, expected_style) = test_run_status_cell_text_and_style(TestRunStatus::Failed, false);
        assert_eq!(buffer[(x, y)].fg, expected_style.fg.unwrap_or_default());
    }

    #[test]
    fn details_modal_logs_pane_shows_a_completed_test_runs_duration() {
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.apply_log_snapshot(vec![
            log_entry("/Users/beet/project", agentmon_proto::LogCategory::TestRun, "started", 0),
            log_entry("/Users/beet/project", agentmon_proto::LogCategory::TestRun, "failed", 45_000),
        ]);
        app.agents_selected = 0;
        app.modal = Some(crate::app::Modal::Details(PathBuf::from("/Users/beet/project")));

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(
            text.contains("45s"),
            "expected the completed test run's duration in the Logs pane, got:\n{text}"
        );
    }

    #[test]
    fn details_modal_shows_placeholders_when_no_test_run_or_activity() {
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(vec![agent("a", AgentStatus::Running, HostContext::Terminal, 4242, 0)], Vec::new());
        app.open_details_modal();

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("No test run"), "got:\n{text}");
        assert!(text.contains("No activity"), "got:\n{text}");
    }

    #[test]
    fn details_modal_agents_pane_shows_each_agents_pid() {
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(vec![agent("a", AgentStatus::Running, HostContext::Terminal, 777_777, 0)], Vec::new());
        app.open_details_modal();

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(
            text.contains("pid 777777"),
            "expected the agent's pid in the Agents pane, got:\n{text}"
        );
    }

    #[test]
    fn details_modal_logs_pane_shows_pid_for_agent_activities_but_not_test_runs() {
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.apply_log_snapshot(vec![
            LogEntry {
                working_dir: PathBuf::from("/Users/beet/project"),
                category: agentmon_proto::LogCategory::Agent,
                status: "done".to_string(),
                occurred_at_ms: 1_000,
                pid: Some(555_555),
            },
            LogEntry {
                working_dir: PathBuf::from("/Users/beet/project"),
                category: agentmon_proto::LogCategory::TestRun,
                status: "failed".to_string(),
                occurred_at_ms: 2_000,
                pid: Some(666_666),
            },
        ]);
        app.modal = Some(crate::app::Modal::Details(PathBuf::from("/Users/beet/project")));

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(
            text.contains("pid 555555"),
            "expected the agent entry's pid in the Logs pane, got:\n{text}"
        );
        assert!(
            !text.contains("pid 666666"),
            "a test-run entry must not show a pid in the Logs pane, got:\n{text}"
        );
    }

    #[test]
    fn closing_the_details_modal_returns_to_the_agents_tab() {
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(vec![agent("a", AgentStatus::Running, HostContext::Terminal, 4242, 0)], Vec::new());
        app.open_details_modal();
        app.close_modal();

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(!text.contains("Details -"), "modal must not still be shown, got:\n{text}");
        assert!(text.contains("Agents"), "got:\n{text}");
    }

    #[test]
    fn tab_bar_shows_shortcut_hints_and_the_title() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("Agents [a]"), "got:\n{text}");
        assert!(text.contains("Logs [l]"), "got:\n{text}");
        assert!(text.contains("(tab)"), "got:\n{text}");
        assert!(text.contains("agentmon"), "got:\n{text}");
    }

    #[test]
    fn tab_bar_title_is_on_the_same_line_as_the_tabs_far_right() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());

        term.draw(|frame| render(frame, &app)).unwrap();

        let buffer = term.backend().buffer();
        let (tabs_x, tabs_y) = find_text(buffer, "Agents").expect("tab labels should be rendered");
        let (title_x, title_y) = find_text(buffer, "agentmon").expect("agentmon title should be rendered");

        assert_eq!(tabs_y, title_y, "the title must be on the same row as the tab labels");
        assert!(
            title_x > tabs_x,
            "the title must be to the right of the tab labels, got title_x={title_x} tabs_x={tabs_x}"
        );
        assert!(
            title_x as usize + "agentmon".len() == buffer.area.width as usize,
            "the title must be flush against the far right edge, got title_x={title_x} width={}",
            buffer.area.width
        );

        let title_cell = &buffer[(title_x, title_y)];
        assert!(
            title_cell.modifier.contains(Modifier::BOLD),
            "the agentmon title should be bold"
        );
    }

    #[test]
    fn tab_bar_has_no_top_left_or_right_border() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());

        term.draw(|frame| render(frame, &app)).unwrap();

        let buffer = term.backend().buffer();
        // Row 0 is the tab bar's content row - it must not carry a border
        // character (┌, ┐, or │) on its edges.
        assert_ne!(buffer[(0, 0)].symbol(), "┌", "no top-left corner expected");
        assert_ne!(buffer[(buffer.area.width - 1, 0)].symbol(), "┐", "no top-right corner expected");
        assert_ne!(buffer[(0, 0)].symbol(), "│", "no left border expected");
        assert_ne!(buffer[(buffer.area.width - 1, 0)].symbol(), "│", "no right border expected");
    }

    #[test]
    fn active_tab_is_visually_highlighted() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.set_tab(Tab::Logs);

        term.draw(|frame| render(frame, &app)).unwrap();

        let logs_tab_cell = find_cell(term.backend().buffer(), "L").expect("Logs tab label should be rendered");
        assert!(
            logs_tab_cell.modifier.contains(Modifier::BOLD),
            "the active tab should be visually highlighted"
        );
    }

    #[test]
    fn selected_row_uses_a_background_fill_rather_than_reversed_video() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(
            vec![agent("a", AgentStatus::NeedsInput, HostContext::Terminal, 4242, 0)],
            Vec::new(),
        );

        term.draw(|frame| render(frame, &app)).unwrap();

        let buffer = term.backend().buffer();
        let (x, y) = find_text(buffer, "NEEDS INPUT").expect("selected row's status should be rendered");
        let cell = &buffer[(x, y)];

        assert_eq!(
            cell.bg,
            SELECTED_ROW_BG,
            "the selected row should be marked with a background fill"
        );
        assert!(
            !cell.modifier.contains(Modifier::REVERSED),
            "the selected row should not use reversed video, which inverts status colors"
        );
        assert_eq!(
            cell.fg,
            SELECTED_ROW_FG,
            "the selected row's text color must be forced to the selected-row \
             foreground, overriding the status's own color, so it stays legible \
             against the selected-row background"
        );
    }

    #[test]
    fn help_modal_lists_keyboard_shortcuts() {
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.open_help_modal();

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("Keyboard Shortcuts"), "got:\n{text}");
        assert!(text.contains("quit"), "got:\n{text}");
    }
}
