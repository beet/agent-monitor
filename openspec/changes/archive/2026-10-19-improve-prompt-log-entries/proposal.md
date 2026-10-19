## Why

The Logs tab's "prompt submitted" entries are noisier and less reliable than they should be: the prompt text itself (the one thing worth reading on that row) is buried behind a redundant `🗣️ prompt submitted — "..."` prefix, a user can't see a truncated prompt's full text without leaving the Logs tab, and the entry can render out of order relative to the "agent started" entry it precipitates - the daemon currently applies hook-reported events on a separate, unsynchronized thread per connection, so two events for the same new agent (e.g. `UserPromptSubmit` and a closely-following `PreToolUse`) are not guaranteed to be processed in the order Claude Code actually fired their hooks.

## What Changes

- **Daemon processes events in arrival order.** `agentd`'s connection handler currently calls `Ingestor::ingest_event` directly from each connection's own thread (`crates/agentd/src/server.rs::handle_connection`), with no ordering guarantee across concurrent connections. Route every `ReportEvent` through a single serialized worker so the daemon applies them in the order their connections were accepted, closing the one verified mechanism by which a "started" entry could be recorded ahead of the "prompt" entry that precipitated it. (The existing same-event tie-breaking logic - shared timestamp, stable sort - is already correct and unaffected; see design.md.)
- **Prompt log entries drop their redundant prefix and quoting.** A prompt-category entry's Logs-tab/modal-pane status text no longer reads `🗣️ prompt submitted — "<snippet>"`; it reads `🗣️ <snippet>` directly - the category is already shown by its own column (Logs tab) or was already identified solely by its emoji in practice, and the single always-"submitted" status word added no information.
- **Enter on any prompt entry in the details modal's Logs pane opens its full text.** Selecting a prompt-category log entry and pressing `Enter`, in the details modal's own Logs pane, opens a new read-only modal showing that entry's full prompt text (scrollable if it overflows the modal) - consistently, whether or not its displayed snippet happened to be truncated, so the user never has to judge a row's length to know what `Enter` will do. `Enter` on a prompt entry in the top-level Logs tab is unchanged - it always opens the project details modal, the same as every other log entry.
- **The details modal's Logs pane always keeps a row selectable.** Previously the pane showed no highlighted selection at all when its entries fit on a single page (no pagination needed), so there was nothing for `Enter` to act on in the common case of a project with only a handful of log entries. It now always shows a selected row whenever it has at least one entry and holds focus; the scrollbar and pagination hint in its title remain hidden when pagination isn't needed, unchanged.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `agent-daemon`: the local socket ingestion requirement gains an in-order-processing guarantee for events reported over concurrently-accepted connections.
- `agent-monitor-tui`: the prompt-category log entry's rendered text (Logs tab and details-modal Logs pane) drops its category/status prefix and quoting; `Enter` on any prompt-category entry in the details modal's Logs pane opens a new full-prompt modal, regardless of truncation; the details modal's Logs pane always shows a selected row when it has entries and holds focus, not only once pagination is needed.

## Impact

- `crates/agentd/src/server.rs` - connection handling gains a serialized event-processing path (e.g. a channel feeding a single worker) ahead of calling into `Ingestor`.
- `crates/agentmon/src/ui.rs` - prompt-category status-line rendering (`log_status_cell_text_and_style`, `log_entry_status_line`); a new render function for the full-prompt modal; `render_details_modal`'s Logs-pane selection is no longer conditioned on needing pagination, only on holding focus and having at least one entry.
- `crates/agentmon/src/app.rs` - new modal/overlay state for the full-prompt view, plus a selector for "the currently selected log entry" in the details modal's Logs pane.
- `crates/agentmon/src/input.rs` - the details modal's Logs-pane branch of `handle_modal_key` branches on whether the selection is a prompt-category entry (no truncation check); `handle_logs_tab_key`'s `Enter` arm is unaffected (always opens the project details modal).
- No wire/protocol changes (`agentmon-proto`) and no hook changes (`agentmon-report`) - the hooks already fire in the correct order; the fix is entirely on the daemon's event-processing side.
