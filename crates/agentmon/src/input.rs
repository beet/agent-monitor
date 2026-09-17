use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, Modal, PageSizes, Tab};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAction {
    Continue,
    Quit,
}

/// Maps a key event to an action, applying it to `app` along the way.
/// Terminal-independent, so it can be tested with synthetic `KeyEvent`s and
/// without a real terminal. `page_sizes` carries however many rows each
/// paginated list actually rendered on the most recent frame - see
/// `PageSizes`.
///
/// A modal, if open, takes priority over tab-level keys: only `Esc` (close),
/// `q` (quit), and - for `Modal::Details` - its own Logs pane's pagination
/// keys are handled while one is showing, so the tab underneath never
/// accidentally reacts to a key meant for the modal.
pub fn handle_key(app: &mut App, key: KeyEvent, page_sizes: PageSizes) -> InputAction {
    if let KeyCode::Char('q') = key.code {
        return InputAction::Quit;
    }

    if app.modal.is_some() {
        handle_modal_key(app, key.code, page_sizes.modal_logs);
        return InputAction::Continue;
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
        _ => {}
    }

    match app.active_tab {
        Tab::Agents => handle_agents_tab_key(app, key.code),
        Tab::Logs => handle_logs_tab_key(app, key.code, page_sizes.logs_tab),
    }

    InputAction::Continue
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
        KeyCode::Char('o') => app.cycle_logs_sort(),
        KeyCode::Char('p') => app.cycle_logs_project_filter(),
        KeyCode::Char('s') => app.cycle_logs_status_filter(),
        KeyCode::Char('c') => app.clear_logs_filters(),
        KeyCode::Enter => app.open_details_modal(),
        _ => {}
    }
}

/// Handles a key while a modal is open. `Esc` closes any modal; a
/// `Modal::Details` additionally routes its own Logs pane's `j`/`k`/`d`/`u`/
/// page-down/page-up to that pane alone, scoped to the modal - per the
/// "Paginated lists support keyboard navigation" requirement's precedence
/// rule. `Modal::Help` has no list of its own, so no other key does
/// anything while it's open.
fn handle_modal_key(app: &mut App, code: KeyCode, modal_logs_page_size: usize) {
    if code == KeyCode::Esc {
        app.close_modal();
        return;
    }
    if matches!(app.modal, Some(Modal::Details(_))) {
        match code {
            KeyCode::Char('j') | KeyCode::Down => app.move_modal_logs_selection(1, modal_logs_page_size),
            KeyCode::Char('k') | KeyCode::Up => app.move_modal_logs_selection(-1, modal_logs_page_size),
            KeyCode::Char('d') | KeyCode::PageDown => app.page_modal_logs(1, modal_logs_page_size),
            KeyCode::Char('u') | KeyCode::PageUp => app.page_modal_logs(-1, modal_logs_page_size),
            _ => {}
        }
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
        PageSizes { logs_tab: 10, modal_logs: 10 }
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
            },
            LogEntry {
                working_dir: "/tmp/b".into(),
                category: LogCategory::Agent,
                status: "done".to_string(),
                occurred_at_ms: 2,
                pid: Some(1),
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
                })
                .collect(),
        );

        handle_key(&mut app, key(KeyCode::Char('d')), page_sizes());
        assert_eq!(app.logs_pagination.selected, page_size - 1);
        handle_key(&mut app, key(KeyCode::Char('u')), page_sizes());
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
}
