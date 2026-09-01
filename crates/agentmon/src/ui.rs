use ratatui::layout::Constraint;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::Frame;

use agentmon_proto::{AgentInfo, AgentStatus, HostContext};

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

    let agents = sorted_by_recency(&app.agents);
    let rows = agents.iter().map(|agent| {
        let (label, style) = status_label_and_style(agent.status);
        Row::new([
            Cell::from(project_name(agent)),
            Cell::from(host_label(agent.host_context)),
            Cell::from(agent.pid.to_string()),
            Cell::from(label).style(style),
            Cell::from(format_last_updated(agent.last_updated_ms)),
        ])
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
        None if app.agents.is_empty() => "agentmon - no agents tracked yet".to_string(),
        None => "agentmon".to_string(),
    };
    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().title(title).borders(Borders::ALL));

    frame.render_widget(table, frame.area());
}

/// Returns agents ordered most-recently-updated first, without touching
/// `App.agents`'s own (insertion) order.
fn sorted_by_recency<'a>(agents: &'a [AgentInfo]) -> Vec<&'a AgentInfo> {
    let mut sorted: Vec<&AgentInfo> = agents.iter().collect();
    sorted.sort_by(|a, b| b.last_updated_ms.cmp(&a.last_updated_ms));
    sorted
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

fn project_name(agent: &AgentInfo) -> String {
    agent
        .cwd
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| agent.cwd.display().to_string())
}

fn host_label(host: HostContext) -> &'static str {
    match host {
        HostContext::Nvim => "nvim",
        HostContext::Terminal => "terminal",
        HostContext::Desktop => "desktop",
    }
}

/// Every status gets both a distinct label and a distinct style, so the
/// distinction survives even in a plain-text rendering (as asserted by
/// tests) and not only through color.
fn status_label_and_style(status: AgentStatus) -> (&'static str, Style) {
    match status {
        AgentStatus::Running => ("running", Style::new().fg(Color::Blue)),
        AgentStatus::Idle => ("idle", Style::new().fg(Color::Gray)),
        AgentStatus::NeedsInput => (
            "NEEDS INPUT",
            Style::new()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        AgentStatus::Done => ("done", Style::new().fg(Color::Green)),
        AgentStatus::Stale => (
            "stale",
            Style::new()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM),
        ),
    }
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
        app.apply_snapshot(vec![agent("s", AgentStatus::Running, HostContext::Nvim, 4242, 0)]);

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
        app.apply_snapshot(vec![
            agent("a", AgentStatus::Running, HostContext::Terminal, 4242, 0),
            agent("b", AgentStatus::NeedsInput, HostContext::Terminal, 4243, 0),
        ]);

        term.draw(|frame| render(frame, &app)).unwrap();

        // Distinguished by label text...
        let text = buffer_text(&term);
        assert!(text.contains("NEEDS INPUT"));
        assert!(text.contains("running"));

        // ...and by style: locate the "NEEDS INPUT" cell and confirm its
        // background differs from a "running" cell's.
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
            needs_input_cell.bg, running_cell.bg,
            "needs-input styling must differ from running styling"
        );
    }

    #[test]
    fn agents_are_rendered_most_recently_updated_first() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(vec![
            agent("older", AgentStatus::Running, HostContext::Terminal, 1111, 1_000),
            agent("newer", AgentStatus::Running, HostContext::Terminal, 2222, 2_000),
        ]);

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
        app.apply_snapshot(vec![
            agent("a", AgentStatus::Running, HostContext::Terminal, 1111, 1_000),
            agent("b", AgentStatus::Running, HostContext::Terminal, 2222, 2_000),
        ]);
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
        app.apply_snapshot(vec![agent(
            "s",
            AgentStatus::Running,
            HostContext::Terminal,
            4242,
            last_updated_ms,
        )]);

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
}
