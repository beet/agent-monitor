use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::layout::Constraint;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::Frame;

use std::path::Path;

use agentmon_proto::{AgentInfo, AgentStatus, TestRunInfo, TestRunStatus};

use crate::app::{App, ConnectionStatus, DirectoryGroup};

/// The order distinct agent statuses appear in a project's combined STATUS
/// cell - a fixed order so the same set of statuses always renders the same
/// way, independent of the order agents happen to be tracked in.
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
        ConnectionStatus::Connected => render_agent_table(frame, app, None),
        ConnectionStatus::Reconnecting => {
            render_agent_table(frame, app, Some("reconnecting to agentd..."))
        }
    }
}

fn render_message(frame: &mut Frame, message: &str) {
    let block = Block::default().title("agentmon").borders(Borders::ALL);
    let paragraph = Paragraph::new(message).block(block);
    frame.render_widget(paragraph, frame.area());
}

fn render_agent_table(frame: &mut Frame, app: &App, banner: Option<&str>) {
    let header = Row::new(["PROJECT", "STATUS", "UPDATED"]).style(Style::new().bold());

    let now = now_ms();
    let groups = app.directory_groups();
    let rows = groups.iter().map(|group| {
        let project = project_name(&group.cwd);
        Row::new([
            Cell::from(project),
            Cell::from(project_status_line(group, now)),
            Cell::from(format_last_updated(group.most_recent_update_ms())),
        ])
    });

    let widths = [
        Constraint::Fill(2),
        Constraint::Fill(3),
        Constraint::Length(19),
    ];

    let title = match banner {
        Some(banner) => format!("agentmon - {banner}"),
        None if app.agents.is_empty() && app.test_runs.is_empty() => {
            "agentmon - no agents tracked yet".to_string()
        }
        None => "agentmon".to_string(),
    };
    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().title(title).borders(Borders::ALL));

    frame.render_widget(table, frame.area());
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

/// Builds a project's combined STATUS cell: one styled segment per distinct
/// agent status present (in `AGENT_STATUS_ORDER`, so the same set of
/// statuses always renders in the same order), followed by the test run's
/// segment if the project has one - joined by " · " so no status is hidden
/// behind another, per the "Status is visually distinguishable" requirement.
fn project_status_line(group: &DirectoryGroup, now_ms: u64) -> Line<'static> {
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
        let (text, style) = if status == AgentStatus::Running && matching.len() == 1 {
            status_cell_text_and_style(first, now_ms)
        } else {
            let (label, style) = status_label_and_style(status);
            (label.to_string(), style)
        };
        spans.push(Span::styled(text, style));
    }

    if let Some(test_run) = group.test_runs.first() {
        if !spans.is_empty() {
            spans.push(Span::raw(" · "));
        }
        let (label, style) = test_run_status_cell_text_and_style(test_run.status);
        let duration = format_test_run_duration(test_run, now_ms);
        spans.push(Span::styled(format!("{label} {duration}"), style));
    }

    Line::from(spans)
}

/// A test run's duration: live and counting up while "started" (computed
/// against the current time, like a running agent's), and the fixed total
/// elapsed time once "passed" or "failed" (computed against its own
/// last-updated time instead, so it stops advancing once the run is done).
fn format_test_run_duration(test_run: &TestRunInfo, now_ms: u64) -> String {
    match test_run.status {
        TestRunStatus::Started => format_running_duration(test_run.run_started_ms, now_ms),
        TestRunStatus::Passed | TestRunStatus::Failed => {
            format_running_duration(test_run.run_started_ms, test_run.last_updated_ms)
        }
    }
}

/// The STATUS cell's text and style for one agent: the running status gets
/// an appended elapsed-duration counter (see `format_running_duration`);
/// every other status renders as just its label.
fn status_cell_text_and_style(agent: &AgentInfo, now_ms: u64) -> (String, Style) {
    let (label, style) = status_label_and_style(agent.status);
    let text = match agent.status {
        AgentStatus::Running => {
            format!("{label} {}", format_running_duration(agent.status_since_ms, now_ms))
        }
        _ => label.to_string(),
    };
    (text, style)
}

/// Every status gets both a distinct label and a distinct style, so the
/// distinction survives even in a plain-text rendering (as asserted by
/// tests) and not only through color.
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
            "🕸️ stale",
            Style::new()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM),
        ),
        AgentStatus::Declined => ("🚫 declined", Style::new().fg(Color::Red)),
    }
}

