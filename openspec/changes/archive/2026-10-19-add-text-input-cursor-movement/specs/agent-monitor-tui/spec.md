## MODIFIED Requirements

### Requirement: Agents tab supports incremental search by project name
The Agents tab SHALL support filtering its project rows by a search string matched as a case-insensitive substring against each row's project name (the name derived from its working directory, as already shown in the PROJECT column). Pressing `/` while the Agents tab is active SHALL enter search-editing mode, replacing the tab's title with a live text-entry prompt showing the search string typed so far and a cursor positioned at its end. While editing, the prompt SHALL track a cursor position within the search string and support the following line-editing keys, each acting relative to that position: `Left`/`Right` move the cursor one character; `Home`/`End` move it to the start/end of the string; `Ctrl+Left`/`Ctrl+Right` move it by one word; `Delete` removes the character after the cursor; `Backspace` removes the character before it; `Ctrl+W` removes the word before the cursor; `Ctrl+K` removes from the cursor to the end of the string; `Ctrl+U` clears the string. Every movement key SHALL be clamped so the cursor never moves before the first character or after the last. Each printable character typed SHALL be inserted at the cursor position (not necessarily the end) and the cursor SHALL advance past it. After every such change the Agents tab's displayed rows SHALL update immediately to show only rows whose project name matches the current search string, or every row if the search string is empty. Rows that match the search string SHALL retain the same relative order (most recent update first) they would have with no filter applied. This search filter SHALL apply only to the Agents tab; the Logs and Reminders tabs' own filters are unaffected and unaffected by it.

#### Scenario: Entering search-editing mode
- **WHEN** the user presses `/` while the Agents tab is active
- **THEN** the Agents tab's title becomes a text-entry prompt showing a cursor at the end of the search string, and no rows are hidden yet beyond whatever filter was already applied

#### Scenario: Typing narrows the list as you go
- **WHEN** the user is in search-editing mode, the cursor is at the end of the search string, and the user types a character
- **THEN** the character is inserted at the end of the search string and the Agents tab immediately shows only rows whose project name contains that search string, case-insensitively

#### Scenario: Backspace removes the last character
- **WHEN** the user is in search-editing mode, the cursor is at the end of the search string, and the user presses `Backspace`
- **THEN** the last character of the search string is removed and the displayed rows update to match the shorter string

#### Scenario: Left and Right move the cursor within the search string
- **WHEN** the user is in search-editing mode and presses `Left` or `Right`
- **THEN** the cursor moves one character in that direction within the search string, without changing the string itself

#### Scenario: Home and End jump the cursor to either end of the search string
- **WHEN** the user is in search-editing mode and presses `Home` or `End`
- **THEN** the cursor moves to the start or end of the search string, respectively, without changing the string itself

#### Scenario: Ctrl+Left and Ctrl+Right move the cursor by word
- **WHEN** the user is in search-editing mode and presses `Ctrl+Left` or `Ctrl+Right`
- **THEN** the cursor moves to the start of the previous word or the start of the next word, respectively

#### Scenario: Typing inserts at the cursor, not always at the end
- **WHEN** the user moves the cursor to a position before the end of the search string and types a character
- **THEN** the character is inserted at the cursor position, the text after it shifts right, the cursor advances past the inserted character, and the displayed rows update to match the new string

