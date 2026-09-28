## ADDED Requirements

### Requirement: Agents tab supports incremental search by project name
The Agents tab SHALL support filtering its project rows by a search string matched as a case-insensitive substring against each row's project name (the name derived from its working directory, as already shown in the PROJECT column). Pressing `/` while the Agents tab is active SHALL enter search-editing mode, replacing the tab's title with a live text-entry prompt showing the search string typed so far and a cursor at its end. While editing, each character typed SHALL append to the search string and each `Backspace` SHALL remove its last character, and the Agents tab's displayed rows SHALL update immediately after every such change to show only rows whose project name matches the current search string, or every row if the search string is empty. Rows that match the search string SHALL retain the same relative order (most recent update first) they would have with no filter applied. This search filter SHALL apply only to the Agents tab; the Logs and Reminders tabs' own filters are unaffected and unaffected by it.

#### Scenario: Entering search-editing mode
- **WHEN** the user presses `/` while the Agents tab is active
- **THEN** the Agents tab's title becomes a text-entry prompt showing a cursor, and no rows are hidden yet beyond whatever filter was already applied

#### Scenario: Typing narrows the list as you go
- **WHEN** the user is in search-editing mode and types a character
- **THEN** the character is appended to the search string and the Agents tab immediately shows only rows whose project name contains that search string, case-insensitively

#### Scenario: Backspace removes the last character
- **WHEN** the user is in search-editing mode and presses `Backspace`
- **THEN** the last character of the search string is removed and the displayed rows update to match the shorter string

#### Scenario: Matching rows keep their existing relative order
- **WHEN** more than one project row matches the current search string
- **THEN** the matching rows are displayed in the same relative order (most recent update first) they would use with no filter applied

#### Scenario: A newly-tracked agent for a matching project appears while filtered
- **WHEN** a search filter is applied on the Agents tab and an agent is newly tracked for a project whose name matches that filter
- **THEN** that project's row appears in the filtered list like any other row that matches

### Requirement: Committing, cancelling, and clearing the Agents tab search
Pressing `Enter` while in search-editing mode SHALL commit the current search string as the Agents tab's applied filter and exit search-editing mode; if the committed string is empty, this SHALL clear any previously applied filter instead. Pressing `Escape` while in search-editing mode SHALL discard the in-progress edit and exit search-editing mode, restoring whichever filter (if any) was applied before `/` was pressed, leaving it unchanged. Re-entering search-editing mode with `/` while an applied filter is already present SHALL start the edit buffer from that filter's text rather than empty, so refining or clearing it does not require retyping it.

#### Scenario: Committing a non-empty search
- **WHEN** the user presses `Enter` in search-editing mode with a non-empty search string
- **THEN** that string becomes the Agents tab's applied filter, search-editing mode ends, and the tab's title displays the applied filter

#### Scenario: Committing an empty search clears the filter
- **WHEN** the user presses `Enter` in search-editing mode with an empty search string
- **THEN** any previously applied Agents tab filter is cleared, search-editing mode ends, and every row is shown again

#### Scenario: Cancelling an edit restores the prior filter
- **WHEN** the user presses `Escape` in search-editing mode after having already applied a filter, and edits it further before cancelling
- **THEN** search-editing mode ends and the previously applied filter is restored unchanged, discarding the in-progress edit

#### Scenario: Cancelling an edit with no prior filter clears the prompt
- **WHEN** the user presses `Escape` in search-editing mode and no filter was applied beforehand
- **THEN** search-editing mode ends with no filter applied and every row is shown

#### Scenario: Re-editing an applied filter starts from its text
- **WHEN** the user presses `/` while an applied filter is already showing in the Agents tab title
- **THEN** search-editing mode begins with the edit buffer pre-filled with that filter's text and the cursor at its end

### Requirement: Agents tab title reflects search state
The Agents tab's title SHALL show a `Filter [/]` keyboard shortcut hint whenever search-editing mode is not active, matching the hint convention already used by the Logs and Reminders tabs' own sort/filter controls, which stay visible whether or not a filter is currently applied. When a search filter is applied (and search-editing mode is not active), the title SHALL display the applied filter text immediately after that hint, bold and in a color distinct from the title's normal text, so an active filter remains obvious at a glance without hiding how to change it.

#### Scenario: Hint shown with no applied filter
- **WHEN** the Agents tab is active, not in search-editing mode, and no filter is applied
- **THEN** the tab's title includes a `Filter [/]` keyboard shortcut hint

#### Scenario: Hint remains visible alongside an applied filter
- **WHEN** the Agents tab has an applied search filter and is not in search-editing mode
- **THEN** the tab's title still includes the `Filter [/]` keyboard shortcut hint, immediately followed by the applied filter text

#### Scenario: Applied filter is visually distinguished
- **WHEN** the Agents tab has an applied search filter and is not in search-editing mode
- **THEN** the filter text shown in the title is bold and a distinct color from the title's normal text
