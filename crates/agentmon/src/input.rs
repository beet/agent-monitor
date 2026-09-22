use crossterm::event::{KeyCode, KeyEvent};

use agentmon_proto::ClientMessage;

use crate::app::{App, Modal, ModalFocus, PageSizes, Tab};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputAction {
    Continue,
    Quit,
    /// A reminder command (create/edit/delete/start/stop) that the caller
    /// should send to the daemon - `App` itself only holds UI state and
    /// never talks to the socket directly, per design.md's "Mutation
    /// commands are fire-and-forget" decision.
    SendReminderCommand(ClientMessage),
}

/// Maps a key event to an action, applying it to `app` along the way.
/// Terminal-independent, so it can be tested with synthetic `KeyEvent`s and
/// without a real terminal. `page_sizes` carries however many rows each
/// paginated list actually rendered on the most recent frame - see
/// `PageSizes`.
///
/// A modal, if open, takes priority over tab-level keys: only `Esc` (close),
/// `q` (quit), and - for `Modal::Details` - its own focused pane's pagination
/// keys (including `g`/`G`) are handled while one is showing, so the tab
/// underneath never accidentally reacts to a key meant for the modal.
pub fn handle_key(app: &mut App, key: KeyEvent, page_sizes: PageSizes) -> InputAction {
    if let KeyCode::Char('q') = key.code {
        return InputAction::Quit;
    }

    if app.modal.is_some() {
        return handle_modal_key(app, key, page_sizes);
    }

    match key.code {
        KeyCode::Esc => return InputAction::Quit,
        KeyCode::Tab => {
            app.cycle_tab();
            return InputAction::Continue;
        }
        KeyCode::Char('?') => {
            app.open_help_modal();
            return InputAction::Continue;
        }
        KeyCode::Char('A') | KeyCode::Char('a') => {
            app.set_tab(Tab::Agents);
            return InputAction::Continue;
        }
        KeyCode::Char('L') | KeyCode::Char('l') => {
            app.set_tab(Tab::Logs);
            return InputAction::Continue;
        }
        KeyCode::Char('R') | KeyCode::Char('r') => {
            app.set_tab(Tab::Reminders);
            return InputAction::Continue;
        }
        _ => {}
    }

    match app.active_tab {
        Tab::Agents => {
            handle_agents_tab_key(app, key.code);
            InputAction::Continue
        }
        Tab::Logs => {
            handle_logs_tab_key(app, key.code, page_sizes.logs_tab);
            InputAction::Continue
        }
        Tab::Reminders => handle_reminders_tab_key(app, key.code, page_sizes.reminders_tab),
    }
}

fn handle_agents_tab_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('j') | KeyCode::Down => app.move_agents_selection(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_agents_selection(-1),
        KeyCode::Enter => app.open_details_modal(),
        _ => {}
    }
}

fn handle_logs_tab_key(app: &mut App, code: KeyCode, page_size: usize) {
    match code {
        KeyCode::Char('j') | KeyCode::Down => app.move_logs_selection(1, page_size),
        KeyCode::Char('k') | KeyCode::Up => app.move_logs_selection(-1, page_size),
        KeyCode::Char('d') | KeyCode::PageDown => app.page_logs(1, page_size),
        KeyCode::Char('u') | KeyCode::PageUp => app.page_logs(-1, page_size),
        KeyCode::Char('g') => app.jump_logs_to_start(page_size),
        KeyCode::Char('G') => app.jump_logs_to_end(page_size),
        KeyCode::Char('o') => app.cycle_logs_sort(),
        KeyCode::Char('p') => app.cycle_logs_project_filter(),
        KeyCode::Char('s') => app.cycle_logs_status_filter(),
        KeyCode::Char('c') => app.clear_logs_filters(),
        KeyCode::Enter => app.open_details_modal(),
        _ => {}
    }
}

/// Handles a key on the Reminders tab. `s` starts/stops the highlighted
/// reminder (rather than cycling the status filter, which moves to `f`
/// here) - see the "Reminders tab supports sorting and filtering"
/// requirement's keybinding-conflict resolution.
fn handle_reminders_tab_key(app: &mut App, code: KeyCode, page_size: usize) -> InputAction {
    match code {
        KeyCode::Char('j') | KeyCode::Down => app.move_reminders_selection(1, page_size),
        KeyCode::Char('k') | KeyCode::Up => app.move_reminders_selection(-1, page_size),
        KeyCode::Char('d') | KeyCode::PageDown => app.page_reminders(1, page_size),
        KeyCode::Char('u') | KeyCode::PageUp => app.page_reminders(-1, page_size),
        KeyCode::Char('g') => app.jump_reminders_to_start(page_size),
        KeyCode::Char('G') => app.jump_reminders_to_end(page_size),
        KeyCode::Char('o') => app.cycle_reminders_sort(),
        KeyCode::Char('p') => app.cycle_reminders_project_filter(),
        KeyCode::Char('f') => app.cycle_reminders_status_filter(),
        KeyCode::Char('c') => app.clear_reminders_filters(),
        KeyCode::Enter => app.open_details_modal(),
        KeyCode::Char('s') => {
            if let Some(id) = app.selected_reminder_in_tab() {
                if let Some(message) = app.toggle_reminder_command(&id) {
                    return InputAction::SendReminderCommand(message);
                }
            }
        }
        _ => {}
    }
    InputAction::Continue
}