/// The STATUS cell's text and style for a test run. Labels are worded
/// distinctly from agent statuses (e.g. "tests passed" vs "done") so a
/// passed test run and a done agent aren't visually confused even though
/// both use a ✅ marker.
fn test_run_status_cell_text_and_style(status: TestRunStatus) -> (String, Style) {
    let (label, style) = match status {
        TestRunStatus::Started => ("⏳ test started", Style::new().fg(Color::Blue)),
        TestRunStatus::Passed => ("✅ tests passed", Style::new().fg(Color::Green)),
        TestRunStatus::Failed => (
            "❌ tests failed",
            Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
    };
    (label.to_string(), style)
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
    fn the_table_has_no_host_or_pid_columns() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(vec![agent("s", AgentStatus::Running, HostContext::Nvim, 4242, 0)], Vec::new());

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("PROJECT"), "got:\n{text}");
        assert!(text.contains("STATUS"), "got:\n{text}");
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
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(
            vec![
                agent("a", AgentStatus::Running, HostContext::Terminal, 4242, 0),
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
        // foreground color differs from the "running" span's, within that
        // same row.
        let buffer = term.backend().buffer();
        let row = 2; // border + header, then the one combined project row
        let needs_input_cell = (0..buffer.area.width)
            .find(|&x| buffer[(x, row)].symbol() == "N")
            .map(|x| &buffer[(x, row)]);
        let running_cell = (0..buffer.area.width)
            .find(|&x| buffer[(x, row)].symbol() == "r")
            .map(|x| &buffer[(x, row)]);

        let needs_input_cell = needs_input_cell.expect("NEEDS INPUT cell should be found");
        let running_cell = running_cell.expect("running cell should be found");
        assert_ne!(
            needs_input_cell.fg, running_cell.fg,
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

        let (text, _) = status_cell_text_and_style(&running, 1_000_000);

        assert_eq!(text, "🔧 running 2m14s");
    }

    #[test]
    fn a_non_running_agent_shows_no_duration() {
        let idle = agent("s", AgentStatus::Idle, HostContext::Terminal, 4242, 0);

        let (text, _) = status_cell_text_and_style(&idle, 1_000_000);

        assert_eq!(text, "💤 idle");
    }

    #[test]
    fn a_declined_agent_shows_no_duration() {
        let declined = agent("s", AgentStatus::Declined, HostContext::Terminal, 4242, 0);

        let (text, _) = status_cell_text_and_style(&declined, 1_000_000);

        assert_eq!(text, "🚫 declined");
    }

    #[test]
    fn declined_is_visually_distinguished_from_other_statuses() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(
            vec![
                agent("a", AgentStatus::Running, HostContext::Terminal, 4242, 0),
                agent("b", AgentStatus::Declined, HostContext::Terminal, 4243, 0),
            ],
            Vec::new(),
        );

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("declined"), "expected declined status, got:\n{text}");
        assert!(text.contains("running"), "expected running status, got:\n{text}");

        let buffer = term.backend().buffer();
        let row = 2; // border + header, then the one combined project row
        let declined_cell = (0..buffer.area.width)
            .find(|&x| buffer[(x, row)].symbol() == "d")
            .map(|x| &buffer[(x, row)]);
        let running_cell = (0..buffer.area.width)
            .find(|&x| buffer[(x, row)].symbol() == "r")
            .map(|x| &buffer[(x, row)]);

        let declined_cell = declined_cell.expect("declined cell should be found");
        let running_cell = running_cell.expect("running cell should be found");
        assert_ne!(
            declined_cell.fg, running_cell.fg,
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
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(
            vec![
                agent("a", AgentStatus::Running, HostContext::Terminal, 4242, 0),
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
        app.apply_test_run_update(test_run(999, TestRunStatus::Started, 0));

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("project"), "expected project name, got:\n{text}");
        assert!(text.contains("test started"), "expected test-run status, got:\n{text}");
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
            test_run_status_cell_text_and_style(TestRunStatus::Started).0,
            "⏳ test started"
        );
        assert_eq!(
            test_run_status_cell_text_and_style(TestRunStatus::Passed).0,
            "✅ tests passed"
        );
        assert_eq!(
            test_run_status_cell_text_and_style(TestRunStatus::Failed).0,
            "❌ tests failed"
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

        let (_, failed_style) = test_run_status_cell_text_and_style(TestRunStatus::Failed);
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

        let text = buffer_text(&term);
        assert!(text.contains("running"), "got:\n{text}");
        assert!(text.contains("tests failed"), "got:\n{text}");
        assert!(
            text.contains(" · "),
            "expected the agent and test-run statuses joined by a separator, got:\n{text}"
        );
    }

    #[test]
    fn format_test_run_duration_for_a_started_run_is_live() {
        let started = test_run_with_start(1, TestRunStatus::Started, 0, 0);

        assert_eq!(format_test_run_duration(&started, 9_000), "9s");
    }

    #[test]
    fn format_test_run_duration_for_a_finished_run_is_fixed_regardless_of_now() {
        let passed = test_run_with_start(1, TestRunStatus::Passed, 0, 134_000);

        assert_eq!(format_test_run_duration(&passed, 999_999_999), "2m14s");
    }

    #[test]
    fn a_started_test_run_shows_a_live_elapsed_duration() {
        let mut term = terminal();
        let mut app = App::new();
        let now = now_ms();
        app.apply_test_run_update(test_run_with_start(999, TestRunStatus::Started, now - 134_000, now));

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
}
