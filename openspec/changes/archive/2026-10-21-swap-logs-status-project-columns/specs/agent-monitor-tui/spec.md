## MODIFIED Requirements

### Requirement: Logs tab shows an aggregated, paginated activity list
The Logs tab SHALL display the activity log entries received from the daemon, aggregated across all projects (including prompt-category entries), as a paginated list ordered by recency (most recent first) by default; when a prompt-category entry and the agent-category "started" entry it precipitated share the same event-occurred time, the prompt entry SHALL sort ahead of (be shown as more recent than) that started entry, consistent with the order the daemon recorded them in. Each entry SHALL show at least its status, event category, project, and event time, in that left-to-right column order - STATUS leftmost, then CATEGORY, then PROJECT, then event time as the last (rightmost) column, consistent with the Agents tab's UPDATED column - so the narrow margin still visible around the details modal's centered overlay shows status/prompt text rather than a project name. Each entry's status SHALL use the same emoji marker and color styling as that status uses in the Agents tab, so status is visually distinguishable the same way it already is there. The PROJECT column's width SHALL be sized to the widest project display name among the currently displayed entries (never narrower than its own "PROJECT" header), taking a user-assigned custom project name into account the same as a directory-derived default name, rather than a fixed proportional share of the table width. Once PROJECT, CATEGORY, and TIME have what they need, the STATUS column SHALL receive the remaining available width, so STATUS - not PROJECT - gets priority for any extra room and is the last of the two to have its content truncated. To keep an unusually long project name from squeezing STATUS down to an unusable width, PROJECT SHALL be capped at half of the combined width available to PROJECT and STATUS; content that doesn't fit within PROJECT's resulting width is clipped without an ellipsis, consistent with how the Agents tab's PROJECT column already handles an oversized name. A test-run entry whose status is "passed" or "failed", an agent entry whose status is "done", or a reminder entry whose status is "finished" or "stopped", SHALL additionally show the elapsed duration of that run/task/reminder, computed from the most recent preceding "started" entry of the same category logged for the same project - for a reminder entry, additionally matched to the "started" entry recording the same reminder name, since a project can have more than one reminder interleaved in the log and an agent/test-run has no equivalent identity to disambiguate by. A reminder entry whose status is "started" SHALL additionally show its recorded ETA (the reminder's local-time due time, `h:MM AM/PM`, as recorded on that entry - see the activity-log capability), rather than the elapsed duration shown for other completed entries, since a just-started reminder has no elapsed run to show. An agent entry ("started", "done", or "needs input") that was recorded with a last-prompt snapshot SHALL additionally show a snippet of that prompt text after its existing content, as a single line truncated with a trailing `...` rather than overflowing the row; an agent entry recorded with no last-prompt snapshot SHALL show no such snippet. A prompt-category entry's entire label, beyond its 🗣️ marker, SHALL be its recorded prompt text rendered as a single-line snippet, separated from the marker by a single space, truncated with a trailing `...` rather than overflowing the row - unlike an agent entry's snippet (appended after other status content), a prompt entry's snippet is its only content, and unlike an agent entry's snapshot, a prompt entry's snippet is never absent. Both kinds of snippet SHALL be sized against the STATUS column's actual rendered width at the current terminal size, the same way the Agents tab's AGENTS and REMINDERS columns and the details modal's Reminders pane size their own truncation against their real rendered width, rather than against a fixed character count chosen independently of how much room the column actually has - so a wider terminal shows more of the prompt instead of leaving the column's unused width blank. Whenever the list holds more entries than fit on a single page, the Logs tab SHALL display a Ratatui vertical `Scrollbar` widget along its right edge, positioned so the scrollbar's thumb reaches the very bottom of its track when the list's last page is being shown (and the very top when its first page is being shown), and its heading SHALL append a pagination keyboard-shortcut hint alongside its existing sort and filter hints; neither the scrollbar nor the pagination hint SHALL be shown when every (filtered) entry already fits on a single page.

#### Scenario: Logs tab lists recent activity across projects
- **WHEN** the user switches to the Logs tab
- **THEN** the TUI displays activity log entries from every project, most recent first, split into pages

#### Scenario: A new entry arrives while viewing the Logs tab
- **WHEN** the daemon pushes a new activity log entry while the Logs tab is active
- **THEN** the TUI incorporates it into the list without requiring the user to restart or reconnect

#### Scenario: Log entry statuses are visually distinguishable
- **WHEN** the Logs tab renders an entry's status
- **THEN** that status is prefixed with the same emoji marker and styled with the same color it would use in the Agents tab

#### Scenario: A completed test run shows its duration
- **WHEN** the Logs tab renders a "passed" or "failed" test-run entry that has a preceding "test-run"/"started" entry logged for the same project
- **THEN** the row includes the elapsed duration between that "started" entry and this one

#### Scenario: A completed agent task shows its duration
- **WHEN** the Logs tab renders a "done" agent entry that has a preceding "agent"/"started" entry logged for the same project
- **THEN** the row includes the elapsed duration between that "started" entry and this one

#### Scenario: A started test run with no completion shows no duration
- **WHEN** the Logs tab renders a "started" test-run entry
- **THEN** the row shows no duration for it, since the run has not yet completed in the log