#### Scenario: Backspace deletes the character before the cursor
- **WHEN** the user moves the cursor to a position before the end of the search string and presses `Backspace`
- **THEN** the character immediately before the cursor is removed (not necessarily the string's last character), the cursor moves back one position, and the displayed rows update to match the shorter string

#### Scenario: Delete removes the character after the cursor
- **WHEN** the user moves the cursor to a position before the end of the search string and presses `Delete`
- **THEN** the character immediately after the cursor is removed, the cursor position is unchanged, and the displayed rows update to match the shorter string

#### Scenario: Ctrl+W removes the word before the cursor
- **WHEN** the user is in search-editing mode and presses `Ctrl+W`
- **THEN** the word immediately before the cursor is removed and the displayed rows update to match the shorter string

#### Scenario: Ctrl+K clears from the cursor to the end of the string
- **WHEN** the user is in search-editing mode and presses `Ctrl+K`
- **THEN** everything from the cursor to the end of the search string is removed and the displayed rows update to match the shorter string

#### Scenario: Ctrl+U clears the search string
- **WHEN** the user is in search-editing mode and presses `Ctrl+U`
- **THEN** the search string becomes empty, the cursor moves to the start, and every row is shown again

#### Scenario: The cursor cannot move past either end of the string
- **WHEN** the cursor is at the start of the search string and the user presses `Left`, or the cursor is at the end and the user presses `Right`
- **THEN** the cursor does not move

#### Scenario: Matching rows keep their existing relative order
- **WHEN** more than one project row matches the current search string
- **THEN** the matching rows are displayed in the same relative order (most recent update first) they would use with no filter applied

#### Scenario: A newly-tracked agent for a matching project appears while filtered
- **WHEN** a search filter is applied on the Agents tab and an agent is newly tracked for a project whose name matches that filter
- **THEN** that project's row appears in the filtered list like any other row that matches

### Requirement: Reminder creation and editing form
The TUI SHALL provide a modal form, opened from the details modal's Reminders pane, for creating (`R`) or editing (`e`) a reminder, with a Name field and a Duration-in-minutes field. `Tab` SHALL move focus between the two fields, resetting the cursor to the end of the newly-focused field's text. When opened for editing, both fields SHALL be pre-populated with the highlighted reminder's current name and duration, with the cursor positioned at the end of the active field's text. Pressing `Enter` SHALL save the form - creating a new reminder for the modal's project, or applying the edited name/duration to the reminder being edited - and close the form. Pressing `Escape` SHALL close the form without creating a reminder or applying any edit made since it was opened. While the form is open, the active field SHALL track a cursor position within its text and support the following line-editing keys, each acting relative to that position: `Left`/`Right` move the cursor one character; `Home`/`End` move it to the start/end of the field's text; `Ctrl+Left`/`Ctrl+Right` move it by one word; `Delete` removes the character after the cursor; `Backspace` removes the character before it; `Ctrl+W` removes the word before the cursor; `Ctrl+K` removes from the cursor to the end of the field's text; `Ctrl+U` clears the field. Every movement key SHALL be clamped so the cursor never moves before the first character or after the last. Each printable character typed (digits only for the Duration field, as before; a non-digit character key on that field SHALL be ignored rather than edit the field) SHALL be inserted at the cursor position and the cursor SHALL advance past it. The TUI SHALL show the terminal's own text cursor positioned at the active field's tracked cursor position within its text (before the Duration field's trailing "m" when that cursor sits at the end of the typed digits), so the user can see where the next typed character will land or be removed from. Once the daemon confirms a newly-created reminder (the daemon assigns its id, so this cannot happen synchronously with `Enter`), the TUI SHALL highlight it in the Reminders pane - focusing that pane if it does not already hold focus - and start it, the same as if the user had pressed `Enter`/`s` on it themselves; this SHALL NOT happen for an edited (as opposed to newly-created) reminder.

#### Scenario: A newly-created reminder is highlighted and started automatically
- **WHEN** the user creates a reminder and the daemon confirms it with the id it assigned
- **THEN** the TUI highlights that reminder in the Reminders pane, focusing the pane if needed, and sends a start request for it - without the user needing to select and start it themselves

#### Scenario: Editing a reminder does not auto-start it
- **WHEN** the user edits an existing reminder's name or duration and the daemon confirms the change
- **THEN** the TUI does not send a start request for it, and does not change which reminder is highlighted

#### Scenario: Creating a reminder
- **WHEN** the user presses `R` in the details modal's Reminders pane, enters a name and a duration, and presses `Enter`
- **THEN** the TUI creates a new reminder for that project with the entered name and duration, and closes the form

#### Scenario: Canceling a new reminder
- **WHEN** the user presses `R`, enters some text, and then presses `Escape`
- **THEN** the TUI closes the form without creating a reminder

#### Scenario: Editing a reminder
- **WHEN** the user presses `e` on a highlighted reminder, the form opens pre-populated with its current name and duration, the user changes a field, and presses `Enter`
- **THEN** the TUI applies the change to that reminder and closes the form

#### Scenario: Discarding an edit
- **WHEN** the user presses `e` on a highlighted reminder, changes a field, and presses `Escape`
- **THEN** the TUI closes the form and the reminder's name and duration remain as they were before the form opened

#### Scenario: Tab moves between the form's fields
- **WHEN** the reminder form is open and the user presses `Tab`
- **THEN** focus moves from the Name field to the Duration field, or back, so the user can edit either without leaving the form

