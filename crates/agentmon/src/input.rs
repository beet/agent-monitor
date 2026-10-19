use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tui_input::backend::crossterm::EventHandler;

use agentmon_proto::ClientMessage;

use crate::app::{App, Modal, ModalFocus, PageSizes, ReminderFormField, Tab};

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
/// underneath never accidentally reacts to a key meant for the modal. At the
/// top level (no modal open), `Esc` opens `Modal::QuitConfirm` rather than
/// quitting directly - only `q` quits immediately there. The reminder form,
/// though reached through a modal, is checked ahead of all of that - see the
/// `reminder_form` check below.
pub fn handle_key(app: &mut App, key: KeyEvent, page_sizes: PageSizes) -> InputAction {
    // Diverts all input into the Agents tab's `/`-search prompt while it's
    // being edited - checked before the global `q` quit shortcut (and
    // everything else) so a typed `q`, `?`, `A`, `L`, `R`, or `Tab` becomes
    // part of the search string instead of firing its usual global effect.
    if app.agents_search_editing {
        return handle_agents_search_editing_key(app, key);
    }

    // Also checked before the global `q` quit shortcut, for the same reason
    // as `agents_search_editing` above: a project name is free text the user
    // types, and "q" is a plausible character to want in one.
    if app.rename_form.is_some() {
        return handle_rename_form_key(app, key);
    }

    // Also checked before the global `q` quit shortcut, for the same reason
    // as `agents_search_editing` and `rename_form` above: a reminder's name
    // is free text the user types, and "q" is a plausible character to want
    // in one. The reminder form is otherwise reached through `handle_modal_key`
    // (it's only ever open alongside `Modal::Details`), but it must be
    // checked here, ahead of the quit guard, rather than there.
    if app.reminder_form.is_some() {
        return handle_reminder_form_key(app, key);
    }

    if let KeyCode::Char('q') = key.code {
        return InputAction::Quit;
    }

    if app.modal.is_some() {
        return handle_modal_key(app, key, page_sizes);
    }

    match key.code {
        KeyCode::Esc => {
            app.open_quit_confirm();
            return InputAction::Continue;
        }
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
        KeyCode::Char('/') => app.open_agents_search(),
        KeyCode::Char('n') => app.open_rename_form_from_agents_tab(),
        _ => {}
    }
}

/// Handles a key while the project rename form is open. `Esc` discards the
/// form without sending anything; `Enter` validates and, on success, returns
/// the `RenameProject` message to send (closing the form), or leaves it open
/// with its inline error set on failure. Every other key (including `q` -
/// see this form's check in `handle_key`) is forwarded to the form's
/// `tui_input::Input`, which owns the text's cursor and handles movement,
/// insertion, and deletion - see design.md's "Key handling forwards to
/// `Input::handle_event`" decision. The inline error is cleared only when
/// that edit actually changed the text, not on pure cursor movement.
fn handle_rename_form_key(app: &mut App, key: KeyEvent) -> InputAction {
    match key.code {
        KeyCode::Esc => {
            app.close_rename_form();
            InputAction::Continue
        }
        KeyCode::Enter => match app.submit_rename_form() {
            Some(message) => InputAction::SendReminderCommand(message),
            None => InputAction::Continue,
        },
        _ => {
            if let Some(form) = &mut app.rename_form {
                let changed = form.name.handle_event(&Event::Key(key)).is_some_and(|r| r.value);
                if changed {
                    form.error = None;
                }
            }
            InputAction::Continue
        }
    }
}

