use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::layout::Constraint;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::Frame;

use std::path::Path;

use agentmon_proto::{AgentInfo, AgentStatus, HostContext, TestRunStatus};

use crate::app::{App, ConnectionStatus};

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
    let header = Row::new(["PROJECT", "HOST", "PID", "STATUS", "UPDATED"]).style(Style::new().bold());

    let now = now_ms();
    let groups = app.directory_groups();
    let rows = groups.iter().flat_map(|group| {
        let project = project_name(&group.cwd);
        let test_run_project = project.clone();
        let agent_rows = group.agents.iter().map(move |agent| {
            let (status_cell, style) = status_cell_text_and_style(agent, now);
            Row::new([
                Cell::from(project.clone()),
                Cell::from(host_label(agent.host_context)),
                Cell::from(agent.pid.to_string()),
                Cell::from(status_cell).style(style),
                Cell::from(format_last_updated(agent.last_updated_ms)),
            ])
        });
        let test_run_rows = group.test_runs.iter().map(move |test_run| {
            let (status_cell, style) = test_run_status_cell_text_and_style(test_run.status);
            Row::new([
                Cell::from(test_run_project.clone()),
                Cell::from("test run"),
                Cell::from(test_run.pid.to_string()),
                Cell::from(status_cell).style(style),
                Cell::from(format_last_updated(test_run.last_updated_ms)),
            ])
        });
        agent_rows.chain(test_run_rows).collect::<Vec<_>>()
    });

    let widths = [
        Constraint::Percentage(32),
        Constraint::Length(10),
        Constraint::Length(8),
        Constraint::Percentage(22),
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

fn host_label(host: HostContext) -> &'static str {
    match host {
        HostContext::Nvim => "nvim",
        HostContext::Terminal => "terminal",
        HostContext::Desktop => "desktop",
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
    use agentmon_proto::SessionId;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::path::PathBuf;

    fn terminal() -> Terminal<TestBackend> {
        Terminal::new(TestBackend::new(100, 10)).unwrap()
    }

    fn agent(id: &str, status: AgentStatus, host: HostContext, pid: u32, last_updated_ms: u64) -> AgentInfo {
        AgentInfo {
            session_id: SessionId(id.to_string()),
            cwd: PathBuf::from("/Users/beet/project"),
            host_context: host,
            pid,
            status,
            last_updated_ms,
            status_since_ms: last_updated_ms,
        }
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
        assert!(text.contains("nvim"), "expected host label, got:\n{text}");
        assert!(text.contains("4242"), "expected pid, got:\n{text}");
        assert!(text.contains("running"), "expected status, got:\n{text}");
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

        // Distinguished by label text...
        let text = buffer_text(&term);
        assert!(text.contains("NEEDS INPUT"));
        assert!(text.contains("running"));

        // ...and by style: locate the "NEEDS INPUT" cell and confirm its
        // foreground color differs from a "running" cell's.
        let buffer = term.backend().buffer();
        let needs_input_cell = (0..buffer.area.width)
            .find(|&x| buffer[(x, 3)].symbol() == "N")
            .map(|x| &buffer[(x, 3)]);
        let running_cell = (0..buffer.area.width)
            .find(|&x| buffer[(x, 2)].symbol() == "r")
            .map(|x| &buffer[(x, 2)]);

        let needs_input_cell = needs_input_cell.expect("NEEDS INPUT cell should be found");
        let running_cell = running_cell.expect("running cell should be found");
        assert_ne!(
            needs_input_cell.fg, running_cell.fg,
            "needs-input styling must differ from running styling"
        );
    }

    #[test]
    fn agents_are_rendered_most_recently_updated_first() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(
            vec![
                agent("older", AgentStatus::Running, HostContext::Terminal, 1111, 1_000),
                agent("newer", AgentStatus::Running, HostContext::Terminal, 2222, 2_000),
            ],
            Vec::new(),
        );

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        let newer_pos = text.find("2222").expect("newer agent's pid should be rendered");
        let older_pos = text.find("1111").expect("older agent's pid should be rendered");
        assert!(
            newer_pos < older_pos,
            "more recently updated agent should render first:\n{text}"
        );
    }

    #[test]
    fn an_update_moves_the_updated_agent_to_the_top() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(
            vec![
                agent("a", AgentStatus::Running, HostContext::Terminal, 1111, 1_000),
                agent("b", AgentStatus::Running, HostContext::Terminal, 2222, 2_000),
            ],
            Vec::new(),
        );
        // "a" starts below "b"; a fresh update should move it back to the top.
        app.apply_update(agent("a", AgentStatus::Running, HostContext::Terminal, 1111, 3_000));

        term.draw(|frame| render(frame, &app)).unwrap();

        let text = buffer_text(&term);
        let a_pos = text.find("1111").expect("agent a's pid should be rendered");
        let b_pos = text.find("2222").expect("agent b's pid should be rendered");
        assert!(
            a_pos < b_pos,
            "the just-updated agent should move to the top of the table:\n{text}"
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
        let declined_cell = (0..buffer.area.width)
            .find(|&x| buffer[(x, 3)].symbol() == "d")
            .map(|x| &buffer[(x, 3)]);
        let running_cell = (0..buffer.area.width)
            .find(|&x| buffer[(x, 2)].symbol() == "r")
            .map(|x| &buffer[(x, 2)]);

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

    fn test_run(pid: u32, status: TestRunStatus, last_updated_ms: u64) -> agentmon_proto::TestRunInfo {
        agentmon_proto::TestRunInfo {
            cwd: PathBuf::from("/Users/beet/project"),
            pid,
            status,
            last_updated_ms,
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
}
