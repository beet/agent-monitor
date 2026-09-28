use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table,
    TableState,
};
use ratatui::Frame;

use std::path::Path;

use agentmon_proto::{AgentInfo, AgentStatus, LogEntry, ReminderInfo, ReminderStatus, TestRunInfo, TestRunStatus};

use crate::app::{
    needs_pagination, App, ConnectionStatus, DirectoryGroup, LogSort, Modal, ModalFocus, PageSizes,
    ReminderFormField, ReminderFormMode, ReminderSort, Tab,
};

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

/// Renders one frame and reports back how many rows each paginated list
/// (the Logs tab, the details modal's Logs pane) actually fit on screen, so
/// the caller can thread that `page_size` into the next key press's paging
/// math - see `PageSizes` and design.md's "`page_size` is a parameter"
/// decision. A pane that wasn't rendered this frame (e.g. the Agents tab is
/// active, or no modal is open) reports `0`, which is never consulted since
/// the same condition that skipped rendering it also skips routing keys to
/// it.
pub fn render(frame: &mut Frame, app: &App) -> PageSizes {
    match &app.connection {
        ConnectionStatus::Connecting => {
            render_message(frame, "Connecting to agentd...");
            PageSizes::default()
        }
        ConnectionStatus::Unreachable(reason) => {
            render_message(
                frame,
                &format!("agentd is not running.\n\nStart it with: agentd\n\n({reason})"),
            );
            PageSizes::default()
        }
        ConnectionStatus::Connected => render_body(frame, app, None),
        ConnectionStatus::Reconnecting => render_body(frame, app, Some("reconnecting to agentd...")),
    }
}

fn render_body(frame: &mut Frame, app: &App, banner: Option<&str>) -> PageSizes {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(frame.area());

    render_tab_bar(frame, app, chunks[0]);

    let mut logs_tab = 0;
    let mut reminders_tab = 0;
    match app.active_tab {
        Tab::Agents => render_agent_table(frame, app, chunks[1], banner),
        Tab::Logs => logs_tab = render_logs_tab(frame, app, chunks[1], banner),
        Tab::Reminders => reminders_tab = render_reminders_tab(frame, app, chunks[1], banner),
    }

    let (modal_logs, modal_reminders) = match &app.modal {
        Some(modal) => render_modal(frame, app, modal),
        None => (0, 0),
    };

    PageSizes { logs_tab, modal_logs, reminders_tab, modal_reminders }
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
        Span::raw(" | "),
        Span::styled("Reminders [r]", tab_style(Tab::Reminders)),
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
    let header = Row::new(["PROJECT", "AGENTS", "TESTS", "REMINDERS", "UPDATED"]).style(Style::new().bold());

    let block = Block::default()
        .title(agents_tab_title(app, banner))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let inner = block.inner(area);

    let now = now_ms();
    let groups = app.visible_agent_groups();

    // Builds each row's content once, up front, so PROJECT/AGENTS/TESTS/
    // REMINDERS can each be sized to what they actually need this frame
    // instead of a fixed weighted ratio that leaves a narrow column
    // cramped while others sit mostly blank next to it.
    let project_texts: Vec<String> = groups.iter().map(|g| project_name(&g.cwd)).collect();
    let agents_lines: Vec<Line> = groups.iter().map(|g| agents_status_line(g, now, true)).collect();
    let tests_lines: Vec<Line> = groups.iter().map(|g| tests_status_line(g, now, true)).collect();
    let reminders_lines: Vec<Line> = groups.iter().map(|g| reminders_status_line(g, app, now)).collect();

    // This table always has a selected row once `groups` is non-empty, so
    // its `"> "` highlight symbol's 2-column width is reserved on the left,
    // same as `Table` itself reserves it (via `HighlightSpacing::
    // WhenSelected`, the default); `column_spacing` (default 1) reserves a
    // further column between each of the 5 columns below.
    let selection_width: u16 = if groups.is_empty() { 0 } else { 2 };
    const COLUMN_SPACING: u16 = 1;
    const UPDATED_WIDTH: u16 = 19;
    let available_for_dynamic = inner
        .width
        .saturating_sub(selection_width)
        .saturating_sub(COLUMN_SPACING * 4)
        .saturating_sub(UPDATED_WIDTH);

    // Each dynamic column's actual need this frame - the widest content
    // among the current rows, floored at its header's own width so a
    // column with blank content still shows its header in full, and capped
    // at half the total dynamic space so one row's unusually long content
    // (e.g. a long reminder name) can't dominate the row and squeeze every
    // other column down to nothing; `truncate_line` below takes over for
    // whatever a capped column can't fully show.
    let cap = (available_for_dynamic / 2).max(1);
    let project_need = content_need(project_texts.iter().map(String::as_str).map(str_width), "PROJECT").min(cap);
    let agents_need = content_need(agents_lines.iter().map(line_width), "AGENTS").min(cap);
    let tests_need = content_need(tests_lines.iter().map(line_width), "TESTS").min(cap);
    let reminders_need = content_need(reminders_lines.iter().map(line_width), "REMINDERS").min(cap);

    // Any space left over once every column has what it needs is split
    // 2:3:2 between PROJECT/AGENTS/TESTS - the same relative weights this
    // table used before Reminders was added - so Agents and Tests
    // continue to receive the majority of it, per the "Project name
    // column scales with terminal width" requirement. REMINDERS gets
    // exactly what its content needs and no bonus growth, since an
    // undersized Reminders column - not an oversized one - is the problem
    // this dynamic sizing solves.
    let leftover = available_for_dynamic.saturating_sub(project_need + agents_need + tests_need + reminders_need);
    let leftover_project = leftover * 2 / 7;
    let leftover_agents = leftover * 3 / 7;
    let leftover_tests = leftover - leftover_project - leftover_agents;

    let widths = [
        Constraint::Length(project_need + leftover_project),
        Constraint::Length(agents_need + leftover_agents),
        Constraint::Length(tests_need + leftover_tests),
        Constraint::Length(reminders_need),
        Constraint::Length(UPDATED_WIDTH),
    ];

    // Resolves the exact column widths `Table` will actually render (only
    // ever narrower than what's requested above if the terminal is too
    // narrow to fit them all, in which case `Length` constraints shrink
    // together proportionally), so the ellipsis truncation below happens
    // at each column's real width - per the "Overflowing content is
    // truncated with an ellipsis" requirement.
    let columns_area = Rect {
        x: inner.x + selection_width,
        width: inner.width.saturating_sub(selection_width),
        ..inner
    };
    let columns = Layout::horizontal(widths).spacing(COLUMN_SPACING).split(columns_area);
    let agents_width = columns[1].width as usize;
    let reminders_width = columns[3].width as usize;

    let rows = groups.iter().enumerate().map(|(i, group)| {
        Row::new([
            Cell::from(project_texts[i].clone()),
            Cell::from(truncate_line(agents_lines[i].clone(), agents_width)),
            Cell::from(tests_lines[i].clone()),
            Cell::from(truncate_line(reminders_lines[i].clone(), reminders_width)),
            Cell::from(format_last_updated(group.most_recent_update_ms())),
        ])
    });

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
fn render_logs_tab(frame: &mut Frame, app: &App, area: Rect, banner: Option<&str>) -> usize {
    let header = Row::new(["PROJECT", "CATEGORY", "STATUS", "TIME"]).style(Style::new().bold());

    let entries = app.visible_logs();
    let rows = entries.iter().map(|entry| {
        let (status_text, status_style) = log_entry_status_line(&app.logs, entry);
        Row::new([
            Cell::from(project_name(&entry.working_dir)),
            Cell::from(log_category_label(entry.category)),
            Cell::from(Span::styled(status_text, status_style)),
            Cell::from(format_last_updated(entry.occurred_at_ms)),
        ])
    });

    let widths = [
        Constraint::Fill(2),
        Constraint::Length(10),
        Constraint::Fill(1),
        Constraint::Length(19),
    ];

    // Computed from the un-titled block so a page is however many data rows
    // (the header row aside) actually fit in the pane at the current
    // terminal size - see design.md's "`page_size` is a parameter" decision.
    let bare_block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded);
    let inner = bare_block.inner(area);
    let page_size = inner.height.saturating_sub(1) as usize;

    let controls = logs_controls_hint(app, entries.len(), page_size);
    let title = match banner {
        Some(banner) => format!("Logs - {banner}  |  {controls}"),
        None => format!("Logs - {controls}"),
    };
    let block = bare_block.title(title);

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(Style::new().bg(SELECTED_ROW_BG).fg(SELECTED_ROW_FG))
        .highlight_symbol("> ")
        .block(block);

    let selected = if entries.is_empty() {
        None
    } else {
        Some(app.logs_pagination.display_selected(entries.len()))
    };
    let top = app.logs_pagination.display_top(entries.len(), page_size);
    let mut state = TableState::default().with_selected(selected).with_offset(top);
    frame.render_stateful_widget(table, area, &mut state);

    if needs_pagination(entries.len(), page_size) {
        render_pagination_scrollbar(frame, area, entries.len(), top, page_size);
    }

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

    page_size
}

/// Renders a Ratatui vertical scrollbar along `area`'s right edge, reflecting
/// a paginated list's current window - used by both the Logs tab and the
/// details modal's Logs pane once their entries overflow a single page, per
/// the "Paginated lists support keyboard navigation" requirement. `area` is
/// the pane's full bordered rect (not its inner content area): the 1-row
/// vertical margin keeps the scrollbar off the border's corner cells.
///
/// `content_length` is the number of valid scroll-window positions
/// (`top`'s actual range is `0..=len - page_size`), not the raw entry count
/// `len` - ratatui's thumb-position math places the thumb flush against the
/// track's end only when `position == content_length - 1`, and `top` never
/// reaches `len - 1` once more than one row fits per page, so passing `len`
/// directly left the thumb stopping short of the bottom on the last page.
fn render_pagination_scrollbar(frame: &mut Frame, area: Rect, len: usize, top: usize, page_size: usize) {
    let content_length = len.saturating_sub(page_size) + 1;
    let mut state = ScrollbarState::new(content_length).position(top).viewport_content_length(page_size);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
    let scrollbar_area = area.inner(Margin { vertical: 1, horizontal: 0 });
    frame.render_stateful_widget(scrollbar, scrollbar_area, &mut state);
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
        agentmon_proto::LogCategory::Reminder => "reminder",
    }
}

