## MODIFIED Requirements

### Requirement: Prompt entry full-text modal
When the details modal's Logs pane holds focus and the user presses `Enter` on its selected log entry, and that entry is either a prompt-category entry or an agent-category entry with status "done" that carries a response snapshot, the TUI SHALL open a read-only modal overlay showing that entry's full text - the prompt text for a prompt-category entry, the response text for a "done" entry - consistently, regardless of whether its displayed snippet happened to be truncated, so the user never has to judge a row's length to know what `Enter` will do. This modal SHALL wrap its text to its own width and, if the full text exceeds the modal's height, SHALL make it scrollable with `j`/`k` (and `g`/`G` to jump to its start/end) rather than truncating it - unlike the non-interactive Last Prompt pane, the entire purpose of this modal is to show text the Logs pane's row might not have fit. The user SHALL be able to close it with `Esc`, returning to the details modal with its Logs pane still focused and its selection unchanged, without altering any tracked agent, test-run, or reminder state. `Enter` on any other selected entry in the Logs pane - any entry that is neither a prompt-category entry nor a "done" agent-category entry carrying a response snapshot - SHALL have no effect, unchanged from the Logs pane's behavior before this modal existed; this includes a "done" entry recorded with no response snapshot (e.g. because its transcript could not be read). The top-level Logs tab's own `Enter` is unaffected by this requirement: per the "Project details modal" requirement, it always opens the project details modal, regardless of the selected entry's category, status, or truncation.

#### Scenario: Enter on any prompt entry in the details modal's Logs pane opens its full text
- **WHEN** the details modal's Logs pane holds focus and the user presses `Enter` on a selected prompt-category entry, whether or not its shown snippet is truncated
- **THEN** the TUI opens a modal, layered above the details modal, showing that entry's full prompt text

#### Scenario: Enter on a done entry with a captured response in the details modal's Logs pane opens its full text
- **WHEN** the details modal's Logs pane holds focus and the user presses `Enter` on a selected agent-category entry with status "done" that carries a response snapshot, whether or not its shown snippet is truncated
- **THEN** the TUI opens a modal, layered above the details modal, showing that entry's full response text

#### Scenario: Enter on a non-prompt entry in the Logs pane has no effect
- **WHEN** the details modal's Logs pane holds focus and the user presses `Enter` on a selected entry that is neither prompt-category nor a "done" entry carrying a response snapshot
- **THEN** nothing happens - the pane's entries and selection are unaffected

#### Scenario: Enter on a done entry with no captured response has no effect
- **WHEN** the details modal's Logs pane holds focus and the user presses `Enter` on a selected agent-category "done" entry that carries no response snapshot
- **THEN** nothing happens - the pane's entries and selection are unaffected, the same as for any other non-qualifying entry

#### Scenario: Closing the full-prompt modal returns to the details modal
- **WHEN** the user presses `Esc` while the full-prompt modal is open
- **THEN** the TUI closes it and returns to the details modal with its Logs pane still focused and selection unchanged

#### Scenario: A full prompt that overflows the modal is scrollable
- **WHEN** the selected prompt entry's full text, wrapped to the modal's width, exceeds the modal's height
- **THEN** the user can scroll through it with `j`/`k` (and jump to its start/end with `g`/`G`) rather than it being truncated or left unreachable

#### Scenario: Enter on a prompt entry in the Logs tab is unaffected by this requirement
- **WHEN** the user presses `Enter` on a selected prompt-category entry in the top-level Logs tab, truncated or not
- **THEN** the TUI opens the project details modal, per the "Project details modal" requirement, not this full-prompt modal

#### Scenario: Enter on a done entry in the Logs tab is unaffected by this requirement
- **WHEN** the user presses `Enter` on a selected agent-category "done" entry in the top-level Logs tab, whether or not it carries a response snapshot
- **THEN** the TUI opens the project details modal, per the "Project details modal" requirement, not this full-prompt modal