/// Handles a key while a modal is open. `Esc` closes whatever is topmost -
/// the reminder form, then the delete confirmation, then the modal itself -
/// so each layer peels back independently. A `Modal::Details` additionally
/// routes `j`/`k`/`d`/`u`/`g`/`G`/page-down/page-up to whichever of its Logs
/// or Reminders panes currently holds focus (`Tab` toggles which), per the
/// "Paginated lists support keyboard navigation" requirement's precedence
/// rule. `Modal::Help` has no list of its own, so no other key does anything
/// while it's open.
fn handle_modal_key(app: &mut App, key: KeyEvent, page_sizes: PageSizes) -> InputAction {
    let code = key.code;

    if app.reminder_form.is_some() {
        return handle_reminder_form_key(app, key);
    }
    if app.confirm_delete_reminder.is_some() {
        match code {
            KeyCode::Enter => {
                if let Some(message) = app.confirm_delete_reminder_command() {
                    return InputAction::SendReminderCommand(message);
                }
            }
            KeyCode::Esc => app.close_delete_reminder_confirm(),
            _ => {}
        }
        return InputAction::Continue;
    }

    if code == KeyCode::Esc {
        app.close_modal();
        return InputAction::Continue;
    }

    if matches!(app.modal, Some(Modal::Details(_))) {
        if code == KeyCode::Tab {
            app.toggle_modal_focus();
            return InputAction::Continue;
        }
        // Creating a reminder doesn't depend on a highlighted selection
        // (unlike start/stop/edit/delete), so `R` works regardless of which
        // pane currently holds focus - matching its always-visible `New
        // [R]` hint in the Reminders pane's title.
        if code == KeyCode::Char('R') {
            app.open_reminder_create_form();
            return InputAction::Continue;
        }

        match app.modal_focus {
            ModalFocus::Logs => match code {
                KeyCode::Char('j') | KeyCode::Down => app.move_modal_logs_selection(1, page_sizes.modal_logs),
                KeyCode::Char('k') | KeyCode::Up => app.move_modal_logs_selection(-1, page_sizes.modal_logs),
                KeyCode::Char('d') | KeyCode::PageDown => app.page_modal_logs(1, page_sizes.modal_logs),
                KeyCode::Char('u') | KeyCode::PageUp => app.page_modal_logs(-1, page_sizes.modal_logs),
                KeyCode::Char('g') => app.jump_modal_logs_to_start(page_sizes.modal_logs),
                KeyCode::Char('G') => app.jump_modal_logs_to_end(page_sizes.modal_logs),
                _ => {}
            },
            ModalFocus::Reminders => match code {
                KeyCode::Char('j') | KeyCode::Down => {
                    app.move_modal_reminders_selection(1, page_sizes.modal_reminders)
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    app.move_modal_reminders_selection(-1, page_sizes.modal_reminders)
                }
                KeyCode::Char('d') | KeyCode::PageDown => app.page_modal_reminders(1, page_sizes.modal_reminders),
                KeyCode::Char('u') | KeyCode::PageUp => {
                    app.page_modal_reminders(-1, page_sizes.modal_reminders)
                }
                KeyCode::Char('g') => app.jump_modal_reminders_to_start(page_sizes.modal_reminders),
                KeyCode::Char('G') => app.jump_modal_reminders_to_end(page_sizes.modal_reminders),
                KeyCode::Enter | KeyCode::Char('s') => {
                    if let Some(id) = app.selected_reminder_in_modal() {
                        if let Some(message) = app.toggle_reminder_command(&id) {
                            return InputAction::SendReminderCommand(message);
                        }
                    }
                }
                KeyCode::Char('e') => app.open_reminder_edit_form(),
                // On most Mac keyboards the key labeled "delete" sends a
                // Backspace control code, not a true forward-delete - `Fn` +
                // `Delete` (or a dedicated key on an external/PC keyboard)
                // is needed to produce `KeyCode::Delete`. `Backspace` has no
                // other meaning in this list view (only inside the open
                // form, a separate context), so both are bound here.
                KeyCode::Delete | KeyCode::Backspace => app.open_delete_reminder_confirm(),
                _ => {}
            },
        }
    }

    InputAction::Continue
}

/// Handles a key while the reminder creation/editing form is open. `Tab`
/// moves between the Name and Duration fields; `Enter` saves and closes the
/// form, returning the command to send; `Escape` discards the form without
/// sending anything; `Backspace` and printable characters edit whichever
/// field currently has focus.
fn handle_reminder_form_key(app: &mut App, key: KeyEvent) -> InputAction {
    match key.code {
        KeyCode::Esc => {
            app.close_reminder_form();
            InputAction::Continue
        }
        KeyCode::Enter => match app.submit_reminder_form() {
            Some(message) => InputAction::SendReminderCommand(message),
            None => InputAction::Continue,
        },
        KeyCode::Tab => {
            app.toggle_reminder_form_field();
            InputAction::Continue
        }
        KeyCode::Backspace => {
            app.reminder_form_backspace();
            InputAction::Continue
        }
        KeyCode::Char(c) => {
            app.reminder_form_input_char(c);
            InputAction::Continue
        }
        _ => InputAction::Continue,
    }
}