/// The details modal's Logs pane visually nests test-run entries beneath the
/// agent activity that ran them by prefixing them with a tree branch marker;
/// agent and reminder entries render unprefixed, forming the trunk of the
/// list. Purely visual - it does not affect entry ordering or the top-level
/// Logs tab.
fn log_entry_tree_prefix(category: agentmon_proto::LogCategory) -> &'static str {
    match category {
        agentmon_proto::LogCategory::Agent | agentmon_proto::LogCategory::Reminder => "",
        agentmon_proto::LogCategory::TestRun => "├─ ",
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
        // A reminder log entry always shows the fixed ⏰ marker regardless of
        // its status (started/stopped/finished) - unlike agent and test-run
        // entries, which use a different emoji per status - per the "Status
        // is visually distinguishable" requirement. The color still varies
        // by status, though: "finished" (a natural completion) matches the
        // Green used for "done"/"passed" elsewhere; "stopped" (a manual
        // interruption, distinct from either a start or a completion) gets
        // its own Yellow rather than sharing "started"'s Blue.
        agentmon_proto::LogCategory::Reminder => {
            let color = match status {
                "finished" => Color::Green,
                "stopped" => Color::Yellow,
                _ => Color::Blue,
            };
            (format!("⏰ reminder {status}"), Style::new().fg(color))
        }
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
            | (agentmon_proto::LogCategory::Reminder, "finished")
            | (agentmon_proto::LogCategory::Reminder, "stopped")
    );
    if !is_completion {
        return None;
    }
    all_logs
        .iter()
        .filter(|e| e.category == entry.category)
        .filter(|e| e.working_dir == entry.working_dir)
        .filter(|e| e.status == "started")
        // A project can have several reminders interleaved in the log, so a
        // reminder completion is only paired with a "started" entry for the
        // same reminder name - unlike agent/test-run entries, which have no
        // such identity to match on and rely on working_dir alone.
        .filter(|e| entry.category != agentmon_proto::LogCategory::Reminder || e.reminder_name == entry.reminder_name)
        .filter(|e| e.occurred_at_ms <= entry.occurred_at_ms)
        .max_by_key(|e| e.occurred_at_ms)
        .map(|started| entry.occurred_at_ms.saturating_sub(started.occurred_at_ms))
}

/// Builds a log entry's styled status text, appending the reminder's name
/// (for reminder-category entries), its ETA (for a "started" reminder entry),
/// and an elapsed duration for a completed run/task, so its total time is
/// visible the same way it already is on the Agents tab - see
/// `log_completion_duration_ms`. The name comes before the ETA/duration
/// (e.g. "reminder finished: Check the build 10m", "reminder started: Check
/// the build, ETA: 10:45"), reading as a single "what, when/how long" phrase.
fn log_entry_status_line(all_logs: &[LogEntry], entry: &LogEntry) -> (String, Style) {
    let (mut text, style) = log_status_cell_text_and_style(entry.category, &entry.status);
    // Carries the reminder's name as of this event, so a later-deleted
    // reminder's past entries keep showing it - per the "A deleted
    // reminder's log entries keep its name" scenario.
    if let Some(name) = &entry.reminder_name {
        text.push_str(&format!(": {name}"));
    }
    // A "started" reminder entry has no elapsed run yet, so it shows its
    // recorded ETA instead of a duration - a point-in-time fact captured
    // when it started, not recomputed from the reminder's current duration.
    if entry.category == agentmon_proto::LogCategory::Reminder && entry.status == "started" {
        if let Some(due_at_ms) = entry.reminder_due_at_ms {
            text.push_str(&format!(", ETA: {}", format_eta_ms(due_at_ms)));
        }
    }
    if let Some(duration_ms) = log_completion_duration_ms(all_logs, entry) {
        text.push(' ');
        let formatted = if entry.category == agentmon_proto::LogCategory::Reminder {
            format_reminder_duration_ms(duration_ms)
        } else {
            format_running_duration(0, duration_ms)
        };
        text.push_str(&formatted);
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

/// Builds the Agents tab's title, reflecting its `/`-search state - see
/// "Agents tab title reflects search state". While the search prompt is
/// being edited, the title becomes the prompt itself (with a visible
/// cursor). Otherwise, the `Filter [/]` keyboard shortcut hint is always
/// shown - mirroring the Logs/Reminders tabs' own `<Word> [<key>]`-style
/// control hints, which stay present whether or not a filter is currently
/// applied - with the applied filter text (if any) appended after it, bold
/// and in a color distinct from the rest of the title.
fn agents_tab_title(app: &App, banner: Option<&str>) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = if app.agents_search_editing {
        vec![
            Span::raw(format!("Agents /{}", app.agents_search_buffer)),
            Span::styled("_", Style::new().add_modifier(Modifier::REVERSED)),
        ]
    } else if let Some(filter) = &app.agents_search_applied {
        vec![
            Span::raw("Agents  Filter [/]: "),
            Span::styled(filter.clone(), Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]
    } else {
        vec![Span::raw("Agents  Filter [/]")]
    };
    if let Some(banner) = banner {
        spans.push(Span::raw(format!(" - {banner}")));
    }
    Line::from(spans)
}

/// Builds the always-visible sort/filter control hint shown in the Logs
/// tab's title, e.g. `Sort [o]: recency  |  Filter: none.  Project [p]
/// Status [s]  Clear [c]` - the keybinding hints stay present whether or not
/// a filter is currently applied, so the user always knows how to reach
/// them.
fn logs_controls_hint(app: &App, len: usize, page_size: usize) -> String {
    let base = format!(
        "Sort [o]: {}  |  Filter: {}.  Project [p]  Status [s]  Clear [c]",
        log_sort_label(app.logs_sort),
        logs_filter_state(app),
    );
    if needs_pagination(len, page_size) {
        // Prepended, not appended: this hint's whole reason for existing is
        // to stay discoverable, so it must survive Ratatui's title
        // truncation on realistic terminal widths - the already-long
        // sort/filter hint alone can already reach the ~80-column ballpark,
        // pushing anything appended after it off the border entirely.
        format!("Page [d/u]  Top/Bottom [g/G]  |  {base}")
    } else {
        base
    }
}

fn logs_filter_state(app: &App) -> String {
    match (&app.logs_filter_project, &app.logs_filter_status) {
        (None, None) => "none".to_string(),
        (Some(project), None) => format!("project={project}"),
        (None, Some(status)) => format!("status={status}"),
        (Some(project), Some(status)) => format!("project={project}, status={status}"),
    }
}

/// Draws `modal` centered on top of whatever tab is currently shown. Returns
/// `(modal_logs_page_size, modal_reminders_page_size)`.
fn render_modal(frame: &mut Frame, app: &App, modal: &Modal) -> (usize, usize) {
    let page_sizes = match modal {
        Modal::Help => {
            render_help_modal(frame);
            (0, 0)
        }
        Modal::Details(cwd) => render_details_modal(frame, app, cwd),
    };

    // The reminder form and delete confirmation are their own layers on top
    // of the details modal - see "Reminder creation and editing form" and
    // "Reminder deletion confirmation".
    if let Some(form) = &app.reminder_form {
        render_reminder_form(frame, form);
    } else if let Some(id) = &app.confirm_delete_reminder {
        render_delete_reminder_confirm(frame, app, id);
    }

    page_sizes
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

/// Renders the selected project's details modal: four panes covering its
/// registered agents, its last test run (if any), its reminders, and its
/// recent activity log entries - see the "Project details modal"
/// requirement. Returns `(logs_page_size, reminders_page_size)` (see
/// `render` / `PageSizes`).
fn render_details_modal(frame: &mut Frame, app: &App, cwd: &Path) -> (usize, usize) {
    let area = centered_rect(80, 80, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(format!("Details - {}", project_name(cwd)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Agents, Tests, and Reminders share the top third, side by side; Logs -
    // typically the longest-running list - gets the remaining two-thirds
    // beneath them.
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)])
        .split(inner);
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(1, 3), Constraint::Ratio(1, 3)])
        .split(rows[0]);
    let (agents_area, tests_area, reminders_area, logs_area) = (top[0], top[1], top[2], rows[1]);

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

    let reminders_page_size = render_reminders_pane(frame, app, cwd, reminders_area);

    let project_logs = app.project_logs(cwd);
    let logs_rows: Vec<Row> = if project_logs.is_empty() {
        vec![Row::new([Cell::from("No activity"), Cell::from("")])]
    } else {
        project_logs
            .iter()
            .map(|entry| {
                let (status_text, style) = log_entry_line_with_pid(&app.logs, entry);
                let prefix = log_entry_tree_prefix(entry.category);
                Row::new([
                    Cell::from(Line::from(vec![
                        Span::raw(prefix),
                        Span::styled(status_text, style),
                    ])),
                    Cell::from(format_last_updated(entry.occurred_at_ms)),
                ])
            })
            .collect()
    };
    let logs_widths = [Constraint::Fill(1), Constraint::Length(19)];

    // No header row in this pane (per the "Project details modal"
    // requirement), so every visible row is content - unlike the Logs tab,
    // whose header eats one row of its page size.
    let bare_logs_block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded);
    let logs_inner = bare_logs_block.inner(logs_area);
    let logs_page_size = logs_inner.height as usize;
    let paginated = needs_pagination(project_logs.len(), logs_page_size);
    let logs_title = if paginated { "Logs  |  Page [d/u]  Top/Bottom [g/G]" } else { "Logs" };
    let logs_block = bare_logs_block.title(logs_title);

    let logs_table = Table::new(logs_rows, logs_widths)
        .row_highlight_style(Style::new().bg(SELECTED_ROW_BG).fg(SELECTED_ROW_FG))
        .block(logs_block);

    // The highlight, like the scrollbar and heading hint, only appears once
    // pagination is actually needed - a project whose activity fits on one
    // page renders exactly as it did before this pane could scroll. The
    // highlight additionally requires this pane to hold focus, now that the
    // Reminders pane can hold it instead.
    let selected = if paginated && matches!(app.modal_focus, ModalFocus::Logs) {
        Some(app.modal_logs_pagination.display_selected(project_logs.len()))
    } else {
        None
    };
    let logs_top = app.modal_logs_pagination.display_top(project_logs.len(), logs_page_size);
    let mut logs_state = TableState::default().with_selected(selected).with_offset(logs_top);
    frame.render_stateful_widget(logs_table, logs_area, &mut logs_state);

    if paginated {
        render_pagination_scrollbar(frame, logs_area, project_logs.len(), logs_top, logs_page_size);
    }

    (logs_page_size, reminders_page_size)
}

