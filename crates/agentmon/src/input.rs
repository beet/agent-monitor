use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAction {
    Continue,
    Quit,
}

/// Applies a key event to `app`. Pure and terminal-independent, so it can be
/// tested with synthetic `KeyEvent`s and without a real terminal.
pub fn handle_key(app: &mut App, key: KeyEvent) -> InputAction {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => InputAction::Quit,
        KeyCode::Down | KeyCode::Char('j') => {
            app.select_next();
            InputAction::Continue
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.select_previous();
            InputAction::Continue
        }
        _ => InputAction::Continue,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentmon_proto::{AgentStatus, HostContext, SessionId};
    use crossterm::event::KeyModifiers;
    use std::path::PathBuf;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn app_with_agents(n: usize) -> App {
        let mut app = App::new();
        let agents = (0..n)
            .map(|i| agentmon_proto::AgentInfo {
                session_id: SessionId(i.to_string()),
                cwd: PathBuf::from("/tmp/project"),
                host_context: HostContext::Terminal,
                pid: 1,
                status: AgentStatus::Running,
                last_updated_ms: 0,
            })
            .collect();
        app.apply_snapshot(agents);
        app
    }

    #[test]
    fn q_quits() {
        let mut app = App::new();
        assert_eq!(handle_key(&mut app, key(KeyCode::Char('q'))), InputAction::Quit);
    }

    #[test]
    fn esc_quits() {
        let mut app = App::new();
        assert_eq!(handle_key(&mut app, key(KeyCode::Esc)), InputAction::Quit);
    }

    #[test]
    fn down_and_j_select_the_next_row() {
        let mut app = app_with_agents(3);

        assert_eq!(handle_key(&mut app, key(KeyCode::Down)), InputAction::Continue);
        assert_eq!(app.selected, 1);

        assert_eq!(handle_key(&mut app, key(KeyCode::Char('j'))), InputAction::Continue);
        assert_eq!(app.selected, 2);
    }

    #[test]
    fn up_and_k_select_the_previous_row() {
        let mut app = app_with_agents(3);
        app.select_next();
        app.select_next();

        assert_eq!(handle_key(&mut app, key(KeyCode::Up)), InputAction::Continue);
        assert_eq!(app.selected, 1);

        assert_eq!(handle_key(&mut app, key(KeyCode::Char('k'))), InputAction::Continue);
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn unrecognized_keys_are_ignored() {
        let mut app = app_with_agents(2);

        assert_eq!(handle_key(&mut app, key(KeyCode::Char('x'))), InputAction::Continue);
        assert_eq!(app.selected, 0);
    }
}
