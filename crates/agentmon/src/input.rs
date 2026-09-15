use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, Modal, Tab};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAction {
    Continue,
    Quit,
}

/// Maps a key event to an action, applying it to `app` along the way.
/// Terminal-independent, so it can be tested with synthetic `KeyEvent`s and
/// without a real terminal.
///
/// A modal, if open, takes priority over tab-level keys: only `Esc` (close)
/// and `q` (quit) are handled while one is showing, so modal content is
/// never accidentally scrolled or filtered by a key meant for the tab
/// underneath.
pub fn handle_key(app: &mut App, key: KeyEvent) -> InputAction {
    if let KeyCode::Char('q') = key.code {
        return InputAction::Quit;
    }

    if app.modal.is_some() {
        if key.code == KeyCode::Esc {
            app.close_modal();
        }
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
        Tab::Logs => handle_logs_tab_key(app, key.code),
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

fn handle_logs_tab_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('j') | KeyCode::Down => app.move_logs_selection(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_logs_selection(-1),
        KeyCode::Char('d') | KeyCode::PageDown => app.page_logs(1),
        KeyCode::Char('u') | KeyCode::PageUp => app.page_logs(-1),
        KeyCode::Char('o') => app.cycle_logs_sort(),
        KeyCode::Char('p') => app.cycle_logs_project_filter(),
        KeyCode::Char('s') => app.cycle_logs_status_filter(),
        KeyCode::Char('c') => app.clear_logs_filters(),
        KeyCode::Enter => app.open_details_modal(),
        _ => {}
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

    #[test]
    fn q_quits() {
        let mut app = App::new();
        assert_eq!(handle_key(&mut app, key(KeyCode::Char('q'))), InputAction::Quit);
    }

    #[test]
    fn esc_quits_when_no_modal_is_open() {
        let mut app = App::new();
        assert_eq!(handle_key(&mut app, key(KeyCode::Esc)), InputAction::Quit);
    }

    #[test]
    fn unrecognized_keys_are_ignored() {
        let mut app = App::new();
        assert_eq!(handle_key(&mut app, key(KeyCode::Char('x'))), InputAction::Continue);
    }

    #[test]
    fn tab_cycles_between_agents_and_logs() {
        let mut app = App::new();
        handle_key(&mut app, key(KeyCode::Tab));
        assert_eq!(app.active_tab, Tab::Logs);
        handle_key(&mut app, key(KeyCode::Tab));
        assert_eq!(app.active_tab, Tab::Agents);
    }

    #[test]
    fn a_and_l_jump_directly_to_their_tabs() {
        let mut app = App::new();
        handle_key(&mut app, key(KeyCode::Char('l')));
        assert_eq!(app.active_tab, Tab::Logs);
        handle_key(&mut app, key(KeyCode::Char('a')));
        assert_eq!(app.active_tab, Tab::Agents);
    }

    #[test]
    fn question_mark_opens_help_modal() {
        let mut app = App::new();
        handle_key(&mut app, key(KeyCode::Char('?')));
        assert!(is_help_modal_open(&app));
    }

    #[test]
    fn esc_closes_the_help_modal_instead_of_quitting() {
        let mut app = App::new();
        app.open_help_modal();

        let action = handle_key(&mut app, key(KeyCode::Esc));

        assert_eq!(action, InputAction::Continue);
        assert_eq!(app.modal, None);
    }

    #[test]
    fn q_still_quits_while_a_modal_is_open() {
        let mut app = App::new();
        app.open_help_modal();

        assert_eq!(handle_key(&mut app, key(KeyCode::Char('q'))), InputAction::Quit);
    }

    #[test]
    fn non_esc_keys_are_ignored_while_a_modal_is_open() {
        let mut app = App::new();
        app.open_help_modal();

        handle_key(&mut app, key(KeyCode::Tab));

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

        handle_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.agents_selected, 1);
        handle_key(&mut app, key(KeyCode::Char('k')));
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

        handle_key(&mut app, key(KeyCode::Enter));

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

        handle_key(&mut app, key(KeyCode::Enter));

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
        handle_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.logs_selected, 0);

        app.set_tab(Tab::Logs);
        handle_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.logs_selected, 1);
    }

    #[test]
    fn d_and_u_page_the_logs_list() {
        use agentmon_proto::{LogCategory, LogEntry};
        use crate::app::LOGS_PAGE_SIZE;

        let mut app = App::new();
        app.set_tab(Tab::Logs);
        app.apply_log_snapshot(
            (0..(LOGS_PAGE_SIZE * 2) as u64)
                .map(|i| LogEntry {
                    working_dir: "/tmp/a".into(),
                    category: LogCategory::Agent,
                    status: "done".to_string(),
                    occurred_at_ms: i,
                    pid: Some(1),
                })
                .collect(),
        );

        handle_key(&mut app, key(KeyCode::Char('d')));
        assert_eq!(app.logs_selected, LOGS_PAGE_SIZE);
        handle_key(&mut app, key(KeyCode::Char('u')));
        assert_eq!(app.logs_selected, 0);
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

        handle_key(&mut app, key(KeyCode::Char('o')));
        assert_eq!(app.logs_sort, crate::app::LogSort::Project);

        handle_key(&mut app, key(KeyCode::Char('p')));
        assert_eq!(app.logs_filter_project.as_deref(), Some("a"));

        handle_key(&mut app, key(KeyCode::Char('s')));
        assert_eq!(app.logs_filter_status.as_deref(), Some("done"));

        handle_key(&mut app, key(KeyCode::Char('c')));
        assert_eq!(app.logs_filter_project, None);
        assert_eq!(app.logs_filter_status, None);
    }
}