/// Renders the details modal's Reminders pane: that project's reminders,
/// most recently updated first, paginated once they overflow the pane - see
/// the "Project details modal" requirement. Unlike the Logs pane, its title
/// never shows a pagination hint, always showing the `New [R]` create hint
/// instead; and it always shows a highlighted selection (when it holds
/// entries and focus) regardless of whether it's paginated, since it's
/// interactive even with just one reminder. Returns its `page_size`.
fn render_reminders_pane(frame: &mut Frame, app: &App, cwd: &Path, area: Rect) -> usize {
    let now = now_ms();
    let reminders = app.project_reminders(cwd);

    let widths = [Constraint::Fill(2), Constraint::Length(9), Constraint::Fill(2)];

    let bare_block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded);
    let inner = bare_block.inner(area);
    let page_size = inner.height as usize;

    // This pane is only a third of the modal's top third, so its Status
    // column has little room - a running reminder's "⏳ Running Xm, ETA:
    // HH:MM" easily overflows it. Resolves the exact column widths the
    // `Table` below will use (including its default 1-column spacing
    // between columns, which a bare `Layout::split` doesn't apply on its
    // own) and truncates to fit - the same technique used for the Agents
    // tab's narrow columns (see `truncate_line`). No `Cell::highlight_symbol`
    // is set on this table, so unlike ratatui's own column-width resolution
    // there's no selection-width reservation to account for here.
    let columns = Layout::horizontal(widths).spacing(1).split(inner);
    let status_width = columns[2].width as usize;

    let rows: Vec<Row> = if reminders.is_empty() {
        vec![Row::new([Cell::from("No reminders"), Cell::from(""), Cell::from("")])]
    } else {
        reminders
            .iter()
            .map(|reminder| {
                let (status_text, style) = reminder_status_cell_text_and_style(reminder, now, false);
                let status_line = truncate_line(Line::from(Span::styled(status_text, style)), status_width);
                Row::new([
                    Cell::from(reminder.name.clone()),
                    Cell::from(format!("{}m", reminder.duration_minutes)),
                    Cell::from(status_line),
                ])
            })
            .collect()
    };

    let block = bare_block.title("Reminders  |  New [R]");

    let table = Table::new(rows, widths)
        .row_highlight_style(Style::new().bg(SELECTED_ROW_BG).fg(SELECTED_ROW_FG))
        .block(block);

    let focused = matches!(app.modal_focus, ModalFocus::Reminders);
    let selected = if focused && !reminders.is_empty() {
        Some(app.modal_reminders_pagination.display_selected(reminders.len()))
    } else {
        None
    };
    let top = app.modal_reminders_pagination.display_top(reminders.len(), page_size);
    let mut state = TableState::default().with_selected(selected).with_offset(top);
    frame.render_stateful_widget(table, area, &mut state);

    if needs_pagination(reminders.len(), page_size) {
        render_pagination_scrollbar(frame, area, reminders.len(), top, page_size);
    }

    page_size
}

/// Renders the keyboard-shortcuts help overlay - see the "Keyboard shortcuts
/// help modal" requirement.
fn render_help_modal(frame: &mut Frame) {
    let area = centered_rect(70, 80, frame.area());
    frame.render_widget(Clear, area);

    let text = [
        "Tab       switch tabs / modal pane focus",
        "A / L / R jump to Agents / Logs / Reminders tab",
        "/         search Agents tab by project (Enter saves, Esc cancels)",
        "j / down  move selection down",
        "k / up    move selection up",
        "d / PgDn  page down",
        "u / PgUp  page up",
        "g / G     jump to first / last entry",
        "Enter     open details / start-stop a reminder",
        "o         cycle sort",
        "p         cycle project filter",
        "s         cycle status filter / start-stop a reminder",
        "f         cycle reminder status filter",
        "c         clear filters",
        "R         create a reminder (modal)",
        "e         edit a reminder (modal)",
        "Del/Bksp  delete a reminder (modal)",
        "?         toggle this help",
        "Esc       close modal / form / dialog",
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

/// Renders the Reminders tab: every project's reminders, aggregated across
/// all projects, filtered/sorted per `App`'s current state - see the
/// "Reminders tab shows an aggregated, paginated reminder list" and
/// "Reminders tab supports sorting and filtering" requirements.
fn render_reminders_tab(frame: &mut Frame, app: &App, area: Rect, banner: Option<&str>) -> usize {
    let header = Row::new(["PROJECT", "NAME", "DURATION", "STATUS", "UPDATED"]).style(Style::new().bold());

    let now = now_ms();
    let entries = app.visible_reminders();
    let rows = entries.iter().map(|reminder| {
        let (status_text, status_style) = reminder_status_cell_text_and_style(reminder, now, false);
        Row::new([
            Cell::from(project_name(&reminder.cwd)),
            Cell::from(reminder.name.clone()),
            Cell::from(format!("{}m", reminder.duration_minutes)),
            Cell::from(Span::styled(status_text, status_style)),
            Cell::from(format_last_updated(reminder.last_updated_ms)),
        ])
    });

    let widths = [
        Constraint::Fill(2),
        Constraint::Fill(2),
        Constraint::Length(9),
        Constraint::Fill(2),
        Constraint::Length(19),
    ];

    let bare_block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded);
    let inner = bare_block.inner(area);
    let page_size = inner.height.saturating_sub(1) as usize;

    let controls = reminders_controls_hint(app, entries.len(), page_size);
    let title = match banner {
        Some(banner) => format!("Reminders - {banner}  |  {controls}"),
        None => format!("Reminders - {controls}"),
    };
    let block = bare_block.title(title);

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(Style::new().bg(SELECTED_ROW_BG).fg(SELECTED_ROW_FG))
        .highlight_symbol("> ")
        .block(block);

    let selected = if entries.is_empty() {
        None
    } else {
        Some(app.reminders_pagination.display_selected(entries.len()))
    };
    let top = app.reminders_pagination.display_top(entries.len(), page_size);
    let mut state = TableState::default().with_selected(selected).with_offset(top);
    frame.render_stateful_widget(table, area, &mut state);

    if needs_pagination(entries.len(), page_size) {
        render_pagination_scrollbar(frame, area, entries.len(), top, page_size);
    }

    if app.reminders.is_empty() || entries.is_empty() {
        let message = if app.reminders.is_empty() {
            "No reminders yet"
        } else {
            "No reminders match the current filter"
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

    page_size
}

fn reminder_sort_label(sort: ReminderSort) -> &'static str {
    match sort {
        ReminderSort::Recency => "recency",
        ReminderSort::Project => "project",
        ReminderSort::Status => "status",
    }
}

fn reminders_filter_state(app: &App) -> String {
    match (&app.reminders_filter_project, &app.reminders_filter_status) {
        (None, None) => "none".to_string(),
        (Some(project), None) => format!("project={project}"),
        (None, Some(status)) => format!("status={status}"),
        (Some(project), Some(status)) => format!("project={project}, status={status}"),
    }
}

/// The Reminders tab's always-visible sort/filter control hint - mirrors
/// `logs_controls_hint`, except the status filter cycles on `f` rather than
/// `s`, since `s` starts/stops the highlighted reminder here.
fn reminders_controls_hint(app: &App, len: usize, page_size: usize) -> String {
    let base = format!(
        "Sort [o]: {}  |  Filter: {}.  Project [p]  Status [f]  Clear [c]  |  Start/Stop [s]",
        reminder_sort_label(app.reminders_sort),
        reminders_filter_state(app),
    );
    if needs_pagination(len, page_size) {
        format!("Page [d/u]  Top/Bottom [g/G]  |  {base}")
    } else {
        base
    }
}

/// Formats an elapsed duration for a reminder: like `format_running_duration`
/// at or above a minute, except a duration landing exactly on a whole minute
/// (no leftover seconds - always true for a natural completion, since it
/// fires exactly at its duration) omits the trailing `00s` (e.g. `10m`
/// rather than `10m00s`), matching the "Done 10m" / "Finished 10m" examples.
fn format_reminder_duration_ms(elapsed_ms: u64) -> String {
    let elapsed_secs = elapsed_ms / 1000;
    let hours = elapsed_secs / 3600;
    let minutes = (elapsed_secs % 3600) / 60;
    let seconds = elapsed_secs % 60;

    if hours > 0 {
        format!("{hours}h{minutes:02}m")
    } else if minutes > 0 && seconds == 0 {
        format!("{minutes}m")
    } else if minutes > 0 {
        format!("{minutes}m{seconds:02}s")
    } else {
        format!("{seconds}s")
    }
}

/// Formats a Unix epoch milliseconds timestamp as the system's local time,
/// `HH:MM` (e.g. `12:34`) - the "ETA" shown wherever a reminder's due time
/// appears, whether computed live (`format_reminder_eta`) or read from a
/// log entry's recorded due time.
fn format_eta_ms(due_at_ms: u64) -> String {
    let datetime = chrono::DateTime::from_timestamp_millis(due_at_ms as i64)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp_millis(0).unwrap());
    datetime.with_timezone(&chrono::Local).format("%H:%M").to_string()
}

/// A running reminder's estimated completion time, in the system's local
/// timezone, formatted `HH:MM` (e.g. `12:34`) - the "ETA" shown alongside its
/// live elapsed duration.
fn format_reminder_eta(run_started_ms: u64, duration_minutes: u32) -> String {
    format_eta_ms(run_started_ms + duration_minutes as u64 * 60_000)
}

/// The Reminders tab/pane's STATUS cell: blank while not-yet-started,
/// `⏳ Running <dur>, ETA: HH:MM` while running, `✅ Done <dur>` once done -
/// see the "Reminders tab shows an aggregated, paginated reminder list"
/// requirement. `with_category_prefix` adds "reminder" after the emoji, for
/// cross-category views (the Agents tab's Reminders column); the top-level
/// Reminders tab and the modal's Reminders pane both pass `false`, since each
/// already dedicates its view to reminders.
fn reminder_status_cell_text_and_style(reminder: &ReminderInfo, now_ms: u64, with_category_prefix: bool) -> (String, Style) {
    let prefix = |emoji: &str| {
        if with_category_prefix {
            format!("{emoji} reminder")
        } else {
            emoji.to_string()
        }
    };
    match reminder.status {
        ReminderStatus::NotYetStarted => (String::new(), Style::new()),
        ReminderStatus::Running => {
            let started = reminder.run_started_ms.unwrap_or(now_ms);
            let elapsed = format_reminder_duration_ms(now_ms.saturating_sub(started));
            let eta = format_reminder_eta(started, reminder.duration_minutes);
            (
                format!("{} Running {elapsed}, ETA: {eta}", prefix("⏳")),
                Style::new().fg(Color::Blue),
            )
        }
        ReminderStatus::Done => {
            let started = reminder.run_started_ms.unwrap_or(reminder.last_updated_ms);
            let elapsed = format_reminder_duration_ms(reminder.last_updated_ms.saturating_sub(started));
            (format!("{} Done {elapsed}", prefix("✅")), Style::new().fg(Color::Green))
        }
    }
}

/// Whether a now-`Done` reminder's most recent transition was a manual stop
/// or a natural finish, reconstructed from the activity log the same way
/// `log_completion_duration_ms` reconstructs a completion's duration - a
/// `ReminderInfo` alone can't distinguish the two, since both set status to
/// `Done`. Defaults to "completed" if no matching log entry is found.
fn reminder_outcome_word(app: &App, reminder: &ReminderInfo) -> &'static str {
    let matched = app
        .logs
        .iter()
        .filter(|e| e.category == agentmon_proto::LogCategory::Reminder)
        .filter(|e| e.working_dir == reminder.cwd)
        .filter(|e| e.reminder_name.as_deref() == Some(reminder.name.as_str()))
        .filter(|e| matches!(e.status.as_str(), "finished" | "stopped"))
        .filter(|e| e.occurred_at_ms <= reminder.last_updated_ms)
        .max_by_key(|e| e.occurred_at_ms);
    match matched.map(|e| e.status.as_str()) {
        Some("stopped") => "stopped",
        _ => "completed",
    }
}