/// Handles a key while the Agents tab's `/`-search prompt is being edited -
/// see "Agents tab supports incremental search by project name" and
/// "Committing, cancelling, and clearing the Agents tab search". `Enter`
/// commits the search and `Esc` cancels it; every other key is forwarded to
/// the search buffer's `tui_input::Input` - see `handle_rename_form_key`'s
/// doc comment.
fn handle_agents_search_editing_key(app: &mut App, key: KeyEvent) -> InputAction {
    match key.code {
        KeyCode::Enter => app.commit_agents_search(),
        KeyCode::Esc => app.cancel_agents_search(),
        _ => {
            app.agents_search_buffer.handle_event(&Event::Key(key));
        }
    }
    InputAction::Continue
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

/// Handles a key while a modal is open. The reminder form - though only ever
/// open alongside `Modal::Details` - is intercepted earlier, in `handle_key`,
/// so it never reaches here. Of what's left, `Esc` closes whatever is
/// topmost (the delete confirmation, then the modal itself), so each layer
/// peels back independently. A `Modal::Details` additionally routes
/// `j`/`k`/`d`/`u`/`g`/`G`/page-down/page-up to whichever of its Logs or
/// Reminders panes currently holds focus (`Tab` toggles which), per the
/// "Paginated lists support keyboard navigation" requirement's precedence
/// rule. `Modal::Help` has no list of its own, so no other key does anything
/// while it's open.
fn handle_modal_key(app: &mut App, key: KeyEvent, page_sizes: PageSizes) -> InputAction {
    let code = key.code;

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

    if app.modal == Some(Modal::QuitConfirm) {
        match code {
            KeyCode::Enter => return InputAction::Quit,
            KeyCode::Esc => app.close_modal(),
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
        // Renaming likewise doesn't depend on a highlighted selection, so
        // it works regardless of which pane currently holds focus - matching
        // its `Rename [n]` hint in the modal's title.
        if code == KeyCode::Char('n') {
            app.open_rename_form_from_details_modal();
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

/// Whether a key reaching the reminder form's Duration field should be
/// forwarded to its `tui_input::Input`. `tui-input`'s own crossterm mapping
/// treats a bare `Ctrl+<letter>` (without `Alt`) as a line-editing command -
/// `Ctrl+B`/`F`/`A`/`E`/`W`/`D`/`U`/`K`/`H` - not a literal character, so
/// those always pass through regardless of field. Everything else that
/// reaches here as a `Char` is literal text input (unmodified, `Shift`, or
/// `Ctrl+Alt`), so the Duration field's existing digits-only rule applies to
/// it, same as before this field delegated to `tui_input`.
fn should_forward_to_duration_field(key: &KeyEvent) -> bool {
    match key.code {
        KeyCode::Char(c) => {
            let bare_ctrl = key.modifiers.contains(KeyModifiers::CONTROL) && !key.modifiers.contains(KeyModifiers::ALT);
            bare_ctrl || c.is_ascii_digit()
        }
        _ => true,
    }
}

/// Handles a key while the reminder creation/editing form is open. `Tab`
/// moves between the Name and Duration fields, resetting the cursor to the
/// newly-focused field's end; `Enter` saves and closes the form, returning
/// the command to send; `Escape` discards the form without sending
/// anything. Every other key is forwarded to whichever field currently has
/// focus's `tui_input::Input` - see `handle_rename_form_key`'s doc comment -
/// except a non-digit character key on the Duration field, which is ignored
/// per `should_forward_to_duration_field`.
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
        _ => {
            if let Some(form) = &mut app.reminder_form {
                match form.field {
                    ReminderFormField::Name => {
                        form.name.handle_event(&Event::Key(key));
                    }
                    ReminderFormField::Duration => {
                        if should_forward_to_duration_field(&key) {
                            form.duration_minutes.handle_event(&Event::Key(key));
                        }
                    }
                }
            }
            InputAction::Continue
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
        PageSizes { logs_tab: 10, modal_logs: 10, reminders_tab: 10, modal_reminders: 10 }
    }

    fn press(app: &mut App, code: KeyCode) -> InputAction {
        handle_key(app, key(code), page_sizes())
    }

    /// Presses `code` with `Ctrl` held - for exercising `tui-input`'s
    /// bare-`Ctrl` line-editing bindings (`Ctrl+Left`/`Right`/`W`/`K`/`U`).
    fn press_ctrl(app: &mut App, code: KeyCode) -> InputAction {
        handle_key(app, KeyEvent::new(code, KeyModifiers::CONTROL), page_sizes())
    }

    #[test]
    fn q_quits() {
        let mut app = App::new();
        assert_eq!(press(&mut app, KeyCode::Char('q')), InputAction::Quit);
    }

    #[test]
    fn esc_opens_quit_confirm_when_no_modal_is_open() {
        let mut app = App::new();

        let action = press(&mut app, KeyCode::Esc);

        assert_eq!(action, InputAction::Continue);
        assert_eq!(app.modal, Some(Modal::QuitConfirm));
    }

    #[test]
    fn enter_confirms_quit_from_the_quit_confirm_dialog() {
        let mut app = App::new();
        app.open_quit_confirm();

        assert_eq!(press(&mut app, KeyCode::Enter), InputAction::Quit);
    }

    #[test]
    fn esc_cancels_the_quit_confirm_dialog() {
        let mut app = App::new();
        app.open_quit_confirm();

        let action = press(&mut app, KeyCode::Esc);

        assert_eq!(action, InputAction::Continue);
        assert_eq!(app.modal, None);
    }

    #[test]
    fn q_still_quits_while_the_quit_confirm_dialog_is_open() {
        let mut app = App::new();
        app.open_quit_confirm();

        assert_eq!(press(&mut app, KeyCode::Char('q')), InputAction::Quit);
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
                    last_prompt: None,
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
                    last_prompt: None,
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
    fn slash_enters_agents_search_editing_mode() {
        let mut app = App::new();

        press(&mut app, KeyCode::Char('/'));

        assert!(app.agents_search_editing);
    }

    #[test]
    fn while_editing_agents_search_global_shortcut_keys_are_typed_instead_of_firing() {
        let mut app = App::new();
        app.open_agents_search();

        for code in [
            KeyCode::Char('q'),
            KeyCode::Char('?'),
            KeyCode::Char('A'),
            KeyCode::Char('L'),
            KeyCode::Char('R'),
        ] {
            let action = press(&mut app, code);
            assert_eq!(action, InputAction::Continue, "must not fire the global effect for {code:?}");
        }
        press(&mut app, KeyCode::Tab);

        assert!(app.agents_search_editing, "must still be editing - none of these should have exited edit mode");
        assert!(!is_help_modal_open(&app), "? must not open the help modal while editing");
        assert_eq!(app.active_tab, Tab::Agents, "A/L/R/Tab must not switch tabs while editing");
        assert_eq!(app.agents_search_buffer.value(), "q?ALR", "every key typed above must land in the search buffer");
    }

    #[test]
    fn enter_commits_the_agents_search_and_filters_the_list() {
        use agentmon_proto::{AgentInfo, AgentStatus, HostContext, SessionId};
        use std::path::PathBuf;

        let mut app = App::new();
        app.apply_snapshot(
            vec![
                AgentInfo {
                    session_id: SessionId("a".to_string()),
                    cwd: PathBuf::from("/tmp/LAB-1234"),
                    host_context: HostContext::Terminal,
                    pid: 1,
                    status: AgentStatus::Running,
                    last_updated_ms: 0,
                    status_since_ms: 0,
                    run_started_ms: 0,
                    last_prompt: None,
                },
                AgentInfo {
                    session_id: SessionId("b".to_string()),
                    cwd: PathBuf::from("/tmp/other"),
                    host_context: HostContext::Terminal,
                    pid: 2,
                    status: AgentStatus::Running,
                    last_updated_ms: 0,
                    status_since_ms: 0,
                    run_started_ms: 0,
                    last_prompt: None,
                },
            ],
            Vec::new(),
        );

        press(&mut app, KeyCode::Char('/'));
        press(&mut app, KeyCode::Char('1'));
        press(&mut app, KeyCode::Char('2'));
        press(&mut app, KeyCode::Char('3'));
        press(&mut app, KeyCode::Char('4'));
        press(&mut app, KeyCode::Enter);

        assert!(!app.agents_search_editing);
        assert_eq!(app.agents_search_applied.as_deref(), Some("1234"));
        assert_eq!(app.visible_agent_groups().len(), 1);
        assert_eq!(app.visible_agent_groups()[0].cwd, PathBuf::from("/tmp/LAB-1234"));
    }

    #[test]
    fn escape_cancels_editing_and_restores_the_prior_applied_filter() {
        let mut app = App::new();
        app.agents_search_applied = Some("kept".to_string());

        press(&mut app, KeyCode::Char('/'));
        press(&mut app, KeyCode::Char('x'));
        press(&mut app, KeyCode::Esc);

        assert!(!app.agents_search_editing);
        assert_eq!(app.agents_search_applied.as_deref(), Some("kept"));
    }

    #[test]
    fn escape_cancels_editing_with_no_prior_filter() {
        let mut app = App::new();

        press(&mut app, KeyCode::Char('/'));
        press(&mut app, KeyCode::Char('x'));
        press(&mut app, KeyCode::Esc);

        assert!(!app.agents_search_editing);
        assert_eq!(app.agents_search_applied, None);
    }

    #[test]
    fn backspace_removes_the_last_character_while_editing() {
        let mut app = App::new();

        press(&mut app, KeyCode::Char('/'));
        press(&mut app, KeyCode::Char('a'));
        press(&mut app, KeyCode::Char('b'));
        press(&mut app, KeyCode::Backspace);

        assert_eq!(app.agents_search_buffer.value(), "a");
    }

    #[test]
    fn re_editing_an_applied_filter_starts_from_its_text() {
        let mut app = App::new();
        app.agents_search_applied = Some("prior".to_string());

        press(&mut app, KeyCode::Char('/'));

        assert_eq!(app.agents_search_buffer.value(), "prior");
        assert_eq!(app.agents_search_buffer.cursor(), 5, "cursor should seed at the end of the filter's text");
    }

    #[test]
    fn left_and_right_move_the_search_cursor_without_changing_the_text() {
        let mut app = app_with_one_agent();
        press(&mut app, KeyCode::Char('/'));
        for c in "xay".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        assert_eq!(app.agents_search_buffer.cursor(), 3);

        press(&mut app, KeyCode::Left);
        assert_eq!(app.agents_search_buffer.cursor(), 2);

        press(&mut app, KeyCode::Right);
        assert_eq!(app.agents_search_buffer.cursor(), 3);
        assert_eq!(app.agents_search_buffer.value(), "xay");
    }

    #[test]
    fn search_cursor_cannot_move_past_either_end_of_the_string() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('/'));
        press(&mut app, KeyCode::Char('a'));

        press(&mut app, KeyCode::Left);
        press(&mut app, KeyCode::Left);
        assert_eq!(app.agents_search_buffer.cursor(), 0, "must clamp at the start");

        press(&mut app, KeyCode::Right);
        press(&mut app, KeyCode::Right);
        assert_eq!(app.agents_search_buffer.cursor(), 1, "must clamp at the end");
    }

    #[test]
    fn home_and_end_jump_the_search_cursor_to_either_end() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('/'));
        for c in "abc".chars() {
            press(&mut app, KeyCode::Char(c));
        }

        press(&mut app, KeyCode::Home);
        assert_eq!(app.agents_search_buffer.cursor(), 0);

        press(&mut app, KeyCode::End);
        assert_eq!(app.agents_search_buffer.cursor(), 3);
    }

    #[test]
    fn ctrl_left_and_ctrl_right_move_the_search_cursor_by_word() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('/'));
        for c in "foo bar".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        let end = app.agents_search_buffer.cursor();

        press_ctrl(&mut app, KeyCode::Left);
        let after_left = app.agents_search_buffer.cursor();
        assert!(after_left < end, "Ctrl+Left should move the cursor backward by a word");
        assert_eq!(app.agents_search_buffer.value(), "foo bar", "word movement must not change the text");

        press_ctrl(&mut app, KeyCode::Right);
        assert_eq!(app.agents_search_buffer.cursor(), end, "Ctrl+Right should return to the end");
    }

    #[test]
    fn typing_mid_string_inserts_at_the_cursor_and_updates_matches() {
        let mut app = app_with_one_agent();
        press(&mut app, KeyCode::Char('/'));
        for c in "ay".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        press(&mut app, KeyCode::Left);

        press(&mut app, KeyCode::Char('x'));

        assert_eq!(app.agents_search_buffer.value(), "axy");
        assert_eq!(app.agents_search_buffer.cursor(), 2);
    }

    #[test]
    fn backspace_mid_string_deletes_the_character_before_the_cursor() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('/'));
        for c in "axy".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        press(&mut app, KeyCode::Left);

        press(&mut app, KeyCode::Backspace);

        assert_eq!(app.agents_search_buffer.value(), "ay");
        assert_eq!(app.agents_search_buffer.cursor(), 1);
    }

    /// An `App` tracking a single agent for project `/tmp/foobar` - used by
    /// search-filter tests that need a project name long enough for a
    /// meaningful substring-match check (unlike `app_with_one_agent`'s
    /// single-character "a").
    fn app_with_foobar_project() -> App {
        use agentmon_proto::{AgentInfo, AgentStatus, HostContext, SessionId};
        use std::path::PathBuf;

        let mut app = App::new();
        app.apply_snapshot(
            vec![AgentInfo {
                session_id: SessionId("a".to_string()),
                cwd: PathBuf::from("/tmp/foobar"),
                host_context: HostContext::Terminal,
                pid: 1,
                status: AgentStatus::Running,
                last_updated_ms: 0,
                status_since_ms: 0,
                run_started_ms: 0,
                last_prompt: None,
            }],
            Vec::new(),
        );
        app
    }

    #[test]
    fn delete_removes_the_character_after_the_search_cursor_and_updates_matches() {
        let mut app = app_with_foobar_project();
        press(&mut app, KeyCode::Char('/'));
        for c in "fooXbar".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        for _ in 0..4 {
            press(&mut app, KeyCode::Left);
        }
        assert_eq!(app.agents_search_buffer.cursor(), 3, "cursor should sit right before the stray \"X\"");
        assert_eq!(app.visible_agent_groups().len(), 0, "\"fooXbar\" must not match project \"foobar\"");

        press(&mut app, KeyCode::Delete);

        assert_eq!(app.agents_search_buffer.value(), "foobar");
        assert_eq!(app.agents_search_buffer.cursor(), 3, "Delete must not move the cursor");
        assert_eq!(app.visible_agent_groups().len(), 1, "\"foobar\" matches project \"foobar\" again");
    }

    #[test]
    fn ctrl_w_removes_the_word_before_the_search_cursor_and_updates_matches() {
        let mut app = app_with_foobar_project();
        press(&mut app, KeyCode::Char('/'));
        for c in "extra foobar".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        assert_eq!(app.visible_agent_groups().len(), 0, "\"extra foobar\" must not match project \"foobar\"");
        press(&mut app, KeyCode::Home);
        for _ in 0..6 {
            press(&mut app, KeyCode::Right);
        }
        assert_eq!(app.agents_search_buffer.cursor(), 6, "cursor should sit right before \"foobar\"");

        press_ctrl(&mut app, KeyCode::Char('w'));

        assert_eq!(app.agents_search_buffer.value().trim(), "foobar");
        assert_eq!(app.visible_agent_groups().len(), 1, "the filter should match project \"foobar\" again");
    }

    #[test]
    fn ctrl_k_clears_from_the_search_cursor_to_the_end() {
        let mut app = app_with_one_agent();
        press(&mut app, KeyCode::Char('/'));
        for c in "aXY".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        press(&mut app, KeyCode::Left);
        press(&mut app, KeyCode::Left);

        press_ctrl(&mut app, KeyCode::Char('k'));

        assert_eq!(app.agents_search_buffer.value(), "a");
        assert_eq!(app.visible_agent_groups().len(), 1);
    }

    #[test]
    fn ctrl_u_clears_the_search_string() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('/'));
        for c in "anything".chars() {
            press(&mut app, KeyCode::Char(c));
        }

        press_ctrl(&mut app, KeyCode::Char('u'));

        assert_eq!(app.agents_search_buffer.value(), "");
        assert_eq!(app.agents_search_buffer.cursor(), 0);
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
                last_prompt: None,
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
            reminder_due_at_ms: None,
            prompt: None,
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
                reminder_due_at_ms: None,
                prompt: None,
            },
            LogEntry {
                working_dir: "/tmp/b".into(),
                category: LogCategory::Agent,
                status: "done".to_string(),
                occurred_at_ms: 2,
                pid: Some(1),
                reminder_name: None,
                reminder_due_at_ms: None,
                prompt: None,
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
                    reminder_due_at_ms: None,
                    prompt: None,
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
                    reminder_due_at_ms: None,
                    prompt: None,
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
            reminder_due_at_ms: None,
            prompt: None,
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
                last_prompt: None,
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
                    reminder_due_at_ms: None,
                    prompt: None,
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
                    reminder_due_at_ms: None,
                    prompt: None,
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
                last_prompt: None,
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
    fn tab_never_focuses_the_non_interactive_last_prompt_pane() {
        // `ModalFocus` only has `Logs`/`Reminders` variants, so the Last
        // Prompt pane (like the Agents/Tests panes) is structurally
        // unreachable via `Tab` - this exercises several toggles to guard
        // against a future third variant being added without updating the
        // cycle.
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
                last_prompt: Some("Fix the flaky test".to_string()),
            }],
            Vec::new(),
        );
        press(&mut app, KeyCode::Enter);

        for _ in 0..5 {
            press(&mut app, KeyCode::Tab);
            assert!(
                matches!(app.modal_focus, crate::app::ModalFocus::Logs | crate::app::ModalFocus::Reminders),
                "Tab must only ever land on Logs or Reminders"
            );
        }
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
                last_prompt: None,
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
                last_prompt: None,
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
                last_prompt: None,
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
                last_prompt: None,
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
                last_prompt: None,
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
                last_prompt: None,
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
                last_prompt: None,
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
                last_prompt: None,
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
                last_prompt: None,
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
                last_prompt: None,
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
        assert_eq!(form.name.value(), "X");
        assert_eq!(form.duration_minutes.value(), "7");
    }

    /// Opens the reminder creation form (Name field focused) for a single
    /// tracked project - shared setup for the cursor/line-editing tests
    /// below.
    fn app_with_reminder_form_open() -> App {
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
                last_prompt: None,
            }],
            Vec::new(),
        );
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Tab);
        press(&mut app, KeyCode::Char('R'));
        app
    }

    #[test]
    fn reminder_form_non_digit_key_leaves_the_duration_field_unchanged() {
        let mut app = app_with_reminder_form_open();
        press(&mut app, KeyCode::Tab); // focus Duration

        press(&mut app, KeyCode::Char('x'));

        let form = app.reminder_form.as_ref().unwrap();
        assert_eq!(form.duration_minutes.value(), "");
        assert_eq!(form.duration_minutes.cursor(), 0);
    }

    #[test]
    fn reminder_form_name_field_supports_left_right_home_end_and_delete() {
        let mut app = app_with_reminder_form_open();
        for c in "fooXbar".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        for _ in 0..4 {
            press(&mut app, KeyCode::Left);
        }
        assert_eq!(app.reminder_form.as_ref().unwrap().name.cursor(), 3, "cursor should sit right before the stray \"X\"");

        press(&mut app, KeyCode::Delete);
        assert_eq!(app.reminder_form.as_ref().unwrap().name.value(), "foobar");

        press(&mut app, KeyCode::Home);
        assert_eq!(app.reminder_form.as_ref().unwrap().name.cursor(), 0);

        press(&mut app, KeyCode::Right);
        press(&mut app, KeyCode::Right);
        assert_eq!(app.reminder_form.as_ref().unwrap().name.cursor(), 2);

        press(&mut app, KeyCode::End);
        assert_eq!(app.reminder_form.as_ref().unwrap().name.cursor(), 6);
    }

    #[test]
    fn reminder_form_name_field_ctrl_w_removes_the_word_before_the_cursor() {
        let mut app = app_with_reminder_form_open();
        for c in "extra foobar".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        press(&mut app, KeyCode::Home);
        for _ in 0..6 {
            press(&mut app, KeyCode::Right);
        }

        press_ctrl(&mut app, KeyCode::Char('w'));

        assert_eq!(app.reminder_form.as_ref().unwrap().name.value().trim(), "foobar");
    }

    #[test]
    fn reminder_form_name_field_ctrl_k_clears_from_the_cursor_to_the_end() {
        let mut app = app_with_reminder_form_open();
        for c in "foobar".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        press(&mut app, KeyCode::Home);
        press(&mut app, KeyCode::Right);
        press(&mut app, KeyCode::Right);
        press(&mut app, KeyCode::Right);

        press_ctrl(&mut app, KeyCode::Char('k'));

        assert_eq!(app.reminder_form.as_ref().unwrap().name.value(), "foo");
    }

    #[test]
    fn reminder_form_name_field_ctrl_u_clears_the_field() {
        let mut app = app_with_reminder_form_open();
        for c in "anything".chars() {
            press(&mut app, KeyCode::Char(c));
        }

        press_ctrl(&mut app, KeyCode::Char('u'));

        let form = app.reminder_form.as_ref().unwrap();
        assert_eq!(form.name.value(), "");
        assert_eq!(form.name.cursor(), 0);
    }

    #[test]
    fn reminder_form_duration_field_supports_cursor_movement_and_deletion() {
        let mut app = app_with_reminder_form_open();
        press(&mut app, KeyCode::Tab); // focus Duration
        for c in "123".chars() {
            press(&mut app, KeyCode::Char(c));
        }

        press(&mut app, KeyCode::Left);
        assert_eq!(app.reminder_form.as_ref().unwrap().duration_minutes.cursor(), 2);

        press(&mut app, KeyCode::Backspace);
        assert_eq!(app.reminder_form.as_ref().unwrap().duration_minutes.value(), "13");

        press(&mut app, KeyCode::Delete);
        assert_eq!(app.reminder_form.as_ref().unwrap().duration_minutes.value(), "1");

        press_ctrl(&mut app, KeyCode::Char('u'));
        assert_eq!(app.reminder_form.as_ref().unwrap().duration_minutes.value(), "");
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
                last_prompt: None,
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
                last_prompt: None,
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

    #[test]
    fn typing_q_while_the_reminder_form_is_open_appends_it_instead_of_quitting() {
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
                last_prompt: None,
            }],
            Vec::new(),
        );
        press(&mut app, KeyCode::Enter); // open details modal
        press(&mut app, KeyCode::Char('R')); // open the reminder creation form, Name field focused

        let action = press(&mut app, KeyCode::Char('q'));

        assert_eq!(action, InputAction::Continue, "q must not quit while the reminder form is open");
        assert_eq!(app.reminder_form.as_ref().unwrap().name.value(), "q");
    }

    #[test]
    fn q_still_quits_with_no_free_text_input_open() {
        // Guards against the reminder-form fix over-widening the exemption:
        // with no reminder form, rename form, or search prompt open, q must
        // still quit immediately, including while some other modal is open.
        let mut app = App::new();
        assert_eq!(press(&mut app, KeyCode::Char('q')), InputAction::Quit);

        let mut app = App::new();
        app.open_help_modal();
        assert_eq!(press(&mut app, KeyCode::Char('q')), InputAction::Quit);
    }

    fn app_with_one_agent() -> App {
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
                last_prompt: None,
            }],
            Vec::new(),
        );
        app
    }

    #[test]
    fn n_opens_the_rename_form_from_the_agents_tab() {
        let mut app = app_with_one_agent();

        let action = press(&mut app, KeyCode::Char('n'));

        assert_eq!(action, InputAction::Continue);
        assert_eq!(app.rename_form.as_ref().unwrap().name.value(), "a");
    }

    #[test]
    fn typing_q_while_the_rename_form_is_open_appends_it_instead_of_quitting() {
        let mut app = app_with_one_agent();
        press(&mut app, KeyCode::Char('n'));

        let action = press(&mut app, KeyCode::Char('q'));

        assert_eq!(action, InputAction::Continue, "q must not quit while the rename form is open");
        assert_eq!(app.rename_form.as_ref().unwrap().name.value(), "aq");
    }

    #[test]
    fn typing_q_mid_string_inserts_it_at_the_cursor_instead_of_quitting() {
        let mut app = app_with_one_agent();
        press(&mut app, KeyCode::Char('n'));
        press(&mut app, KeyCode::Char('x')); // pre-populated "a" -> "ax"
        press(&mut app, KeyCode::Left); // cursor between "a" and "x"

        let action = press(&mut app, KeyCode::Char('q'));

        assert_eq!(action, InputAction::Continue, "q must not quit while the rename form is open");
        assert_eq!(app.rename_form.as_ref().unwrap().name.value(), "aqx");
    }

    #[test]
    fn rename_form_supports_left_right_home_end_and_delete() {
        let mut app = app_with_one_agent();
        press(&mut app, KeyCode::Char('n')); // pre-populated with "a"
        press(&mut app, KeyCode::Backspace); // clear it
        for c in "fooXbar".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        for _ in 0..4 {
            press(&mut app, KeyCode::Left);
        }
        assert_eq!(app.rename_form.as_ref().unwrap().name.cursor(), 3, "cursor should sit right before the stray \"X\"");

        press(&mut app, KeyCode::Delete);
        assert_eq!(app.rename_form.as_ref().unwrap().name.value(), "foobar");

        press(&mut app, KeyCode::Home);
        assert_eq!(app.rename_form.as_ref().unwrap().name.cursor(), 0);

        press(&mut app, KeyCode::Right);
        press(&mut app, KeyCode::Right);
        assert_eq!(app.rename_form.as_ref().unwrap().name.cursor(), 2);

        press(&mut app, KeyCode::End);
        assert_eq!(app.rename_form.as_ref().unwrap().name.cursor(), 6);
    }

    #[test]
    fn rename_form_ctrl_left_and_ctrl_right_move_the_cursor_by_word() {
        let mut app = app_with_one_agent();
        press(&mut app, KeyCode::Char('n'));
        press(&mut app, KeyCode::Backspace);
        for c in "foo bar".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        let end = app.rename_form.as_ref().unwrap().name.cursor();

        press_ctrl(&mut app, KeyCode::Left);
        let after_left = app.rename_form.as_ref().unwrap().name.cursor();
        assert!(after_left < end, "Ctrl+Left should move the cursor backward by a word");
        assert_eq!(app.rename_form.as_ref().unwrap().name.value(), "foo bar");

        press_ctrl(&mut app, KeyCode::Right);
        assert_eq!(app.rename_form.as_ref().unwrap().name.cursor(), end, "Ctrl+Right should return to the end");
    }

    #[test]
    fn rename_form_ctrl_w_removes_the_word_before_the_cursor() {
        let mut app = app_with_one_agent();
        press(&mut app, KeyCode::Char('n'));
        press(&mut app, KeyCode::Backspace);
        for c in "extra foobar".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        press(&mut app, KeyCode::Home);
        for _ in 0..6 {
            press(&mut app, KeyCode::Right);
        }

        press_ctrl(&mut app, KeyCode::Char('w'));

        assert_eq!(app.rename_form.as_ref().unwrap().name.value().trim(), "foobar");
    }

    #[test]
    fn rename_form_ctrl_k_clears_from_the_cursor_to_the_end() {
        let mut app = app_with_one_agent();
        press(&mut app, KeyCode::Char('n'));
        press(&mut app, KeyCode::Backspace);
        for c in "foobar".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        press(&mut app, KeyCode::Home);
        press(&mut app, KeyCode::Right);
        press(&mut app, KeyCode::Right);
        press(&mut app, KeyCode::Right);

        press_ctrl(&mut app, KeyCode::Char('k'));

        assert_eq!(app.rename_form.as_ref().unwrap().name.value(), "foo");
    }

    #[test]
    fn rename_form_ctrl_u_clears_the_field() {
        let mut app = app_with_one_agent();
        press(&mut app, KeyCode::Char('n'));

        press_ctrl(&mut app, KeyCode::Char('u'));

        let form = app.rename_form.as_ref().unwrap();
        assert_eq!(form.name.value(), "");
        assert_eq!(form.name.cursor(), 0);
    }

    #[test]
    fn esc_cancels_the_rename_form_without_a_message() {
        let mut app = app_with_one_agent();
        press(&mut app, KeyCode::Char('n'));

        let action = press(&mut app, KeyCode::Esc);

        assert_eq!(action, InputAction::Continue);
        assert!(app.rename_form.is_none());
        assert_eq!(app.project_name(std::path::Path::new("/tmp/a")), "a");
    }

    #[test]
    fn enter_submits_a_valid_rename_from_the_agents_tab() {
        let mut app = app_with_one_agent();
        press(&mut app, KeyCode::Char('n'));
        press(&mut app, KeyCode::Char('!'));

        let action = press(&mut app, KeyCode::Enter);

        assert_eq!(
            action,
            InputAction::SendReminderCommand(ClientMessage::RenameProject {
                cwd: std::path::PathBuf::from("/tmp/a"),
                name: "a!".to_string(),
            })
        );
        assert!(app.rename_form.is_none());
    }

    #[test]
    fn enter_on_a_blank_rename_keeps_the_form_open_with_an_error() {
        let mut app = app_with_one_agent();
        press(&mut app, KeyCode::Char('n'));
        press(&mut app, KeyCode::Backspace);

        let action = press(&mut app, KeyCode::Enter);

        assert_eq!(action, InputAction::Continue);
        let form = app.rename_form.expect("form must stay open on invalid input");
        assert!(form.error.is_some());
    }

    #[test]
    fn n_opens_the_rename_form_from_the_details_modal_regardless_of_focus() {
        let mut app = app_with_one_agent();
        press(&mut app, KeyCode::Enter); // open details modal, default focus Logs

        press(&mut app, KeyCode::Char('n'));
        assert_eq!(app.rename_form.as_ref().unwrap().cwd, std::path::PathBuf::from("/tmp/a"));
        app.close_rename_form();

        press(&mut app, KeyCode::Tab); // switch focus to Reminders
        assert_eq!(app.modal_focus, ModalFocus::Reminders);
        let action = press(&mut app, KeyCode::Char('n'));

        assert_eq!(action, InputAction::Continue);
        assert_eq!(app.rename_form.as_ref().unwrap().cwd, std::path::PathBuf::from("/tmp/a"));
    }
}