#### Scenario: STATUS is the leftmost column
- **WHEN** the Logs tab renders its header and rows
- **THEN** the STATUS column appears first (far left), followed by CATEGORY, then PROJECT, then TIME

#### Scenario: The Logs tab's timestamp column is the last column
- **WHEN** the Logs tab renders its header and rows
- **THEN** the event-time column appears after the status, category, and project columns, rather than before them

#### Scenario: A long project name widens the PROJECT column
- **WHEN** a currently displayed entry's project display name (including a user-assigned custom name) is wider than the PROJECT header and than any other currently displayed entry's project name
- **THEN** the PROJECT column widens to fit that name, up to its half-of-combined-width cap, rather than leaving it truncated while STATUS holds a fixed share of the width

#### Scenario: Short project names leave more room for STATUS
- **WHEN** every currently displayed entry's project display name is narrower than a fixed proportional share of the table would have reserved for it
- **THEN** the PROJECT column shrinks to only what those names actually need, and the STATUS column receives the width that frees up

#### Scenario: An unusually long project name does not starve STATUS
- **WHEN** a currently displayed entry's project display name is long enough that giving it everything it needs would leave STATUS with less than half of the two columns' combined available width
- **THEN** the PROJECT column is capped at half of that combined width, its content is clipped without an ellipsis, and the STATUS column receives the remaining half

#### Scenario: A scrollbar appears once entries overflow a page
- **WHEN** the Logs tab's (filtered) entries hold more rows than fit on a single page
- **THEN** the TUI renders a vertical scrollbar along the pane's right edge, reflecting the current page's position within the full list

#### Scenario: No scrollbar when everything fits on one page
- **WHEN** the Logs tab's (filtered) entries all fit within a single page
- **THEN** no scrollbar is rendered

#### Scenario: The heading gains a pagination hint once entries overflow a page
- **WHEN** the Logs tab's (filtered) entries hold more rows than fit on a single page
- **THEN** its heading shows a hint for the pagination keys alongside its existing sort and filter hints

#### Scenario: No pagination hint when everything fits on one page
- **WHEN** the Logs tab's (filtered) entries all fit within a single page
- **THEN** its heading shows no pagination hint, only the sort and filter hints

#### Scenario: The scrollbar thumb reaches the bottom of the track on the last page
- **WHEN** the Logs tab's (filtered) entries hold more rows than fit on a single page, and the list is scrolled to its last page
- **THEN** the scrollbar's thumb is positioned flush against the bottom of its track, not stopping short of it

#### Scenario: A finished reminder shows its elapsed duration
- **WHEN** the Logs tab renders a "finished" reminder entry that has a preceding "started" entry logged for the same project and the same reminder name
- **THEN** the row includes the elapsed duration between that "started" entry and this one

#### Scenario: A stopped reminder shows its elapsed duration
- **WHEN** the Logs tab renders a "stopped" reminder entry that has a preceding "started" entry logged for the same project and the same reminder name
- **THEN** the row includes the elapsed duration between that "started" entry and this one

#### Scenario: A started reminder with no completion shows no duration
- **WHEN** the Logs tab renders a "started" reminder entry
- **THEN** the row shows no duration for it, since it has not yet stopped or finished in the log

#### Scenario: A started reminder shows its ETA
- **WHEN** the Logs tab renders a "started" reminder entry
- **THEN** the row includes that entry's recorded ETA (`h:MM AM/PM`, local time) alongside its name

#### Scenario: Interleaved reminders in the same project do not cross-match durations
- **WHEN** the Logs tab renders a "finished" or "stopped" reminder entry, and a different reminder in the same project also has "started"/"finished"/"stopped" entries interleaved with it in the log
- **THEN** the displayed duration is computed only from that reminder's own preceding "started" entry (matched by reminder name), not another reminder's

#### Scenario: An agent entry shows its recorded prompt snippet
- **WHEN** the Logs tab renders an agent-category entry that was recorded with a last-prompt snapshot
- **THEN** the row shows a single-line snippet of that prompt after its existing status/duration text, with a trailing `...` if it does not fit

#### Scenario: An agent entry with no recorded prompt shows no snippet
- **WHEN** the Logs tab renders an agent-category entry recorded with no last-prompt snapshot
- **THEN** the row shows no prompt snippet, rather than empty quotes or placeholder text

#### Scenario: A prompt entry shows its submitted text
- **WHEN** the Logs tab renders a prompt-category entry
- **THEN** the row shows 🗣️, then a single space, then a single-line snippet of the submitted prompt text, with no "prompt submitted" wording and no surrounding quotes

#### Scenario: A prompt entry that starts a run sorts just ahead of that started entry
- **WHEN** the Logs tab renders a prompt-category entry and the agent-category "started" entry it precipitated, both logged with the same event-occurred time
- **THEN** the prompt entry appears immediately above (more recent than) the started entry, rather than the two appearing in an arbitrary relative order

#### Scenario: A wider STATUS column shows more of a prompt snippet
- **WHEN** the terminal is wide enough that the Logs tab's STATUS column (or the details modal's Logs pane) renders wider than it would at a narrower terminal size, and a prompt-category or last-prompt-snapshot entry's full text exceeds what fit at the narrower size
- **THEN** the rendered snippet shows more of that entry's text than it did at the narrower size, up to the column's new width, rather than staying capped at the same fixed length regardless of the column's actual width
