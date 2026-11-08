## 1. Row rendering split into styled spans

- [x] 1.1 Change `log_entry_status_line` in `crates/agentmon/src/ui.rs` to return the label/duration text and style separately from an optional quoted-snippet text and style, instead of one concatenated `(String, Style)` - verify with `cargo test -p agentmon` after updating its existing unit tests (around line 5045 onward) to read the new return shape
- [x] 1.2 Update `log_entry_line_with_pid` to thread the split pieces through (pid suffix stays attached to the label, not the snippet) - verify its own existing unit tests still pass with the new shape
- [x] 1.3 Update the Logs tab's row-building closure (`render_logs_tab`, around `ui.rs:373-381`) and the details modal's Logs pane row-building (around `ui.rs:917`) to build each `Cell` from a `Line` of multiple `Span`s (label span + optional snippet span) instead of a single `Span::styled` - verified via the existing and new `buffer_text`-based render tests (e.g. `details_modal_logs_pane_shows_the_prompt_snippet_for_agent_activities`, `logs_tab_shows_no_inline_snippet_for_a_started_entry_with_a_recorded_prompt`), which confirm each row still renders as one line with the label and snippet adjacent

## 2. Remove "started" entries' inline snippet

- [x] 2.1 In `log_entry_status_line`, stop sourcing a snippet for an agent-category entry whose status is "started" (keep it for "needs input" and "done" exactly as before) - verified with `a_started_entry_shows_no_inline_snippet_regardless_of_a_recorded_prompt`, asserting the returned snippet piece is `None` regardless of a recorded last-prompt snapshot
- [x] 2.2 Added `logs_tab_shows_no_inline_snippet_for_a_started_entry_with_a_recorded_prompt` and `details_modal_logs_pane_shows_no_inline_snippet_for_a_started_entry_but_still_shows_its_pid`, confirming a "started" entry's row shows its status (and, in the modal pane, its pid) with no trailing snippet text

## 3. "Done" entry snippet color

- [x] 3.1 In `log_entry_status_line`, style the quoted-snippet piece (response, or its last-prompt fallback) for a "done" agent-category entry with the same unstyled `Style::new()` already used for a prompt-category entry's snippet, rather than the status (green) style - verified with `a_done_entrys_snippet_renders_in_the_user_prompt_color_not_the_status_color` and `a_done_entrys_fallback_prompt_snippet_also_renders_in_the_user_prompt_color`, asserting the returned snippet style has no `fg` override while the label style still does
- [x] 3.2 Confirmed via the style-level unit tests above (ratatui's `TestBackend` buffer does not capture color, so an automated render-buffer assertion can't check this directly) that a "done" row's label style keeps `Color::Green` while its snippet style is bare `Style::new()`

## 4. "Started" entry opens the full-text modal on Enter

- [x] 4.1 In `crates/agentmon/src/input.rs`'s `handle_modal_key` (`ModalFocus::Logs` match), extended the `Enter` branch's entry match with `LogCategory::Agent if entry.status == "started" => entry.prompt.clone()`, mirroring the "done" branch - verified with `enter_opens_the_full_prompt_modal_for_a_started_entry_with_a_last_prompt_snapshot`
- [x] 4.2 Added `enter_on_a_started_entry_with_no_last_prompt_snapshot_has_no_effect`, confirming `Enter` on a "started" entry recorded with no last-prompt snapshot leaves `app.prompt_detail` as `None`
- [x] 4.3 Added `enter_on_a_started_entry_in_the_top_level_logs_tab_opens_the_details_modal_not_the_full_text_modal`, confirming the top-level Logs tab's `Enter` on a "started" entry still opens the project details modal rather than the full-text modal

## 5. Spec and changelog hygiene

- [x] 5.1 Ran `openspec validate --strict tweak-log-entry-presentation` - passes
- [x] 5.2 Ran the full `agentmon` test suite (`cargo test -p agentmon`) - 389 passed, 0 failed