/// Builds one reminder's segment for the Agents tab's REMINDERS column: a
/// status emoji, its name in bold, and its duration/ETA or outcome - see the
/// "Agents tab shows a Reminders column" requirement. No category-word
/// prefix here (unlike the Agents/Tests columns): the column is narrow and
/// already labeled REMINDERS, and the name itself is the primary content.
fn reminder_agents_column_segment(reminder: &ReminderInfo, app: &App, now_ms: u64) -> Vec<Span<'static>> {
    match reminder.status {
        ReminderStatus::NotYetStarted => Vec::new(),
        ReminderStatus::Running => {
            let started = reminder.run_started_ms.unwrap_or(now_ms);
            let elapsed = format_reminder_duration_ms(now_ms.saturating_sub(started));
            let eta = format_reminder_eta(started, reminder.duration_minutes);
            vec![
                Span::styled("⏳ ", Style::new().fg(Color::Blue)),
                Span::styled(reminder.name.clone(), Style::new().add_modifier(Modifier::BOLD)),
                Span::raw(format!(" {elapsed}, ETA {eta}")),
            ]
        }
        ReminderStatus::Done => {
            let started = reminder.run_started_ms.unwrap_or(reminder.last_updated_ms);
            let elapsed = format_reminder_duration_ms(reminder.last_updated_ms.saturating_sub(started));
            let outcome = reminder_outcome_word(app, reminder);
            vec![
                Span::styled("✅ ", Style::new().fg(Color::Green)),
                Span::styled(reminder.name.clone(), Style::new().add_modifier(Modifier::BOLD)),
                Span::raw(format!(" {outcome}, {elapsed}")),
            ]
        }
    }
}

/// Builds a project's REMINDERS cell: one segment per reminder with activity
/// to show (not-yet-started reminders contribute nothing), most recently
/// updated first (per `DirectoryGroup`'s own ordering), joined by " · " like
/// the AGENTS cell.
fn reminders_status_line(group: &DirectoryGroup, app: &App, now_ms: u64) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    for reminder in &group.reminders {
        let segment = reminder_agents_column_segment(reminder, app, now_ms);
        if segment.is_empty() {
            continue;
        }
        if !spans.is_empty() {
            spans.push(Span::raw(" · "));
        }
        spans.extend(segment);
    }
    Line::from(spans)
}

/// A `Line`'s combined display width, in terminal columns.
fn line_width(line: &Line) -> usize {
    use unicode_width::UnicodeWidthStr;
    line.spans.iter().map(|s| s.content.width()).sum()
}

/// A plain string's display width, in terminal columns.
fn str_width(s: &str) -> usize {
    use unicode_width::UnicodeWidthStr;
    s.width()
}

/// A dynamically-sized column's width for this frame: the widest content
/// among `widths` (already-measured display widths for each row), floored
/// at `header`'s own display width so a column with blank/short content
/// still shows its header in full - see `render_agent_table`.
fn content_need(widths: impl Iterator<Item = usize>, header: &str) -> u16 {
    widths.max().unwrap_or(0).max(str_width(header)) as u16
}