#### Scenario: The cursor tracks the active field as the user types
- **WHEN** the reminder form is open, the cursor is at the end of the active field's text, and the user types or deletes characters
- **THEN** the terminal's cursor is shown immediately after that field's current text, advancing or retreating with each keystroke

#### Scenario: The cursor moves when focus moves between fields
- **WHEN** the reminder form is open and the user presses `Tab`
- **THEN** the cursor moves to the end of the newly-focused field's current text

#### Scenario: Left and Right move the cursor within the active field
- **WHEN** the reminder form is open and the user presses `Left` or `Right`
- **THEN** the cursor moves one character in that direction within the active field's text, and the terminal's cursor is redrawn at the new position

#### Scenario: Home and End jump the cursor to either end of the active field
- **WHEN** the reminder form is open and the user presses `Home` or `End`
- **THEN** the cursor moves to the start or end of the active field's text, respectively

#### Scenario: Ctrl+Left and Ctrl+Right move the cursor by word
- **WHEN** the reminder form is open and the user presses `Ctrl+Left` or `Ctrl+Right`
- **THEN** the cursor moves to the start of the previous word or the start of the next word within the active field's text, respectively

#### Scenario: Typing inserts at the cursor, not always at the end
- **WHEN** the user moves the cursor to a position before the end of the active field's text and types a character
- **THEN** the character is inserted at the cursor position, the text after it shifts right, and the cursor advances past the inserted character

