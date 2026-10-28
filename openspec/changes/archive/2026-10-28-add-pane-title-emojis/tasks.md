## 1. Pane title updates

- [x] 1.1 Prefix the Agents pane title with `🤖` in `render_agents_pane` (`crates/agentmon/src/ui.rs`) and verify it renders as `🤖 Agents [a]`
- [x] 1.2 Prefix the Tests pane title with `🧪` in `render_tests_pane` and verify it renders as `🧪 Tests [t]`
- [x] 1.3 Prefix the Reminders pane title with `⏰` in `render_reminders_pane` and verify it renders as `⏰ Reminders [r]  |  New [R]`
- [x] 1.4 Prefix the Last Prompt pane title with `🗣️` in `render_last_prompt_pane` and verify it renders as `🗣️ Last Prompt [p]`
- [x] 1.5 Prefix the Logs pane's `logs_title` with `🪵` (both the paginated and non-paginated variants) and verify it renders as `🪵 Logs [l]` or `🪵 Logs [l]  |  Page [d/u]  Top/Bottom [g/G]`

## 2. Test updates

- [x] 2.1 Update existing `ui.rs` tests that assert on exact pane title text (e.g. `find_text_from(buffer, "Agents", ...)`, `text.contains("Last Prompt")`, the pagination-hint tests around line 3417 and 416-418 of the spec) to expect the new emoji-prefixed titles, and verify `cargo test -p agentmon` passes - no changes were needed: all existing assertions use substring/positional matching (`.contains`, `find_text_from`) against the pane name alone, which still matches with the emoji prefix in place
- [x] 2.2 Add or extend a test asserting all five pane titles show their emoji prefix simultaneously when the details modal is open, and verify it passes - added `details_modal_pane_titles_are_each_prefixed_with_a_category_emoji`

## 3. Verification

- [x] 3.1 Run `cargo test` for the full workspace and verify it passes - all suites pass (368 passed in agentmon's lib)
- [x] 3.2 Run `cargo clippy` and verify no new warnings are introduced - the 2 remaining warnings (agentd doc-comment indentation, an unrelated pre-existing unused `buffer` in ui.rs) are pre-existing, confirmed via `git stash`

## 4. Top tab bar

- [x] 4.1 Prefix the tab bar's three labels in `render_tab_bar` (`crates/agentmon/src/ui.rs`) with `🤖`/`🪵`/`⏰` and verify they render as `🤖 Agents [a]`, `🪵 Logs [l]`, `⏰ Reminders [r]`

## 5. Main-view pane/table titles

- [x] 5.1 Prefix `agents_tab_title`'s "Agents" text with `🤖` in all three of its branches (idle, live search-editing, applied-filter) and verify each still shows its existing hints (`Filter [/]`, `Rename [n]`, the applied filter text, or the live search prompt) unchanged after the prefix
- [x] 5.2 Prefix the Logs tab's heading in `render_logs_tab` with `🪵` and verify it renders as `🪵 Logs - <controls>`
- [x] 5.3 Prefix the Reminders tab's heading in `render_reminders_tab` with `⏰` and verify it renders as `⏰ Reminders - <controls>`

## 6. Table headings

- [x] 6.1 Prefix the Agents tab's `AGENTS`, `TESTS`, and `REMINDERS` column headers in `render_agent_table` with `🤖`, `🧪`, and `⏰` respectively; leave `PROJECT` and `UPDATED` unprefixed, and verify the header renders as `PROJECT | 🤖 AGENTS | 🧪 TESTS | ⏰ REMINDERS | UPDATED`

## 7. Test updates

- [x] 7.1 Update existing `ui.rs` tests that assert on the tab bar's, the three tab titles', or the Agents tab header's exact text, and verify `cargo test -p agentmon` passes - no changes needed to existing assertions (all substring/positional, still match); extended `tab_bar_shows_shortcut_hints_and_the_title` with emoji checks
- [x] 7.2 Add or extend a test asserting the tab bar's three labels, each tab's own title, and the Agents tab's three recolored headers all show their emoji prefix, and verify it passes - added `the_agents_tab_header_columns_are_each_prefixed_with_a_category_emoji` and `each_tabs_own_title_is_prefixed_with_a_category_emoji`

## 8. Verification

- [x] 8.1 Run `cargo test` for the full workspace and verify it passes
- [x] 8.2 Run `cargo clippy` and verify no new warnings are introduced
