use crossterm::event::{KeyCode, KeyEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAction {
    Continue,
    Quit,
}

/// Maps a key event to an action. Pure and terminal-independent, so it can
/// be tested with synthetic `KeyEvent`s and without a real terminal.
pub fn handle_key(key: KeyEvent) -> InputAction {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => InputAction::Quit,
        _ => InputAction::Continue,
    }
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
        assert_eq!(handle_key(key(KeyCode::Char('q'))), InputAction::Quit);
    }

    #[test]
    fn esc_quits() {
        assert_eq!(handle_key(key(KeyCode::Esc)), InputAction::Quit);
    }

    #[test]
    fn unrecognized_keys_are_ignored() {
        assert_eq!(handle_key(key(KeyCode::Char('x'))), InputAction::Continue);
    }
}