/// Whether `app.modal` is currently showing the help overlay - a small
/// readability helper for callers (and tests) that only care about that one
/// case.
pub fn is_help_modal_open(app: &App) -> bool {
    matches!(app.modal, Some(Modal::Help))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    /// A `PageSizes` big enough that none of the tests below that don't care
    /// about pagination windowing get clamped unexpectedly.
    fn page_sizes() -> PageSizes {
        PageSizes { logs_tab: 10, modal_logs: 10, reminders_tab: 10, modal_reminders: 10 }
    }

    fn press(app: &mut App, code: KeyCode) -> InputAction {
        handle_key(app, key(code), page_sizes())
    }

    #[test]
    fn q_quits() {
        let mut app = App::new();
        assert_eq!(press(&mut app, KeyCode::Char('q')), InputAction::Quit);
    }

    #[test]
    fn esc_quits_when_no_modal_is_open() {
        let mut app = App::new();
        assert_eq!(press(&mut app, KeyCode::Esc), InputAction::Quit);
    }

    #[test]
    fn unrecognized_keys_are_ignored() {
        let mut app = App::new();
        assert_eq!(press(&mut app, KeyCode::Char('x')), InputAction::Continue);
    }

    #[test]
    fn tab_cycles_between_agents_and_logs() {
        let mut app = App::new();
        press(&mut app, KeyCode::Tab);
        assert_eq!(app.active_tab, Tab::Logs);
        press(&mut app, KeyCode::Tab);
        assert_eq!(app.active_tab, Tab::Reminders);
        press(&mut app, KeyCode::Tab);
        assert_eq!(app.active_tab, Tab::Agents);
    }

    #[test]
    fn a_and_l_jump_directly_to_their_tabs() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('l'));
        assert_eq!(app.active_tab, Tab::Logs);
        press(&mut app, KeyCode::Char('a'));
        assert_eq!(app.active_tab, Tab::Agents);
    }

    #[test]
    fn question_mark_opens_help_modal() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('?'));
        assert!(is_help_modal_open(&app));
    }

    #[test]
    fn esc_closes_the_help_modal_instead_of_quitting() {
        let mut app = App::new();
        app.open_help_modal();

        let action = press(&mut app, KeyCode::Esc);

        assert_eq!(action, InputAction::Continue);
        assert_eq!(app.modal, None);
    }

    #[test]
    fn q_still_quits_while_a_modal_is_open() {
        let mut app = App::new();
        app.open_help_modal();

        assert_eq!(press(&mut app, KeyCode::Char('q')), InputAction::Quit);
    }

    #[test]
    fn non_esc_keys_are_ignored_while_the_help_modal_is_open() {
        let mut app = App::new();
        app.open_help_modal();

        press(&mut app, KeyCode::Tab);

        assert_eq!(app.active_tab, Tab::Agents, "tab switching must not leak through a modal");
        assert!(is_help_modal_open(&app), "the modal must stay open");
    }

    #[test]
    fn j_and_k_move_the_agents_selection_on_the_agents_tab() {
        use agentmon_proto::{AgentInfo, AgentStatus, HostContext, SessionId};
        use std::path::PathBuf;

        let mut app = App::new();
        app.apply_snapshot(
            vec![
                AgentInfo {
                    session_id: SessionId("a".to_string()),
                    cwd: PathBuf::from("/tmp/a"),
                    host_context: HostContext::Terminal,
                    pid: 1,
                    status: AgentStatus::Running,
                    last_updated_ms: 2_000,
                    status_since_ms: 0,
                    run_started_ms: 0,
                },
                AgentInfo {
                    session_id: SessionId("b".to_string()),
                    cwd: PathBuf::from("/tmp/b"),
                    host_context: HostContext::Terminal,
                    pid: 2,
                    status: AgentStatus::Running,
                    last_updated_ms: 1_000,
                    status_since_ms: 0,
                    run_started_ms: 0,
                },
            ],
            Vec::new(),
        );

        press(&mut app, KeyCode::Char('j'));
        assert_eq!(app.agents_selected, 1);
        press(&mut app, KeyCode::Char('k'));
        assert_eq!(app.agents_selected, 0);
    }

    #[test]
    fn enter_opens_the_details_modal_on_the_agents_tab() {
        use agentmon_proto::{AgentInfo, AgentStatus, HostContext, SessionId};
        use std::path::PathBuf;

        let mut app = App::new();
        app.apply_snapshot(
            vec![AgentInfo {
                session_id: SessionId("a".to_string()),
                cwd: PathBuf::from("/tmp/a"),
                host_context: HostContext::Terminal,
                pid: 1,
                status: AgentStatus::Running,
                last_updated_ms: 0,
                status_since_ms: 0,
                run_started_ms: 0,
            }],
            Vec::new(),
        );

        press(&mut app, KeyCode::Enter);

        assert_eq!(app.modal, Some(Modal::Details(PathBuf::from("/tmp/a"))));
    }

    #[test]
    fn enter_opens_the_details_modal_on_the_logs_tab() {
        use agentmon_proto::{LogCategory, LogEntry};
        use std::path::PathBuf;

        let mut app = App::new();
        app.set_tab(Tab::Logs);
        app.apply_log_snapshot(vec![LogEntry {
            working_dir: "/tmp/a".into(),
            category: LogCategory::Agent,
            status: "done".to_string(),
            occurred_at_ms: 1,
            pid: Some(1),
            reminder_name: None,
        }]);

        press(&mut app, KeyCode::Enter);

        assert_eq!(app.modal, Some(Modal::Details(PathBuf::from("/tmp/a"))));
    }

    #[test]
    fn logs_tab_keys_only_apply_when_the_logs_tab_is_active() {
        use agentmon_proto::{LogCategory, LogEntry};

        let mut app = App::new();
        app.apply_log_snapshot(vec![
            LogEntry {
                working_dir: "/tmp/a".into(),
                category: LogCategory::Agent,
                status: "done".to_string(),
                occurred_at_ms: 1,
                pid: Some(1),
                reminder_name: None,
            },
            LogEntry {
                working_dir: "/tmp/b".into(),
                category: LogCategory::Agent,
                status: "done".to_string(),
                occurred_at_ms: 2,
                pid: Some(1),
                reminder_name: None,
            },
        ]);

        // On the Agents tab, 'j' moves agent selection, not the log list.
        press(&mut app, KeyCode::Char('j'));
        assert_eq!(app.logs_pagination.selected, 0);

        app.set_tab(Tab::Logs);
        press(&mut app, KeyCode::Char('j'));
        assert_eq!(app.logs_pagination.selected, 1);
    }

    #[test]
    fn d_and_u_page_the_logs_list_with_a_1_row_overlap() {
        use agentmon_proto::{LogCategory, LogEntry};

        let page_size = 10;
        let mut app = App::new();
        app.set_tab(Tab::Logs);
        app.apply_log_snapshot(
            (0..(page_size * 2) as u64)
                .map(|i| LogEntry {
                    working_dir: "/tmp/a".into(),
                    category: LogCategory::Agent,
                    status: "done".to_string(),
                    occurred_at_ms: i,
                    pid: Some(1),
                    reminder_name: None,
                })
                .collect(),
        );

        handle_key(&mut app, key(KeyCode::Char('d')), page_sizes());
        assert_eq!(app.logs_pagination.selected, page_size - 1);
        handle_key(&mut app, key(KeyCode::Char('u')), page_sizes());
        assert_eq!(app.logs_pagination.selected, 0);
    }

    #[test]
    fn g_and_shift_g_jump_the_logs_list_to_its_first_and_last_entry() {
        use agentmon_proto::{LogCategory, LogEntry};

        let page_size = 10;
        let mut app = App::new();
        app.set_tab(Tab::Logs);
        app.apply_log_snapshot(
            (0..(page_size * 3) as u64)
                .map(|i| LogEntry {
                    working_dir: "/tmp/a".into(),
                    category: LogCategory::Agent,
                    status: "done".to_string(),
                    occurred_at_ms: i,
                    pid: Some(1),
                    reminder_name: None,
                })
                .collect(),
        );

        handle_key(&mut app, key(KeyCode::Char('G')), page_sizes());
        assert_eq!(app.logs_pagination.selected, page_size * 3 - 1);
        handle_key(&mut app, key(KeyCode::Char('g')), page_sizes());
        assert_eq!(app.logs_pagination.selected, 0);
    }

    #[test]
    fn o_cycles_sort_p_cycles_project_filter_s_cycles_status_filter_c_clears() {
        use agentmon_proto::{LogCategory, LogEntry};

        let mut app = App::new();
        app.set_tab(Tab::Logs);
        app.apply_log_snapshot(vec![LogEntry {
            working_dir: "/tmp/a".into(),
            category: LogCategory::Agent,
            status: "done".to_string(),
            occurred_at_ms: 1,
            pid: Some(1),
            reminder_name: None,
        }]);

        press(&mut app, KeyCode::Char('o'));
        assert_eq!(app.logs_sort, crate::app::LogSort::Project);

        press(&mut app, KeyCode::Char('p'));
        assert_eq!(app.logs_filter_project.as_deref(), Some("a"));

        press(&mut app, KeyCode::Char('s'));
        assert_eq!(app.logs_filter_status.as_deref(), Some("done"));

        press(&mut app, KeyCode::Char('c'));
        assert_eq!(app.logs_filter_project, None);
        assert_eq!(app.logs_filter_status, None);
    }

    #[test]
    fn the_details_modals_logs_pane_keys_move_its_own_selection() {
        use agentmon_proto::{AgentInfo, AgentStatus, HostContext, SessionId};
        use std::path::PathBuf;

        let mut app = App::new();
        app.apply_snapshot(
            vec![AgentInfo {
                session_id: SessionId("a".to_string()),
                cwd: PathBuf::from("/tmp/a"),
                host_context: HostContext::Terminal,
                pid: 1,
                status: AgentStatus::Running,
                last_updated_ms: 0,
                status_since_ms: 0,
                run_started_ms: 0,
            }],
            Vec::new(),
        );
        app.open_details_modal();

        press(&mut app, KeyCode::Char('j'));
        assert_eq!(app.modal_logs_pagination.selected, 0, "no log entries to move onto yet, but must not panic");

        press(&mut app, KeyCode::Esc);
        assert_eq!(app.modal, None);
    }

    #[test]
    fn the_details_modals_logs_pane_keys_do_not_leak_to_the_logs_tab_underneath() {
        use agentmon_proto::{LogCategory, LogEntry};

        let mut app = App::new();
        app.set_tab(Tab::Logs);
        app.apply_log_snapshot(
            (0..20u64)
                .map(|i| LogEntry {
                    working_dir: "/tmp/a".into(),
                    category: LogCategory::Agent,
                    status: "done".to_string(),
                    occurred_at_ms: i,
                    pid: Some(1),
                    reminder_name: None,
                })
                .collect(),
        );
        press(&mut app, KeyCode::Enter); // open the modal on the selected entry's project
        assert!(matches!(app.modal, Some(Modal::Details(_))));

        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('d'));

        assert_eq!(app.modal_logs_pagination.selected, 9, "the modal's own pane moved");
        assert_eq!(
            app.logs_pagination.selected, 0,
            "the Logs tab underneath must not react to keys handled by the modal"
        );
    }

    #[test]
    fn g_and_shift_g_move_the_modals_own_logs_pane_without_leaking_to_the_logs_tab() {
        use agentmon_proto::{LogCategory, LogEntry};

        let mut app = App::new();
        app.set_tab(Tab::Logs);
        app.apply_log_snapshot(
            (0..20u64)
                .map(|i| LogEntry {
                    working_dir: "/tmp/a".into(),
                    category: LogCategory::Agent,
                    status: "done".to_string(),
                    occurred_at_ms: i,
                    pid: Some(1),
                    reminder_name: None,
                })
                .collect(),
        );
        press(&mut app, KeyCode::Enter); // open the modal on the selected entry's project
        assert!(matches!(app.modal, Some(Modal::Details(_))));

        press(&mut app, KeyCode::Char('G'));
        assert_eq!(app.modal_logs_pagination.selected, 19, "the modal's own pane jumped to its last entry");
        assert_eq!(
            app.logs_pagination.selected, 0,
            "the Logs tab underneath must not react to keys handled by the modal"
        );

        press(&mut app, KeyCode::Char('g'));
        assert_eq!(app.modal_logs_pagination.selected, 0);
    }

    fn reminder(id: &str, cwd: &str, name: &str, status: agentmon_proto::ReminderStatus) -> agentmon_proto::ReminderInfo {
        agentmon_proto::ReminderInfo {
            id: agentmon_proto::ReminderId(id.to_string()),
            cwd: std::path::PathBuf::from(cwd),
            name: name.to_string(),
            duration_minutes: 10,
            status,
            created_at_ms: 0,
            run_started_ms: Some(0),
            last_updated_ms: 0,
        }
    }

    #[test]
    fn r_jumps_directly_to_the_reminders_tab() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('r'));
        assert_eq!(app.active_tab, Tab::Reminders);
        app.set_tab(Tab::Agents);
        press(&mut app, KeyCode::Char('R'));
        assert_eq!(app.active_tab, Tab::Reminders);
    }

    #[test]
    fn j_and_k_move_the_reminders_selection() {
        let mut app = App::new();
        app.set_tab(Tab::Reminders);
        app.apply_reminder_snapshot(vec![
            reminder("a", "/tmp/a", "A", agentmon_proto::ReminderStatus::NotYetStarted),
            reminder("b", "/tmp/b", "B", agentmon_proto::ReminderStatus::NotYetStarted),
        ]);

        press(&mut app, KeyCode::Char('j'));
        assert_eq!(app.reminders_pagination.selected, 1);
        press(&mut app, KeyCode::Char('k'));
        assert_eq!(app.reminders_pagination.selected, 0);
    }

    #[test]
    fn enter_opens_the_details_modal_on_the_reminders_tab() {
        let mut app = App::new();
        app.set_tab(Tab::Reminders);
        app.apply_reminder_snapshot(vec![reminder("a", "/tmp/a", "A", agentmon_proto::ReminderStatus::NotYetStarted)]);

        press(&mut app, KeyCode::Enter);

        assert_eq!(app.modal, Some(Modal::Details(std::path::PathBuf::from("/tmp/a"))));
    }

    #[test]
    fn s_starts_or_stops_the_highlighted_reminder_on_the_reminders_tab() {
        let mut app = App::new();
        app.set_tab(Tab::Reminders);
        app.apply_reminder_snapshot(vec![reminder("a", "/tmp/a", "A", agentmon_proto::ReminderStatus::NotYetStarted)]);

        let action = press(&mut app, KeyCode::Char('s'));

        assert_eq!(
            action,
            InputAction::SendReminderCommand(agentmon_proto::ClientMessage::StartReminder {
                id: agentmon_proto::ReminderId("a".to_string()),
            })
        );
    }

    #[test]
    fn s_does_not_cycle_the_status_filter_on_the_reminders_tab_unlike_the_logs_tab() {
        let mut app = App::new();
        app.set_tab(Tab::Reminders);
        app.apply_reminder_snapshot(vec![reminder("a", "/tmp/a", "A", agentmon_proto::ReminderStatus::NotYetStarted)]);

        press(&mut app, KeyCode::Char('s'));

        assert_eq!(app.reminders_filter_status, None, "s must not cycle the status filter on the Reminders tab");
    }

    #[test]
    fn f_cycles_the_reminders_status_filter() {
        let mut app = App::new();
        app.set_tab(Tab::Reminders);
        app.apply_reminder_snapshot(vec![reminder(
            "a",
            "/tmp/a",
            "A",
            agentmon_proto::ReminderStatus::NotYetStarted,
        )]);

        press(&mut app, KeyCode::Char('f'));

        assert_eq!(app.reminders_filter_status.as_deref(), Some("not_yet_started"));
    }

    #[test]
    fn o_cycles_sort_p_cycles_project_filter_c_clears_on_the_reminders_tab() {
        let mut app = App::new();
        app.set_tab(Tab::Reminders);
        app.apply_reminder_snapshot(vec![reminder(
            "a",
            "/tmp/a",
            "A",
            agentmon_proto::ReminderStatus::NotYetStarted,
        )]);

        press(&mut app, KeyCode::Char('o'));
        assert_eq!(app.reminders_sort, crate::app::ReminderSort::Project);

        press(&mut app, KeyCode::Char('p'));
        assert_eq!(app.reminders_filter_project.as_deref(), Some("a"));

        press(&mut app, KeyCode::Char('c'));
        assert_eq!(app.reminders_filter_project, None);
    }

    #[test]
    fn tab_toggles_modal_focus_between_logs_and_reminders() {
        use agentmon_proto::{AgentInfo, AgentStatus, HostContext, SessionId};
        use std::path::PathBuf;

        let mut app = App::new();
        app.apply_snapshot(
            vec![AgentInfo {
                session_id: SessionId("a".to_string()),
                cwd: PathBuf::from("/tmp/a"),
                host_context: HostContext::Terminal,
                pid: 1,
                status: AgentStatus::Running,
                last_updated_ms: 0,
                status_since_ms: 0,
                run_started_ms: 0,
            }],
            Vec::new(),
        );
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.modal_focus, crate::app::ModalFocus::Logs);

        press(&mut app, KeyCode::Tab);
        assert_eq!(app.modal_focus, crate::app::ModalFocus::Reminders);

        press(&mut app, KeyCode::Tab);
        assert_eq!(app.modal_focus, crate::app::ModalFocus::Logs);
    }

    #[test]
    fn reminders_pane_selection_dependent_keys_are_inert_when_the_logs_pane_holds_focus() {
        use agentmon_proto::{AgentInfo, AgentStatus, HostContext, SessionId};
        use std::path::PathBuf;

        let mut app = App::new();
        app.apply_snapshot(
            vec![AgentInfo {
                session_id: SessionId("a".to_string()),
                cwd: PathBuf::from("/tmp/a"),
                host_context: HostContext::Terminal,
                pid: 1,
                status: AgentStatus::Running,
                last_updated_ms: 0,
                status_since_ms: 0,
                run_started_ms: 0,
            }],
            Vec::new(),
        );
        app.apply_reminder_update(reminder("r1", "/tmp/a", "A", agentmon_proto::ReminderStatus::NotYetStarted));
        press(&mut app, KeyCode::Enter); // opens with Logs pane focused

        // e and Delete/Backspace act on the highlighted reminder, so they
        // require the Reminders pane to hold focus.
        press(&mut app, KeyCode::Char('e'));
        assert!(app.reminder_form.is_none(), "e must not open the form while the Logs pane holds focus");
        press(&mut app, KeyCode::Delete);
        assert!(app.confirm_delete_reminder.is_none(), "Delete must not open the delete confirm while the Logs pane holds focus");
        press(&mut app, KeyCode::Backspace);
        assert!(app.confirm_delete_reminder.is_none(), "Backspace must not open the delete confirm while the Logs pane holds focus");
    }

    #[test]
    fn r_opens_the_create_form_even_while_the_logs_pane_holds_focus() {
        // Regression test: creating a reminder doesn't depend on a
        // highlighted selection, so R must work as soon as the details
        // modal is open - not only after tabbing into the Reminders pane.
        use agentmon_proto::{AgentInfo, AgentStatus, HostContext, SessionId};
        use std::path::PathBuf;

        let mut app = App::new();
        app.apply_snapshot(
            vec![AgentInfo {
                session_id: SessionId("a".to_string()),
                cwd: PathBuf::from("/tmp/a"),
                host_context: HostContext::Terminal,
                pid: 1,
                status: AgentStatus::Running,
                last_updated_ms: 0,
                status_since_ms: 0,
                run_started_ms: 0,
            }],
            Vec::new(),
        );
        press(&mut app, KeyCode::Enter); // opens with Logs pane focused (the default)
        assert_eq!(app.modal_focus, crate::app::ModalFocus::Logs);

        press(&mut app, KeyCode::Char('R'));

        assert!(app.reminder_form.is_some(), "R should open the create form regardless of modal pane focus");
    }

    #[test]
    fn reminders_pane_s_starts_or_stops_when_focused() {
        use agentmon_proto::{AgentInfo, AgentStatus, HostContext, SessionId};
        use std::path::PathBuf;

        let mut app = App::new();
        app.apply_snapshot(
            vec![AgentInfo {
                session_id: SessionId("a".to_string()),
                cwd: PathBuf::from("/tmp/a"),
                host_context: HostContext::Terminal,
                pid: 1,
                status: AgentStatus::Running,
                last_updated_ms: 0,
                status_since_ms: 0,
                run_started_ms: 0,
            }],
            Vec::new(),
        );
        app.apply_reminder_update(reminder("r1", "/tmp/a", "A", agentmon_proto::ReminderStatus::NotYetStarted));
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Tab); // focus the Reminders pane

        let action = press(&mut app, KeyCode::Char('s'));

        assert_eq!(
            action,
            InputAction::SendReminderCommand(agentmon_proto::ClientMessage::StartReminder {
                id: agentmon_proto::ReminderId("r1".to_string()),
            })
        );
    }

    #[test]
    fn reminders_pane_r_opens_the_create_form_when_focused() {
        use agentmon_proto::{AgentInfo, AgentStatus, HostContext, SessionId};
        use std::path::PathBuf;

        let mut app = App::new();
        app.apply_snapshot(
            vec![AgentInfo {
                session_id: SessionId("a".to_string()),
                cwd: PathBuf::from("/tmp/a"),
                host_context: HostContext::Terminal,
                pid: 1,
                status: AgentStatus::Running,
                last_updated_ms: 0,
                status_since_ms: 0,
                run_started_ms: 0,
            }],
            Vec::new(),
        );
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Tab);

        press(&mut app, KeyCode::Char('R'));

        assert!(app.reminder_form.is_some());
    }

    #[test]
    fn reminders_pane_delete_key_opens_delete_confirm_when_focused() {
        use agentmon_proto::{AgentInfo, AgentStatus, HostContext, SessionId};
        use std::path::PathBuf;

        let mut app = App::new();
        app.apply_snapshot(
            vec![AgentInfo {
                session_id: SessionId("a".to_string()),
                cwd: PathBuf::from("/tmp/a"),
                host_context: HostContext::Terminal,
                pid: 1,
                status: AgentStatus::Running,
                last_updated_ms: 0,
                status_since_ms: 0,
                run_started_ms: 0,
            }],
            Vec::new(),
        );
        app.apply_reminder_update(reminder("r1", "/tmp/a", "A", agentmon_proto::ReminderStatus::NotYetStarted));
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Tab);

        press(&mut app, KeyCode::Delete);

        assert_eq!(app.confirm_delete_reminder, Some(agentmon_proto::ReminderId("r1".to_string())));
    }

    #[test]
    fn reminders_pane_backspace_also_opens_delete_confirm_when_focused() {
        // Regression test: on most Mac keyboards, the key labeled "delete"
        // sends a Backspace control code, not a true forward-delete - so
        // Backspace must trigger the same delete confirmation as the
        // Delete key while the Reminders pane holds focus.
        use agentmon_proto::{AgentInfo, AgentStatus, HostContext, SessionId};
        use std::path::PathBuf;

        let mut app = App::new();
        app.apply_snapshot(
            vec![AgentInfo {
                session_id: SessionId("a".to_string()),
                cwd: PathBuf::from("/tmp/a"),
                host_context: HostContext::Terminal,
                pid: 1,
                status: AgentStatus::Running,
                last_updated_ms: 0,
                status_since_ms: 0,
                run_started_ms: 0,
            }],
            Vec::new(),
        );
        app.apply_reminder_update(reminder("r1", "/tmp/a", "A", agentmon_proto::ReminderStatus::NotYetStarted));
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Tab);

        press(&mut app, KeyCode::Backspace);

        assert_eq!(app.confirm_delete_reminder, Some(agentmon_proto::ReminderId("r1".to_string())));
    }

    #[test]
    fn reminders_pane_d_pages_down_like_every_other_paginated_list() {
        use agentmon_proto::{AgentInfo, AgentStatus, HostContext, SessionId};
        use std::path::PathBuf;

        let mut app = App::new();
        app.apply_snapshot(
            vec![AgentInfo {
                session_id: SessionId("a".to_string()),
                cwd: PathBuf::from("/tmp/a"),
                host_context: HostContext::Terminal,
                pid: 1,
                status: AgentStatus::Running,
                last_updated_ms: 0,
                status_since_ms: 0,
                run_started_ms: 0,
            }],
            Vec::new(),
        );
        let page_size = 10;
        app.apply_reminder_snapshot(
            (0..(page_size * 2) as u64)
                .map(|i| reminder(&i.to_string(), "/tmp/a", "A", agentmon_proto::ReminderStatus::NotYetStarted))
                .collect(),
        );
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Tab);

        handle_key(&mut app, key(KeyCode::Char('d')), PageSizes { modal_reminders: page_size, ..page_sizes() });

        assert_eq!(app.modal_reminders_pagination.selected, page_size - 1);
        assert!(app.confirm_delete_reminder.is_none(), "d must not open the delete confirm here");
    }

    #[test]
    fn escape_dismisses_the_delete_confirm_without_deleting_and_leaves_the_modal_open() {
        use agentmon_proto::{AgentInfo, AgentStatus, HostContext, SessionId};
        use std::path::PathBuf;

        let mut app = App::new();
        app.apply_snapshot(
            vec![AgentInfo {
                session_id: SessionId("a".to_string()),
                cwd: PathBuf::from("/tmp/a"),
                host_context: HostContext::Terminal,
                pid: 1,
                status: AgentStatus::Running,
                last_updated_ms: 0,
                status_since_ms: 0,
                run_started_ms: 0,
            }],
            Vec::new(),
        );
        app.apply_reminder_update(reminder("r1", "/tmp/a", "A", agentmon_proto::ReminderStatus::NotYetStarted));
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Tab);
        press(&mut app, KeyCode::Delete);

        press(&mut app, KeyCode::Esc);

        assert!(app.confirm_delete_reminder.is_none());
        assert!(app.modal.is_some(), "Esc on the confirm dialog must not also close the modal");
    }

    #[test]
    fn enter_confirms_deletion() {
        use agentmon_proto::{AgentInfo, AgentStatus, HostContext, SessionId};
        use std::path::PathBuf;

        let mut app = App::new();
        app.apply_snapshot(
            vec![AgentInfo {
                session_id: SessionId("a".to_string()),
                cwd: PathBuf::from("/tmp/a"),
                host_context: HostContext::Terminal,
                pid: 1,
                status: AgentStatus::Running,
                last_updated_ms: 0,
                status_since_ms: 0,
                run_started_ms: 0,
            }],
            Vec::new(),
        );
        app.apply_reminder_update(reminder("r1", "/tmp/a", "A", agentmon_proto::ReminderStatus::NotYetStarted));
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Tab);
        press(&mut app, KeyCode::Delete);

        let action = press(&mut app, KeyCode::Enter);

        assert_eq!(
            action,
            InputAction::SendReminderCommand(agentmon_proto::ClientMessage::DeleteReminder {
                id: agentmon_proto::ReminderId("r1".to_string()),
            })
        );
    }

    #[test]
    fn reminder_form_tab_switches_fields_and_typing_edits_them() {
        use agentmon_proto::{AgentInfo, AgentStatus, HostContext, SessionId};
        use std::path::PathBuf;

        let mut app = App::new();
        app.apply_snapshot(
            vec![AgentInfo {
                session_id: SessionId("a".to_string()),
                cwd: PathBuf::from("/tmp/a"),
                host_context: HostContext::Terminal,
                pid: 1,
                status: AgentStatus::Running,
                last_updated_ms: 0,
                status_since_ms: 0,
                run_started_ms: 0,
            }],
            Vec::new(),
        );
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Tab);
        press(&mut app, KeyCode::Char('R'));

        press(&mut app, KeyCode::Char('X'));
        press(&mut app, KeyCode::Tab);
        press(&mut app, KeyCode::Char('7'));

        let form = app.reminder_form.as_ref().unwrap();
        assert_eq!(form.name, "X");
        assert_eq!(form.duration_minutes, "7");
    }

    #[test]
    fn reminder_form_enter_submits_and_sends_a_create_command() {
        use agentmon_proto::{AgentInfo, AgentStatus, HostContext, SessionId};
        use std::path::PathBuf;

        let mut app = App::new();
        app.apply_snapshot(
            vec![AgentInfo {
                session_id: SessionId("a".to_string()),
                cwd: PathBuf::from("/tmp/a"),
                host_context: HostContext::Terminal,
                pid: 1,
                status: AgentStatus::Running,
                last_updated_ms: 0,
                status_since_ms: 0,
                run_started_ms: 0,
            }],
            Vec::new(),
        );
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Tab);
        press(&mut app, KeyCode::Char('R'));
        press(&mut app, KeyCode::Char('X'));

        let action = press(&mut app, KeyCode::Enter);

        assert_eq!(
            action,
            InputAction::SendReminderCommand(agentmon_proto::ClientMessage::CreateReminder {
                cwd: PathBuf::from("/tmp/a"),
                name: "X".to_string(),
                duration_minutes: 0,
            })
        );
        assert!(app.reminder_form.is_none());
    }

    #[test]
    fn reminder_form_escape_discards_without_sending_and_leaves_the_modal_open() {
        use agentmon_proto::{AgentInfo, AgentStatus, HostContext, SessionId};
        use std::path::PathBuf;

        let mut app = App::new();
        app.apply_snapshot(
            vec![AgentInfo {
                session_id: SessionId("a".to_string()),
                cwd: PathBuf::from("/tmp/a"),
                host_context: HostContext::Terminal,
                pid: 1,
                status: AgentStatus::Running,
                last_updated_ms: 0,
                status_since_ms: 0,
                run_started_ms: 0,
            }],
            Vec::new(),
        );
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Tab);
        press(&mut app, KeyCode::Char('R'));
        press(&mut app, KeyCode::Char('X'));

        let action = press(&mut app, KeyCode::Esc);

        assert_eq!(action, InputAction::Continue);
        assert!(app.reminder_form.is_none());
        assert!(app.modal.is_some(), "Esc on the form must not also close the modal");
    }
}
