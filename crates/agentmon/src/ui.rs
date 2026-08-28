use ratatui::layout::Constraint;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
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
    let header = Row::new(["PROJECT", "HOST", "PID", "STATUS"]).style(Style::new().bold());

    let rows = app.agents.iter().map(|agent| {
        let (label, style) = status_label_and_style(agent.status);
        Row::new([
            Cell::from(project_name(agent)),
            Cell::from(host_label(agent.host_context)),
            Cell::from(agent.pid.to_string()),
            Cell::from(label).style(style),
        ])
    });

    let widths = [
        Constraint::Percentage(40),
        Constraint::Length(10),
        Constraint::Length(8),
        Constraint::Percentage(30),
    ];

    let title = match banner {
        Some(banner) => format!("agentmon - {banner}"),
        None if app.agents.is_empty() => "agentmon - no agents tracked yet".to_string(),
        None => "agentmon".to_string(),
    };
    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().title(title).borders(Borders::ALL))
        .row_highlight_style(Style::new().add_modifier(Modifier::REVERSED));

    let mut state = TableState::new().with_selected(if app.agents.is_empty() {
        None
    } else {
        Some(app.selected)
    });
    frame.render_stateful_widget(table, frame.area(), &mut state);
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
        Terminal::new(TestBackend::new(60, 10)).unwrap()
    }

    fn agent(status: AgentStatus, host: HostContext) -> AgentInfo {
        AgentInfo {
            session_id: SessionId("s".to_string()),
            cwd: PathBuf::from("/Users/beet/project"),
            host_context: host,
            pid: 4242,
            status,
            last_updated_ms: 0,
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
        app.apply_snapshot(vec![agent(AgentStatus::Running, HostContext::Nvim)]);

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
            agent(AgentStatus::Running, HostContext::Terminal),
            agent(AgentStatus::NeedsInput, HostContext::Terminal),
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
}
