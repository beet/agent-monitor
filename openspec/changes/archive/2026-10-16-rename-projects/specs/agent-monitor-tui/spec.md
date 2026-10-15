## ADDED Requirements

### Requirement: Project names reflect custom renames everywhere

Every place the TUI displays, sorts by, or filters by a project's name - the Agents tab's PROJECT column, the Logs tab, the Reminders tab, the details modal's title, every cycle-filter, and the Agents tab's incremental search - SHALL use that project's current effective display name (its custom name if it has been renamed, per the project-renaming capability, otherwise its directory-derived default) rather than the name in effect at some earlier point in time. This applies to historic activity-log entries exactly as it does to a currently-tracked agent, test run, or reminder: a log entry carries no name of its own, so renaming a project changes how every one of its past entries is displayed too, immediately and without needing to re-fetch or replay anything.

#### Scenario: A renamed project's historic log entries show the new name

- **WHEN** a project is renamed and the Logs tab or the details modal's Logs pane renders an entry recorded before the rename
- **THEN** that entry displays the project's new name, not the name in effect when the entry was recorded

#### Scenario: A rename is reflected in the Agents tab immediately

- **WHEN** a project is renamed while its row is visible in the Agents tab
- **THEN** the row's PROJECT column updates to the new name without the user needing to scroll, re-select, or restart the TUI

#### Scenario: A rename is reflected in the Reminders tab and its filters

- **WHEN** a project with tracked reminders is renamed
- **THEN** the Reminders tab's PROJECT column, and its project cycle-filter's choices, use the new name

#### Scenario: A rename is reflected in the details modal's title

- **WHEN** a project is renamed while its details modal is open
- **THEN** the modal's title updates to show the new name

### Requirement: Agents tab title shows a Rename shortcut hint

The Agents tab's title SHALL show a `Rename [n]` keyboard shortcut hint immediately after the `Filter [/]` hint (and after any applied filter text, if one is shown), whenever search-editing mode is not active - the same always-visible convention the `Filter [/]` hint itself already follows. While search-editing mode is active, the title SHALL show only the in-progress search prompt, consistent with how the `Filter [/]` hint itself is already hidden then.

#### Scenario: Rename hint shown with no applied filter

- **WHEN** the Agents tab is active and not in search-editing mode
- **THEN** the tab's title includes a `Rename [n]` keyboard shortcut hint after the `Filter [/]` hint

#### Scenario: Rename hint remains visible alongside an applied filter

- **WHEN** the Agents tab has an applied search filter and is not in search-editing mode
- **THEN** the tab's title still includes the `Rename [n]` keyboard shortcut hint, after the `Filter [/]` hint and the applied filter text

#### Scenario: Rename hint is hidden while editing the search prompt

- **WHEN** the Agents tab's search prompt is being edited
- **THEN** the tab's title does not include the `Rename [n]` hint, matching the `Filter [/]` hint's own behavior while editing

### Requirement: Project details modal title shows a Rename shortcut hint

The details modal's title SHALL show a `Rename [n]` keyboard shortcut hint immediately after the project's name.

#### Scenario: The details modal title shows the hint next to the project name

- **WHEN** the details modal is open for a project
- **THEN** its title shows that project's name followed by a `Rename [n]` keyboard shortcut hint

### Requirement: Project rename form

Pressing `n` on a selected project row in the Agents tab, or pressing `n` while the details modal is open (regardless of which pane currently holds focus, the same as creating a reminder), SHALL open a rename form for that project, pre-populated with its current display name and with editing focus already on that text. Pressing `n` in the Agents tab with no project row selected (an empty list) SHALL have no effect. While the form is open, typed characters append to its text and `Backspace` removes the last character, with no character treated specially - including `q`, which SHALL be appended to the text rather than quitting the TUI, since quitting-while-typing would make it impossible to include that character in a project name. Pressing `Esc` SHALL discard the form and leave the project's display name unchanged. Pressing `Enter` SHALL validate the entered text against the project-renaming capability's rules (non-blank after trimming, unique among all known projects' effective display names); if validation fails, the form SHALL remain open with the entered text unchanged and an inline message explaining the problem, and no rename SHALL be requested; if validation succeeds, the form SHALL close and request the rename, and every view showing that project's name SHALL update once the rename takes effect, per "Project names reflect custom renames everywhere."

#### Scenario: Opening the rename form from the Agents tab

- **WHEN** the user presses `n` on a selected project row in the Agents tab
- **THEN** the TUI opens a rename form pre-populated with that project's current display name, with editing focus on it

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
- **THEN** the character is appended to the form's text and the TUI does not quit