/// Truncates `line`'s combined text to at most `max_width` terminal columns,
/// appending a trailing `...` when it had to cut anything short - per the
/// "Overflowing content is truncated with an ellipsis" requirement, applied
/// to the Agents tab's AGENTS and REMINDERS columns and the details modal's
/// Reminders pane. Measured in display width (via `unicode-width`), not
/// `char` count: several status lines start with a double-width emoji, and
/// budgeting by raw character count under-counts those by one column each,
/// letting the truncated-plus-ellipsis text still overflow by that much once
/// ratatui renders it. A `max_width` of `0` (not yet known, e.g. before the
/// first render) is treated as "no limit" rather than truncating everything
/// away.
fn truncate_line(line: Line<'static>, max_width: usize) -> Line<'static> {
    use unicode_width::UnicodeWidthStr;
    let total_width: usize = line.spans.iter().map(|s| s.content.width()).sum();
    if max_width == 0 || total_width <= max_width {
        return line;
    }
    let budget = max_width.saturating_sub(3);
    let mut remaining = budget;
    let mut spans = Vec::new();
    for span in line.spans {
        if remaining == 0 {
            break;
        }
        let mut text = String::new();
        for c in span.content.chars() {
            let w = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
            if w > remaining {
                break;
            }
            text.push(c);
            remaining -= w;
        }
        spans.push(Span::styled(text, span.style));
    }
    spans.push(Span::raw("..."));
    Line::from(spans)
}

/// Renders the reminder creation/editing form overlay on top of the details
/// modal - see "Reminder creation and editing form".
/// Width of the field labels ("Name:     " / "Duration: "), so the cursor
/// position below lines up with where each field's text actually starts.
const REMINDER_FORM_LABEL_WIDTH: u16 = 10;

fn render_reminder_form(frame: &mut Frame, form: &crate::app::ReminderForm) {
    let area = centered_rect(50, 30, frame.area());
    frame.render_widget(Clear, area);

    let title = match &form.mode {
        ReminderFormMode::Create => "New Reminder",
        ReminderFormMode::Edit(_) => "Edit Reminder",
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let inner = block.inner(area);

    let field_style = |field: ReminderFormField| {
        if form.field == field {
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::new()
        }
    };
    let text = vec![
        Line::from(vec![
            Span::styled("Name:     ", field_style(ReminderFormField::Name)),
            Span::raw(form.name.clone()),
        ]),
        Line::from(vec![
            Span::styled("Duration: ", field_style(ReminderFormField::Duration)),
            Span::raw(format!("{}m", form.duration_minutes)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Tab: switch field   Enter: save   Esc: cancel",
            Style::new().fg(Color::DarkGray),
        )),
    ];

    frame.render_widget(Paragraph::new(text).block(block), area);

    // Shows the terminal's own cursor at the end of the active field's
    // typed text, right where the next keystroke will land.
    let (row, field_len) = match form.field {
        ReminderFormField::Name => (0, form.name.chars().count()),
        ReminderFormField::Duration => (1, form.duration_minutes.chars().count()),
    };
    frame.set_cursor_position((inner.x + REMINDER_FORM_LABEL_WIDTH + field_len as u16, inner.y + row));
}

/// Renders the bold, red delete-confirmation dialog on top of the details
/// modal - see "Reminder deletion confirmation".
fn render_delete_reminder_confirm(frame: &mut Frame, app: &App, id: &agentmon_proto::ReminderId) {
    let area = centered_rect(50, 20, frame.area());
    frame.render_widget(Clear, area);

    let name = app
        .reminders
        .iter()
        .find(|r| &r.id == id)
        .map(|r| r.name.as_str())
        .unwrap_or("this reminder");

    let text = vec![
        Line::from(Span::styled(
            format!("Delete \"{name}\"?"),
            Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Enter: delete   Esc: cancel",
            Style::new().fg(Color::DarkGray),
        )),
    ];

    frame.render_widget(
        Paragraph::new(text).block(
            Block::default()
                .title(Span::styled(
                    "Delete Reminder",
                    Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(Color::Red).add_modifier(Modifier::BOLD)),
        ),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentmon_proto::{HostContext, SessionId};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::path::PathBuf;

    fn terminal() -> Terminal<TestBackend> {
        // Widened from 100 (pre-Reminders-column default) so the AGENTS and
        // TESTS columns keep enough real width for their pre-existing test
        // content now that a fifth REMINDERS column shares the row.
        Terminal::new(TestBackend::new(120, 10)).unwrap()
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

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("agentd is not running"), "got:\n{text}");
        assert!(text.contains("Start it with: agentd"), "got:\n{text}");
    }

    #[test]
    fn agents_tab_shows_a_gray_placeholder_in_the_table_body_when_empty() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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
        term.draw(|frame| { render(frame, &app); }).unwrap();
        let text = buffer_text(&term);
        assert!(text.contains('╭'), "Agents tab should use rounded corners, got:\n{text}");
        assert!(!text.contains('┌'), "no sharp corners should remain, got:\n{text}");

        // Logs tab.
        app.set_tab(Tab::Logs);
        term.draw(|frame| { render(frame, &app); }).unwrap();
        let text = buffer_text(&term);
        assert!(text.contains('╭'), "Logs tab should use rounded corners, got:\n{text}");

        // Details modal (Agents/Tests/Logs panes, plus the outer frame).
        app.set_tab(Tab::Agents);
        app.open_details_modal();
        term.draw(|frame| { render(frame, &app); }).unwrap();
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
        term.draw(|frame| { render(frame, &app); }).unwrap();
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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("tests passed"), "got:\n{text}");
        assert!(text.contains("2m14s"), "expected the total run duration, got:\n{text}");
    }

    #[test]
    fn a_failed_test_run_shows_its_total_duration() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_test_run_update(test_run_with_start(999, TestRunStatus::Failed, 0, 45_000));

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

    #[test]
    fn agents_tab_shows_a_slash_hint_with_no_filter_applied_or_being_edited() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(vec![agent("a", AgentStatus::Running, HostContext::Terminal, 1, 0)], Vec::new());

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("Agents  Filter [/]"), "got:\n{text}");
    }

    #[test]
    fn agents_tab_title_becomes_a_live_search_prompt_while_editing() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(vec![agent("a", AgentStatus::Running, HostContext::Terminal, 1, 0)], Vec::new());
        app.open_agents_search();
        app.push_agents_search_char('a');
        app.push_agents_search_char('b');

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let buffer = term.backend().buffer();
        let text = buffer_text(&term);
        assert!(text.contains("Agents /ab"), "got:\n{text}");
        let (prompt_x, prompt_y) = find_text(buffer, "Agents /ab").expect("search prompt should be rendered");
        let cursor_cell = &buffer[(prompt_x + "Agents /ab".len() as u16, prompt_y)];
        assert!(
            cursor_cell.modifier.contains(Modifier::REVERSED),
            "a cursor should be visible immediately after the typed text"
        );
    }

    #[test]
    fn agents_tab_title_shows_the_applied_filter_bold_and_in_a_distinct_color() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(vec![agent("a", AgentStatus::Running, HostContext::Terminal, 1, 0)], Vec::new());
        app.agents_search_applied = Some("xyz".to_string());

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let buffer = term.backend().buffer();
        let text = buffer_text(&term);
        assert!(
            text.contains("Filter [/]"),
            "the keyboard shortcut hint must stay visible even with a filter applied, got:\n{text}"
        );
        assert!(text.contains("Agents  Filter [/]: xyz"), "got:\n{text}");
        let (filter_x, filter_y) = find_text(buffer, "xyz").expect("applied filter text should be rendered");
        let filter_cell = &buffer[(filter_x, filter_y)];
        assert!(filter_cell.modifier.contains(Modifier::BOLD), "the applied filter text should be bold");
        assert_ne!(
            filter_cell.style().fg,
            None,
            "the applied filter text should use a color distinct from the title's plain text"
        );
    }

    #[test]
    fn agents_tab_search_filter_hides_non_matching_rows_and_keeps_matching_rows_in_order() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(
            vec![
                agent_in("/tmp/other-project", "a", AgentStatus::Running, HostContext::Terminal, 1, 1_000),
                agent_in("/tmp/LAB-1234", "b", AgentStatus::Running, HostContext::Terminal, 2, 2_000),
            ],
            Vec::new(),
        );
        app.agents_search_applied = Some("1234".to_string());

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("LAB-1234"), "the matching project must still be shown, got:\n{text}");
        assert!(!text.contains("other-project"), "the non-matching project must be hidden, got:\n{text}");
    }

    fn log_entry(cwd: &str, category: agentmon_proto::LogCategory, status: &str, occurred_at_ms: u64) -> LogEntry {
        LogEntry {
            working_dir: PathBuf::from(cwd),
            category,
            status: status.to_string(),
            occurred_at_ms,
            pid: Some(1),
            reminder_name: None,
            reminder_due_at_ms: None,
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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains('⏳'), "got:\n{text}");
        assert!(text.contains("agent started"), "got:\n{text}");
    }

    #[test]
    fn details_modal_logs_pane_highlights_the_selected_row_once_entries_overflow_a_page() {
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.apply_log_snapshot(
            (0..20u64)
                .map(|i| log_entry("/Users/beet/project", agentmon_proto::LogCategory::Agent, "done", i))
                .collect(),
        );
        app.modal = Some(crate::app::Modal::Details(PathBuf::from("/Users/beet/project")));

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let buffer = term.backend().buffer();
        let (x, y) = find_text(buffer, "done").expect("a log entry's status text should be rendered");
        assert_eq!(
            buffer[(x, y)].bg, SELECTED_ROW_BG,
            "the top (selected) row should be highlighted once the pane paginates"
        );
        assert_eq!(buffer[(x, y)].fg, SELECTED_ROW_FG);
    }

    #[test]
    fn details_modal_logs_pane_shows_a_scrollbar_only_once_entries_overflow_a_page() {
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.apply_log_snapshot(vec![log_entry(
            "/Users/beet/project",
            agentmon_proto::LogCategory::Agent,
            "done",
            0,
        )]);
        app.modal = Some(crate::app::Modal::Details(PathBuf::from("/Users/beet/project")));

        term.draw(|frame| { render(frame, &app); }).unwrap();
        let text = buffer_text(&term);
        assert!(!text.contains('▲'), "no scrollbar expected with a single entry, got:\n{text}");

        app.apply_log_snapshot(
            (0..20u64)
                .map(|i| log_entry("/Users/beet/project", agentmon_proto::LogCategory::Agent, "done", i))
                .collect(),
        );
        term.draw(|frame| { render(frame, &app); }).unwrap();
        let text = buffer_text(&term);
        assert!(text.contains('▲'), "expected a scrollbar once entries overflow a page, got:\n{text}");
    }

    #[test]
    fn details_modal_logs_pane_scrollbar_thumb_reaches_the_bottom_of_its_track_on_the_last_page() {
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.apply_log_snapshot(
            (0..20u64)
                .map(|i| log_entry("/Users/beet/project", agentmon_proto::LogCategory::Agent, "done", i))
                .collect(),
        );
        app.modal = Some(crate::app::Modal::Details(PathBuf::from("/Users/beet/project")));
        // A tiny page_size guess forces `top` past the true max before
        // `display_top`'s final clamp brings it back down to exactly that
        // max - i.e. the true last page - regardless of the pane's actual
        // rendered page size.
        app.jump_modal_logs_to_end(1);

        term.draw(|frame| { render(frame, &app); }).unwrap();
        let buffer = term.backend().buffer();
        let (arrow_x, bottom_arrow_y) = find_text(buffer, "▼").expect("expected a scrollbar with a bottom arrow");
        assert_eq!(
            buffer[(arrow_x, bottom_arrow_y - 1)].symbol(),
            "█",
            "the thumb should be flush against the track's bottom edge on the last page, got:\n{}",
            buffer_text(&term)
        );
    }

    #[test]
    fn details_modal_logs_pane_title_gains_a_pagination_hint_once_entries_overflow_a_page() {
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.apply_log_snapshot(vec![log_entry(
            "/Users/beet/project",
            agentmon_proto::LogCategory::Agent,
            "done",
            0,
        )]);
        app.modal = Some(crate::app::Modal::Details(PathBuf::from("/Users/beet/project")));

        term.draw(|frame| { render(frame, &app); }).unwrap();
        let text = buffer_text(&term);
        assert!(!text.contains("Page [d/u]"), "no pagination hint expected with a single entry, got:\n{text}");

        app.apply_log_snapshot(
            (0..20u64)
                .map(|i| log_entry("/Users/beet/project", agentmon_proto::LogCategory::Agent, "done", i))
                .collect(),
        );
        term.draw(|frame| { render(frame, &app); }).unwrap();
        let text = buffer_text(&term);
        assert!(
            text.contains("Page [d/u]"),
            "expected the Logs pane title to gain a pagination hint once entries overflow a page, got:\n{text}"
        );
    }

    #[test]
    fn details_modal_logs_pane_shows_no_selection_scrollbar_or_hint_when_entries_fit_on_one_page() {
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.apply_log_snapshot(vec![log_entry(
            "/Users/beet/project",
            agentmon_proto::LogCategory::Agent,
            "done",
            0,
        )]);
        app.modal = Some(crate::app::Modal::Details(PathBuf::from("/Users/beet/project")));

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let buffer = term.backend().buffer();
        let text = buffer_text(&term);
        assert!(!text.contains('▲'), "no scrollbar expected when entries fit on one page, got:\n{text}");
        assert!(!text.contains("Page [d/u]"), "no pagination hint expected when entries fit on one page, got:\n{text}");
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                assert_ne!(
                    buffer[(x, y)].bg,
                    SELECTED_ROW_BG,
                    "no row should be highlighted in the Logs pane when it isn't paginated"
                );
            }
        }
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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("Sort [o]: project"), "got:\n{text}");
        assert!(text.contains("Filter: project=project-a."), "got:\n{text}");
        assert!(
            text.contains("Project [p]") && text.contains("Status [s]") && text.contains("Clear [c]"),
            "shortcut hints must remain visible once a filter is applied, got:\n{text}"
        );
    }

    #[test]
    fn logs_tab_pages_by_a_full_page_with_a_1_row_overlap_and_scrolls_the_window() {
        // 100x10 -> an 8-row body, a 6-row bordered pane, and (minus the
        // header row) a 5-row page - see render_logs_tab's page_size.
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.set_tab(crate::app::Tab::Logs);
        app.apply_log_snapshot(
            (0..12u64)
                .map(|i| log_entry(&format!("/tmp/p{i:02}"), agentmon_proto::LogCategory::Agent, "done", i))
                .collect(),
        );

        term.draw(|frame| { render(frame, &app); }).unwrap();
        let text = buffer_text(&term);
        // Most recent first: p11..p07 fill the first 5-row page.
        for i in 7..12 {
            assert!(text.contains(&format!("p{i:02}")), "expected p{i:02} on the first page, got:\n{text}");
        }
        for i in 0..7 {
            assert!(!text.contains(&format!("p{i:02}")), "p{i:02} should not be visible on the first page, got:\n{text}");
        }

        app.page_logs(1, 5); // page down: advances by page_size - 1 = 4 rows
        term.draw(|frame| { render(frame, &app); }).unwrap();
        let text = buffer_text(&term);
        // The new page runs from p07 through p03, overlapping the previous
        // page's last row (p07) by exactly 1 row.
        for i in 3..8 {
            assert!(text.contains(&format!("p{i:02}")), "expected p{i:02} on the second page, got:\n{text}");
        }
        assert!(!text.contains("p02"), "p02 should not be visible yet, got:\n{text}");
        assert!(!text.contains("p09"), "p09 should have scrolled off the previous page, got:\n{text}");
    }

    #[test]
    fn logs_tab_shows_a_scrollbar_only_once_entries_overflow_a_page() {
        let mut term = terminal(); // page_size = 5, see above
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.set_tab(crate::app::Tab::Logs);
        app.apply_log_snapshot(vec![log_entry("/tmp/a", agentmon_proto::LogCategory::Agent, "done", 1)]);

        term.draw(|frame| { render(frame, &app); }).unwrap();
        let text = buffer_text(&term);
        assert!(!text.contains('▲'), "no scrollbar expected with a single entry, got:\n{text}");

        app.apply_log_snapshot(
            (0..8u64)
                .map(|i| log_entry(&format!("/tmp/p{i}"), agentmon_proto::LogCategory::Agent, "done", i))
                .collect(),
        );
        term.draw(|frame| { render(frame, &app); }).unwrap();
        let text = buffer_text(&term);
        assert!(text.contains('▲'), "expected a scrollbar once entries overflow a page, got:\n{text}");
    }

    #[test]
    fn logs_tab_scrollbar_thumb_reaches_the_bottom_of_its_track_on_the_last_page() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.set_tab(crate::app::Tab::Logs);
        app.apply_log_snapshot(
            (0..13u64)
                .map(|i| log_entry(&format!("/tmp/p{i}"), agentmon_proto::LogCategory::Agent, "done", i))
                .collect(),
        );
        // A tiny page_size guess forces `top` past the true max before
        // `display_top`'s final clamp brings it back down to exactly that
        // max - i.e. the true last page - regardless of the Logs tab's
        // actual rendered page size.
        app.jump_logs_to_end(1);

        term.draw(|frame| { render(frame, &app); }).unwrap();
        let buffer = term.backend().buffer();
        let (arrow_x, bottom_arrow_y) = find_text(buffer, "▼").expect("expected a scrollbar with a bottom arrow");
        assert_eq!(
            buffer[(arrow_x, bottom_arrow_y - 1)].symbol(),
            "█",
            "the thumb should be flush against the track's bottom edge on the last page, got:\n{}",
            buffer_text(&term)
        );
    }

    #[test]
    fn logs_tab_shows_a_pagination_hint_only_once_entries_overflow_a_page() {
        let mut term = terminal(); // page_size = 5, see above
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.set_tab(crate::app::Tab::Logs);
        app.apply_log_snapshot(vec![log_entry("/tmp/a", agentmon_proto::LogCategory::Agent, "done", 1)]);

        term.draw(|frame| { render(frame, &app); }).unwrap();
        let text = buffer_text(&term);
        assert!(!text.contains("Page [d/u]"), "no pagination hint expected with a single entry, got:\n{text}");

        app.apply_log_snapshot(
            (0..8u64)
                .map(|i| log_entry(&format!("/tmp/p{i}"), agentmon_proto::LogCategory::Agent, "done", i))
                .collect(),
        );
        term.draw(|frame| { render(frame, &app); }).unwrap();
        let text = buffer_text(&term);
        assert!(
            text.contains("Page [d/u]"),
            "expected a pagination hint alongside the sort/filter hints once entries overflow a page, got:\n{text}"
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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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
        term.draw(|frame| { render(frame, &app); }).unwrap();
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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("No test run"), "got:\n{text}");
        assert!(text.contains("No activity"), "got:\n{text}");
    }

    #[test]
    fn details_modal_agents_pane_shows_each_agents_pid() {
        // Wider than the default 100: the Agents pane now shares the
        // modal's top third with Tests and Reminders (a three-way split
        // instead of the previous 50/50), so it needs more absolute width
        // to fit this row's long duration text (`status_since_ms: 0` makes
        // for a many-digit hour count) alongside the pid.
        let mut term = Terminal::new(TestBackend::new(160, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(vec![agent("a", AgentStatus::Running, HostContext::Terminal, 777_777, 0)], Vec::new());
        app.open_details_modal();

        term.draw(|frame| { render(frame, &app); }).unwrap();

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
                reminder_name: None,
                reminder_due_at_ms: None,
            },
            LogEntry {
                working_dir: PathBuf::from("/Users/beet/project"),
                category: agentmon_proto::LogCategory::TestRun,
                status: "failed".to_string(),
                occurred_at_ms: 2_000,
                pid: Some(666_666),
                reminder_name: None,
                reminder_due_at_ms: None,
            },
        ]);
        app.modal = Some(crate::app::Modal::Details(PathBuf::from("/Users/beet/project")));

        term.draw(|frame| { render(frame, &app); }).unwrap();

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
    fn details_modal_logs_pane_nests_a_test_run_entry_under_the_tree_prefix() {
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.apply_log_snapshot(vec![log_entry(
            "/Users/beet/project",
            agentmon_proto::LogCategory::TestRun,
            "passed",
            0,
        )]);
        app.modal = Some(crate::app::Modal::Details(PathBuf::from("/Users/beet/project")));

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let text = buffer_text(&term);
        assert!(
            text.contains("├─ ✅"),
            "expected the test-run entry to be prefixed with a tree branch marker, got:\n{text}"
        );
    }

    #[test]
    fn details_modal_logs_pane_renders_agent_entries_without_a_tree_prefix() {
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.apply_log_snapshot(vec![log_entry(
            "/Users/beet/project",
            agentmon_proto::LogCategory::Agent,
            "done",
            0,
        )]);
        app.modal = Some(crate::app::Modal::Details(PathBuf::from("/Users/beet/project")));

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let text = buffer_text(&term);
        assert!(
            !text.contains("├─"),
            "an agent entry must not render a tree branch prefix, got:\n{text}"
        );
    }

    #[test]
    fn details_modal_logs_pane_prefixes_each_consecutive_test_run_entry() {
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        app.apply_log_snapshot(vec![
            log_entry("/Users/beet/project", agentmon_proto::LogCategory::Agent, "started", 0),
            log_entry("/Users/beet/project", agentmon_proto::LogCategory::TestRun, "started", 1_000),
            log_entry("/Users/beet/project", agentmon_proto::LogCategory::TestRun, "failed", 2_000),
            log_entry("/Users/beet/project", agentmon_proto::LogCategory::Agent, "done", 3_000),
        ]);
        app.modal = Some(crate::app::Modal::Details(PathBuf::from("/Users/beet/project")));

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let text = buffer_text(&term);
        let branch_count = text.matches("├─ ").count();
        assert_eq!(
            branch_count, 2,
            "expected both consecutive test-run entries to each get their own branch marker, got:\n{text}"
        );
        assert!(text.contains("agent started"), "got:\n{text}");
        assert!(text.contains("agent done"), "got:\n{text}");
    }

    #[test]
    fn details_modal_logs_pane_tree_prefix_coexists_with_an_agent_entrys_pid() {
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
                reminder_name: None,
                reminder_due_at_ms: None,
            },
            LogEntry {
                working_dir: PathBuf::from("/Users/beet/project"),
                category: agentmon_proto::LogCategory::TestRun,
                status: "failed".to_string(),
                occurred_at_ms: 2_000,
                pid: Some(666_666),
                reminder_name: None,
                reminder_due_at_ms: None,
            },
        ]);
        app.modal = Some(crate::app::Modal::Details(PathBuf::from("/Users/beet/project")));

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let text = buffer_text(&term);
        assert!(
            text.contains("pid 555555"),
            "expected the agent entry's pid in the Logs pane, got:\n{text}"
        );
        for line in text.lines() {
            if line.contains("pid 555555") {
                assert!(
                    !line.trim_start().starts_with("├─"),
                    "the agent entry's own line must not start with a tree prefix, got line:\n{line}"
                );
            }
        }
        assert!(
            text.contains("├─ ❌"),
            "expected the test-run entry to keep its tree prefix, got:\n{text}"
        );
    }

    #[test]
    fn details_modal_logs_pane_aligns_timestamps_in_a_fixed_far_right_column() {
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());
        let short_entry = LogEntry {
            working_dir: PathBuf::from("/Users/beet/project"),
            category: agentmon_proto::LogCategory::TestRun,
            status: "failed".to_string(),
            occurred_at_ms: 0,
            pid: Some(1),
            reminder_name: None,
            reminder_due_at_ms: None,
        };
        let long_entry = LogEntry {
            working_dir: PathBuf::from("/Users/beet/project"),
            category: agentmon_proto::LogCategory::Agent,
            status: "started".to_string(),
            occurred_at_ms: 5_000_000_000,
            pid: Some(555_555),
            reminder_name: None,
            reminder_due_at_ms: None,
        };
        app.apply_log_snapshot(vec![short_entry.clone(), long_entry.clone()]);
        app.modal = Some(crate::app::Modal::Details(PathBuf::from("/Users/beet/project")));

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let buffer = term.backend().buffer();
        let short_ts = format_last_updated(short_entry.occurred_at_ms);
        let long_ts = format_last_updated(long_entry.occurred_at_ms);
        let (short_x, _) = find_text(buffer, &short_ts)
            .unwrap_or_else(|| panic!("expected to find '{short_ts}' in the Logs pane"));
        let (long_x, _) = find_text(buffer, &long_ts)
            .unwrap_or_else(|| panic!("expected to find '{long_ts}' in the Logs pane"));
        assert_eq!(
            short_x, long_x,
            "expected both entries' timestamps to start at the same fixed column despite \
             differently-lengthed status text (short entry's text is much shorter than the long \
             entry's 'agent started ... pid 555555'), got short_x={short_x} long_x={long_x}"
        );
    }

    #[test]
    fn closing_the_details_modal_returns_to_the_agents_tab() {
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(vec![agent("a", AgentStatus::Running, HostContext::Terminal, 4242, 0)], Vec::new());
        app.open_details_modal();
        app.close_modal();

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let text = buffer_text(&term);
        assert!(!text.contains("Details -"), "modal must not still be shown, got:\n{text}");
        assert!(text.contains("Agents"), "got:\n{text}");
    }

    #[test]
    fn tab_bar_shows_shortcut_hints_and_the_title() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new());

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

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

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("Keyboard Shortcuts"), "got:\n{text}");
        assert!(text.contains("quit"), "got:\n{text}");
        assert!(text.contains("g / G"), "expected the jump-to-first/last shortcut to be listed, got:\n{text}");
    }

    fn reminder_in(
        cwd: &str,
        id: &str,
        name: &str,
        status: agentmon_proto::ReminderStatus,
        duration_minutes: u32,
        run_started_ms: Option<u64>,
        last_updated_ms: u64,
    ) -> agentmon_proto::ReminderInfo {
        agentmon_proto::ReminderInfo {
            id: agentmon_proto::ReminderId(id.to_string()),
            cwd: PathBuf::from(cwd),
            name: name.to_string(),
            duration_minutes,
            status,
            created_at_ms: 0,
            run_started_ms,
            last_updated_ms,
        }
    }

    #[test]
    fn reminders_tab_lists_reminders_across_projects() {
        let mut term = terminal();
        let mut app = App::new();
        app.apply_snapshot(Vec::new(), Vec::new()); // marks the connection Connected
        app.set_tab(Tab::Reminders);
        app.apply_reminder_snapshot(vec![reminder_in(
            "/tmp/project",
            "r1",
            "Check the build",
            agentmon_proto::ReminderStatus::NotYetStarted,
            10,
            None,
            0,
        )]);

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("Check the build"), "got:\n{text}");
        assert!(text.contains("10m"), "expected the duration column, got:\n{text}");
    }

    #[test]
    fn reminders_tab_shows_a_blank_status_for_a_never_started_reminder() {
        let entries = [reminder_in(
            "/tmp/project",
            "r1",
            "Check the build",
            agentmon_proto::ReminderStatus::NotYetStarted,
            10,
            None,
            0,
        )];
        let (text, _style) = reminder_status_cell_text_and_style(&entries[0], now_ms(), false);
        assert_eq!(text, "");
    }

    #[test]
    fn reminders_tab_shows_a_running_reminders_live_duration_and_eta() {
        let now = 1_700_000_000_000u64;
        let reminder = reminder_in(
            "/tmp/project",
            "r1",
            "Check the build",
            agentmon_proto::ReminderStatus::Running,
            10,
            Some(now - 83_000),
            now - 83_000,
        );

        let (text, _style) = reminder_status_cell_text_and_style(&reminder, now, false);

        assert!(text.contains("Running"), "got: {text}");
        assert!(text.contains("1m23s"), "got: {text}");
        assert!(text.contains("ETA:"), "got: {text}");
    }

    #[test]
    fn reminders_tab_shows_a_done_reminders_total_duration_without_seconds() {
        let reminder = reminder_in(
            "/tmp/project",
            "r1",
            "Check the build",
            agentmon_proto::ReminderStatus::Done,
            10,
            Some(0),
            600_000, // exactly 10 minutes elapsed
        );

        let (text, _style) = reminder_status_cell_text_and_style(&reminder, now_ms(), false);

        assert_eq!(text, "✅ Done 10m");
    }

    #[test]
    fn agents_tab_reminders_column_shows_a_running_reminder() {
        let mut term = terminal();
        let mut app = App::new();
        let now = now_ms();
        app.apply_snapshot(vec![agent("a", AgentStatus::Running, HostContext::Terminal, 1, now)], Vec::new());
        // A short name: the REMINDERS column is intentionally narrow (see
        // "Overflowing content is truncated with an ellipsis"), and that
        // truncation behavior has its own dedicated test below - this one
        // just checks the reminder's name appears at all.
        app.apply_reminder_snapshot(vec![reminder_in(
            "/Users/beet/project",
            "r1",
            "Build",
            agentmon_proto::ReminderStatus::Running,
            10,
            Some(now),
            now,
        )]);

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("Build"), "got:\n{text}");
    }

    #[test]
    fn agents_tab_reminders_column_truncates_with_an_ellipsis_on_a_narrow_terminal() {
        // Even though REMINDERS is now sized dynamically to its content (see
        // `content_need`), a single row's unusually long content is still
        // capped at half the table's dynamic space so it can't dominate the
        // row and starve every other column - this exercises that cap.
        let mut term = terminal();
        let mut app = App::new();
        let now = now_ms();
        app.apply_snapshot(vec![agent("a", AgentStatus::Running, HostContext::Terminal, 1, now)], Vec::new());
        app.apply_reminder_snapshot(vec![reminder_in(
            "/Users/beet/project",
            "r1",
            "A very long reminder name that will not fit in the column",
            agentmon_proto::ReminderStatus::Running,
            10,
            Some(now),
            now,
        )]);

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let text = buffer_text(&term);
        assert!(
            !text.contains("A very long reminder name that will not fit in the column"),
            "expected the long name to be truncated, got:\n{text}"
        );
        assert!(text.contains("..."), "expected a trailing ellipsis, got:\n{text}");
    }

    #[test]
    fn agents_tab_reminders_column_sizes_to_content_instead_of_a_fixed_ratio() {
        // Regression test: previously REMINDERS was a fixed, narrow Fill(1)
        // slice regardless of how little space Agents/Tests actually used,
        // so a moderately long (but reasonable) reminder name got clipped
        // even with plenty of unused whitespace elsewhere in the row. With
        // dynamic sizing, a short Agents/Tests row frees that space up for
        // Reminders to use instead.
        let mut term = terminal();
        let mut app = App::new();
        let now = now_ms();
        // A short agent status ("idle") leaves the AGENTS column with very
        // little real content, unlike the wide multi-segment rows other
        // tests use.
        app.apply_snapshot(vec![agent("a", AgentStatus::Idle, HostContext::Terminal, 1, now)], Vec::new());
        let reminder_name = "Check the nightly build";
        app.apply_reminder_snapshot(vec![reminder_in(
            "/Users/beet/project",
            "r1",
            reminder_name,
            agentmon_proto::ReminderStatus::Done,
            10,
            Some(now - 600_000),
            now,
        )]);

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let text = buffer_text(&term);
        assert!(
            text.contains(reminder_name),
            "expected the full reminder name to fit once Agents/Tests freed up their unused space, got:\n{text}"
        );
    }

    #[test]
    fn agents_tab_reminders_column_is_empty_for_a_project_with_no_reminders() {
        let group = DirectoryGroup {
            cwd: PathBuf::from("/tmp/project"),
            agents: vec![agent("a", AgentStatus::Running, HostContext::Terminal, 1, 0)],
            test_runs: Vec::new(),
            reminders: Vec::new(),
        };
        let app = App::new();

        let line = reminders_status_line(&group, &app, now_ms());

        assert!(line.spans.is_empty());
    }

    #[test]
    fn agents_tab_reminders_column_shows_multiple_reminders_bullet_separated() {
        let group = DirectoryGroup {
            cwd: PathBuf::from("/tmp/project"),
            agents: Vec::new(),
            test_runs: Vec::new(),
            reminders: vec![
                reminder_in(
                    "/tmp/project",
                    "r1",
                    "First",
                    agentmon_proto::ReminderStatus::Running,
                    10,
                    Some(0),
                    0,
                ),
                reminder_in(
                    "/tmp/project",
                    "r2",
                    "Second",
                    agentmon_proto::ReminderStatus::Done,
                    10,
                    Some(0),
                    600_000,
                ),
            ],
        };
        let app = App::new();

        let line = reminders_status_line(&group, &app, 1_000);
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

        assert!(text.contains("First"), "got: {text}");
        assert!(text.contains("Second"), "got: {text}");
        assert!(text.contains(" · "), "expected a bullet separator between reminders, got: {text}");
    }

    #[test]
    fn agents_tab_reminders_column_distinguishes_completed_from_stopped() {
        let app_with_log = |status: &str| {
            let mut app = App::new();
            app.apply_log_snapshot(vec![LogEntry {
                working_dir: PathBuf::from("/tmp/project"),
                category: agentmon_proto::LogCategory::Reminder,
                status: status.to_string(),
                occurred_at_ms: 500,
                pid: None,
                reminder_name: Some("Check the build".to_string()),
                reminder_due_at_ms: None,
            }]);
            app
        };
        let reminder = reminder_in(
            "/tmp/project",
            "r1",
            "Check the build",
            agentmon_proto::ReminderStatus::Done,
            10,
            Some(0),
            500,
        );

        assert_eq!(reminder_outcome_word(&app_with_log("finished"), &reminder), "completed");
        assert_eq!(reminder_outcome_word(&app_with_log("stopped"), &reminder), "stopped");
    }

    #[test]
    fn truncate_line_appends_an_ellipsis_when_content_overflows() {
        let line = Line::from(vec![Span::raw("hello world")]);

        let truncated = truncate_line(line.clone(), 8);

        let text: String = truncated.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.ends_with("..."), "got: {text}");
        assert!(text.chars().count() <= 8, "got: {text}");
    }

    #[test]
    fn truncate_line_leaves_content_that_already_fits_unchanged() {
        let line = Line::from(vec![Span::raw("short")]);

        let truncated = truncate_line(line.clone(), 20);

        let text: String = truncated.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "short");
    }

    #[test]
    fn details_modal_shows_the_reminders_pane_with_the_create_hint() {
        let mut term = Terminal::new(TestBackend::new(160, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(vec![agent("a", AgentStatus::Running, HostContext::Terminal, 1, 0)], Vec::new());
        app.open_details_modal();

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("New [R]"), "got:\n{text}");
        assert!(text.contains("No reminders"), "got:\n{text}");
    }

    #[test]
    fn details_modal_reminders_pane_lists_the_projects_reminders() {
        let mut term = Terminal::new(TestBackend::new(160, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(vec![agent("a", AgentStatus::Running, HostContext::Terminal, 1, 0)], Vec::new());
        app.apply_reminder_update(reminder_in(
            "/Users/beet/project",
            "r1",
            "Check the build",
            agentmon_proto::ReminderStatus::NotYetStarted,
            10,
            None,
            0,
        ));
        app.open_details_modal();

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("Check the build"), "got:\n{text}");
    }

    #[test]
    fn details_modal_reminders_pane_shows_the_duration_after_the_name() {
        let mut term = Terminal::new(TestBackend::new(160, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(vec![agent("a", AgentStatus::Running, HostContext::Terminal, 1, 0)], Vec::new());
        app.apply_reminder_update(reminder_in(
            "/Users/beet/project",
            "r1",
            "Check the build",
            agentmon_proto::ReminderStatus::NotYetStarted,
            10,
            None,
            0,
        ));
        app.open_details_modal();

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let buffer = term.backend().buffer();
        let (name_x, name_y) = find_text(buffer, "Check the build").expect("expected the reminder's name");
        let (duration_x, duration_y) = find_text_from(buffer, "10m", name_y).expect("expected the duration");
        assert_eq!(duration_y, name_y, "the duration should be on the same row as the name");
        assert!(duration_x > name_x, "the duration should come after the name");
    }

    #[test]
    fn details_modal_reminders_pane_truncates_a_running_reminders_status_instead_of_overflowing() {
        // Regression test: the Reminders pane is only a third of the
        // modal's top third, so a running reminder's "Running Xm, ETA:
        // HH:MM" status must be truncated to fit rather than spilling past
        // the pane's right border.
        let mut term = Terminal::new(TestBackend::new(160, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(vec![agent("a", AgentStatus::Running, HostContext::Terminal, 1, 0)], Vec::new());
        let now = now_ms();
        app.apply_reminder_update(reminder_in(
            "/Users/beet/project",
            "r1",
            "Check the build",
            agentmon_proto::ReminderStatus::Running,
            10,
            Some(now - 83_000),
            now - 83_000,
        ));
        app.open_details_modal();

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let text = buffer_text(&term);
        assert!(
            !text.contains("Running 1m23s, ETA:"),
            "the full status text should be truncated rather than rendered in full, got:\n{text}"
        );
        assert!(text.contains("..."), "expected a trailing ellipsis on the truncated status, got:\n{text}");

        // No row extends past the modal's own right border: every "│"
        // border character in the modal's row band should still be the
        // rightmost non-blank glyph on its row.
        let buffer = term.backend().buffer();
        let (modal_right_border_x, top_y) =
            find_text(buffer, "╮").expect("expected the modal's top-right corner");
        for y in top_y..(top_y + 10) {
            let cell = &buffer[(modal_right_border_x, y)];
            assert!(
                cell.symbol() == "│" || cell.symbol() == "╮" || cell.symbol() == "╯",
                "expected the modal's right border at ({modal_right_border_x}, {y}), got {:?}",
                cell.symbol()
            );
        }
    }

    #[test]
    fn reminder_log_entry_shows_the_clock_marker_and_the_reminder_name() {
        let entry = LogEntry {
            working_dir: PathBuf::from("/tmp/project"),
            category: agentmon_proto::LogCategory::Reminder,
            status: "finished".to_string(),
            occurred_at_ms: 1,
            pid: None,
            reminder_name: Some("Check the build".to_string()),
            reminder_due_at_ms: None,
        };

        let (text, _style) = log_entry_status_line(&[], &entry);

        assert!(text.contains('⏰'), "got: {text}");
        assert!(text.contains("Check the build"), "got: {text}");
    }

    fn reminder_log(status: &str, name: &str, occurred_at_ms: u64) -> LogEntry {
        LogEntry {
            working_dir: PathBuf::from("/tmp/project"),
            category: agentmon_proto::LogCategory::Reminder,
            status: status.to_string(),
            occurred_at_ms,
            pid: None,
            reminder_name: Some(name.to_string()),
            reminder_due_at_ms: None,
        }
    }

    #[test]
    fn a_finished_reminder_shows_its_elapsed_duration() {
        let all_logs = vec![reminder_log("started", "Check the build", 0), reminder_log("finished", "Check the build", 600_000)];

        let (text, _style) = log_entry_status_line(&all_logs, &all_logs[1]);

        assert!(text.contains("10m"), "expected the elapsed duration, got: {text}");
        assert!(!text.contains("10m00s"), "reminder durations should drop trailing 00s, got: {text}");
    }

    #[test]
    fn a_stopped_reminder_shows_its_elapsed_duration() {
        let all_logs = vec![reminder_log("started", "Check the build", 0), reminder_log("stopped", "Check the build", 83_000)];

        let (text, _style) = log_entry_status_line(&all_logs, &all_logs[1]);

        assert!(text.contains("1m23s"), "expected the elapsed duration, got: {text}");
    }

    #[test]
    fn a_started_reminder_shows_no_duration() {
        let all_logs = vec![reminder_log("started", "Check the build", 0)];

        let (text, _style) = log_entry_status_line(&all_logs, &all_logs[0]);

        assert_eq!(
            text, "⏰ reminder started: Check the build",
            "a started entry should show no duration"
        );
    }

    #[test]
    fn a_started_reminder_shows_its_eta() {
        let entry = LogEntry {
            working_dir: PathBuf::from("/tmp/project"),
            category: agentmon_proto::LogCategory::Reminder,
            status: "started".to_string(),
            occurred_at_ms: 0,
            pid: None,
            reminder_name: Some("Check the build".to_string()),
            reminder_due_at_ms: Some(600_000),
        };

        let (text, _style) = log_entry_status_line(&[], &entry);

        assert_eq!(
            text,
            format!("⏰ reminder started: Check the build, ETA: {}", format_eta_ms(600_000)),
            "got: {text}"
        );
    }

    #[test]
    fn a_stopped_or_finished_reminder_still_shows_duration_not_an_eta() {
        let all_logs = vec![reminder_log("started", "Check the build", 0), reminder_log("stopped", "Check the build", 83_000)];

        let (text, _style) = log_entry_status_line(&all_logs, &all_logs[1]);

        assert!(text.contains("1m23s"), "expected the elapsed duration, got: {text}");
        assert!(!text.contains("ETA"), "a stopped entry should not show an ETA, got: {text}");
    }

    #[test]
    fn interleaved_reminders_in_the_same_project_do_not_cross_match_durations() {
        // Two different reminders in the same project, started/finished out
        // of order - the "finished" duration must be measured from its own
        // reminder's "started" entry, not the other one's.
        let all_logs = vec![
            reminder_log("started", "First", 0),
            reminder_log("started", "Second", 10_000),
            reminder_log("finished", "Second", 70_000),
            reminder_log("finished", "First", 600_000),
        ];

        let (first_text, _) = log_entry_status_line(&all_logs, &all_logs[3]);
        let (second_text, _) = log_entry_status_line(&all_logs, &all_logs[2]);

        assert!(first_text.contains("10m"), "First ran for 10m (600_000ms from t=0), got: {first_text}");
        assert!(second_text.contains("1m"), "Second ran for 1m (60_000ms from t=10_000), got: {second_text}");
    }

    #[test]
    fn reminder_log_entry_uses_the_clock_marker_regardless_of_status() {
        for status in ["started", "stopped", "finished"] {
            let (text, _style) = log_status_cell_text_and_style(agentmon_proto::LogCategory::Reminder, status);
            assert!(text.starts_with('⏰'), "status {status} got: {text}");
        }
    }

    #[test]
    fn reminder_log_entry_colors_differ_by_status() {
        let (_, started_style) = log_status_cell_text_and_style(agentmon_proto::LogCategory::Reminder, "started");
        let (_, stopped_style) = log_status_cell_text_and_style(agentmon_proto::LogCategory::Reminder, "stopped");
        let (_, finished_style) = log_status_cell_text_and_style(agentmon_proto::LogCategory::Reminder, "finished");

        assert_eq!(finished_style.fg, Some(Color::Green), "a natural completion should be green");
        assert_eq!(stopped_style.fg, Some(Color::Yellow), "a manual stop should be yellow");
        assert_eq!(started_style.fg, Some(Color::Blue));
        assert_ne!(stopped_style.fg, started_style.fg, "stopped must be visually distinct from started");
        assert_ne!(stopped_style.fg, finished_style.fg, "stopped must be visually distinct from finished");
    }

    #[test]
    fn reminder_delete_confirmation_dialog_names_the_reminder_in_red_bold() {
        let mut term = Terminal::new(TestBackend::new(160, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(vec![agent("a", AgentStatus::Running, HostContext::Terminal, 1, 0)], Vec::new());
        app.apply_reminder_update(reminder_in(
            "/Users/beet/project",
            "r1",
            "Check the build",
            agentmon_proto::ReminderStatus::NotYetStarted,
            10,
            None,
            0,
        ));
        app.open_details_modal();
        app.open_delete_reminder_confirm();

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("Delete \"Check the build\"?"), "got:\n{text}");
        // Searches for the dialog's own full line (rather than just "Check
        // the build", which also appears - unstyled - in the modal's
        // Reminders pane behind the dialog) so the style check lands on the
        // dialog's own red text, not the background pane's.
        let (x, y) = find_text(term.backend().buffer(), "Delete \"Check the build\"?")
            .expect("expected the dialog's own line");
        let cell = term.backend().buffer().cell((x, y)).unwrap();
        assert!(cell.style().fg == Some(Color::Red), "the dialog text should be red");
    }

    #[test]
    fn reminder_form_renders_its_fields_and_hint() {
        let mut term = Terminal::new(TestBackend::new(160, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(vec![agent("a", AgentStatus::Running, HostContext::Terminal, 1, 0)], Vec::new());
        app.open_details_modal();
        app.open_reminder_create_form();
        app.reminder_form_input_char('X');

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let text = buffer_text(&term);
        assert!(text.contains("New Reminder"), "got:\n{text}");
        assert!(text.contains("Name:"), "got:\n{text}");
        assert!(text.contains("Duration:"), "got:\n{text}");
        assert!(text.contains("Enter: save"), "got:\n{text}");
    }

    #[test]
    fn reminder_form_shows_the_cursor_at_the_end_of_the_active_fields_text() {
        let mut term = Terminal::new(TestBackend::new(160, 30)).unwrap();
        let mut app = App::new();
        app.apply_snapshot(vec![agent("a", AgentStatus::Running, HostContext::Terminal, 1, 0)], Vec::new());
        app.open_details_modal();
        app.open_reminder_create_form();
        app.reminder_form_input_char('X');
        app.reminder_form_input_char('Y');

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let (name_x, name_y) =
            find_text(term.backend().buffer(), "Name:").expect("expected the Name label");
        let cursor = term.get_cursor_position().expect("cursor position should be set");
        assert_eq!(
            cursor,
            ratatui::layout::Position { x: name_x + REMINDER_FORM_LABEL_WIDTH + 2, y: name_y },
            "cursor should sit right after the two typed characters on the Name line"
        );

        app.toggle_reminder_form_field();
        app.reminder_form_input_char('5');

        term.draw(|frame| { render(frame, &app); }).unwrap();

        let (duration_x, duration_y) =
            find_text(term.backend().buffer(), "Duration:").expect("expected the Duration label");
        let cursor = term.get_cursor_position().expect("cursor position should be set");
        assert_eq!(
            cursor,
            ratatui::layout::Position { x: duration_x + REMINDER_FORM_LABEL_WIDTH + 1, y: duration_y },
            "cursor should sit right after the typed digit, before the trailing 'm'"
        );
    }
}