#### Scenario: Backspace deletes the character before the cursor
- **WHEN** the user moves the cursor to a position before the end of the active field's text and presses `Backspace`
- **THEN** the character immediately before the cursor is removed (not necessarily the field's last character), and the cursor moves back one position

#### Scenario: Delete removes the character after the cursor
- **WHEN** the user moves the cursor to a position before the end of the active field's text and presses `Delete`
- **THEN** the character immediately after the cursor is removed and the cursor position is unchanged

#### Scenario: Ctrl+W removes the word before the cursor
- **WHEN** the reminder form is open and the user presses `Ctrl+W`
- **THEN** the word immediately before the cursor in the active field is removed

#### Scenario: Ctrl+K clears from the cursor to the end of the active field
- **WHEN** the reminder form is open and the user presses `Ctrl+K`
- **THEN** everything from the cursor to the end of the active field's text is removed

#### Scenario: Ctrl+U clears the active field
- **WHEN** the reminder form is open and the user presses `Ctrl+U`
- **THEN** the active field's text becomes empty and its cursor moves to the start

#### Scenario: A non-digit key leaves the Duration field unchanged
- **WHEN** the Duration field is active and the user presses a character key that is not a digit
- **THEN** the Duration field's text and cursor are unchanged

#### Scenario: The cursor cannot move past either end of the active field's text
- **WHEN** the cursor is at the start of the active field's text and the user presses `Left`, or at the end and the user presses `Right`
- **THEN** the cursor does not move

### Requirement: Project rename form

Pressing `n` on a selected project row in the Agents tab, or pressing `n` while the details modal is open (regardless of which pane currently holds focus, the same as creating a reminder), SHALL open a rename form for that project, pre-populated with its current display name, with editing focus already on that text and the cursor positioned at the end of it. Pressing `n` in the Agents tab with no project row selected (an empty list) SHALL have no effect. While the form is open, it SHALL track a cursor position within its text and support the following line-editing keys, each acting relative to that position: `Left`/`Right` move the cursor one character; `Home`/`End` move it to the start/end of the text; `Ctrl+Left`/`Ctrl+Right` move it by one word; `Delete` removes the character after the cursor; `Backspace` removes the character before it; `Ctrl+W` removes the word before the cursor; `Ctrl+K` removes from the cursor to the end of the text; `Ctrl+U` clears the text. Every movement key SHALL be clamped so the cursor never moves before the first character or after the last. Each typed character SHALL be inserted at the cursor position (not necessarily the end) and the cursor SHALL advance past it. No character is treated specially - including `q`, which SHALL be inserted into the text rather than quitting the TUI, since quitting-while-typing would make it impossible to include that character in a project name. Pressing `Esc` SHALL discard the form and leave the project's display name unchanged. Pressing `Enter` SHALL validate the entered text against the project-renaming capability's rules (non-blank after trimming, unique among all known projects' effective display names); if validation fails, the form SHALL remain open with the entered text unchanged and an inline message explaining the problem, and no rename SHALL be requested; if validation succeeds, the form SHALL close and request the rename, and every view showing that project's name SHALL update once the rename takes effect, per "Project names reflect custom renames everywhere."

#### Scenario: Opening the rename form from the Agents tab

- **WHEN** the user presses `n` on a selected project row in the Agents tab
- **THEN** the TUI opens a rename form pre-populated with that project's current display name, with editing focus on it and the cursor at its end

#### Scenario: Opening the rename form with no row selected

- **WHEN** the user presses `n` in the Agents tab while its project list is empty
- **THEN** nothing happens

#### Scenario: Opening the rename form from the details modal

- **WHEN** the user presses `n` while the details modal is open, regardless of whether its Logs or Reminders pane currently holds focus
- **THEN** the TUI opens a rename form pre-populated with that project's current display name

#### Scenario: Cancelling the rename form

- **WHEN** the user presses `Esc` while the rename form is open
- **THEN** the form closes and the project's display name is unchanged

#### Scenario: Submitting a valid new name

- **WHEN** the user edits the rename form's text to a non-blank value that does not collide with any other known project and presses `Enter`
- **THEN** the form closes and the project is renamed, with every view showing that project's name updating to reflect it

#### Scenario: Submitting a blank name

- **WHEN** the user clears the rename form's text (or leaves only whitespace) and presses `Enter`
- **THEN** the form remains open, showing an inline message that the name cannot be blank, and the project is not renamed

#### Scenario: Submitting a name that collides with another project

- **WHEN** the user enters a name that matches another, different project's current effective display name and presses `Enter`
- **THEN** the form remains open, showing an inline message that the name is already in use, and the project is not renamed

#### Scenario: Submitting the project's own current name

- **WHEN** the user presses `Enter` without changing the pre-populated text
- **THEN** the form closes as if the rename succeeded, and the project's display name is unchanged

#### Scenario: Typing "q" adds it to the name instead of quitting

- **WHEN** the user types the character `q` while the rename form is open
- **THEN** the character is inserted into the form's text at the cursor position and the TUI does not quit

#### Scenario: Left and Right move the cursor within the rename form's text

- **WHEN** the rename form is open and the user presses `Left` or `Right`
- **THEN** the cursor moves one character in that direction within the form's text, and the terminal's cursor is redrawn at the new position

#### Scenario: Home and End jump the cursor to either end of the text

- **WHEN** the rename form is open and the user presses `Home` or `End`
- **THEN** the cursor moves to the start or end of the form's text, respectively

#### Scenario: Ctrl+Left and Ctrl+Right move the cursor by word

- **WHEN** the rename form is open and the user presses `Ctrl+Left` or `Ctrl+Right`
- **THEN** the cursor moves to the start of the previous word or the start of the next word, respectively

#### Scenario: Typing inserts at the cursor, not always at the end

- **WHEN** the user moves the cursor to a position before the end of the rename form's text and types a character
- **THEN** the character is inserted at the cursor position, the text after it shifts right, and the cursor advances past the inserted character

#### Scenario: Backspace deletes the character before the cursor

- **WHEN** the user moves the cursor to a position before the end of the rename form's text and presses `Backspace`
- **THEN** the character immediately before the cursor is removed (not necessarily the text's last character), and the cursor moves back one position

#### Scenario: Delete removes the character after the cursor

- **WHEN** the user moves the cursor to a position before the end of the rename form's text and presses `Delete`
- **THEN** the character immediately after the cursor is removed and the cursor position is unchanged

#### Scenario: Ctrl+W removes the word before the cursor

- **WHEN** the rename form is open and the user presses `Ctrl+W`
- **THEN** the word immediately before the cursor is removed

#### Scenario: Ctrl+K clears from the cursor to the end of the text

- **WHEN** the rename form is open and the user presses `Ctrl+K`
- **THEN** everything from the cursor to the end of the form's text is removed

#### Scenario: Ctrl+U clears the rename form's text

- **WHEN** the rename form is open and the user presses `Ctrl+U`
- **THEN** the form's text becomes empty and its cursor moves to the start

#### Scenario: The cursor cannot move past either end of the text

- **WHEN** the cursor is at the start of the rename form's text and the user presses `Left`, or at the end and the user presses `Right`
- **THEN** the cursor does not move
