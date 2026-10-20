## MODIFIED Requirements

### Requirement: Logs tab shows an aggregated, paginated activity list
The Logs tab SHALL display the activity log entries received from the daemon, aggregated across all projects (including prompt-category entries), as a paginated list ordered by recency (most recent first) by default; when a prompt-category entry and the agent-category "started" entry it precipitated share the same event-occurred time, the prompt entry SHALL sort ahead of (be shown as more recent than) that started entry, consistent with the order the daemon recorded them in. Each entry SHALL show at least its project, event category, status, and event time, with event time as the last (rightmost) column, consistent with the Agents tab's UPDATED column. Each entry's status SHALL use the same emoji marker and color styling as that status uses in the Agents tab, so status is visually distinguishable the same way it already is there. A test-run entry whose status is "passed" or "failed", an agent entry whose status is "done", or a reminder entry whose status is "finished" or "stopped", SHALL additionally show the elapsed duration of that run/task/reminder, computed from the most recent preceding "started" entry of the same category logged for the same project - for a reminder entry, additionally matched to the "started" entry recording the same reminder name, since a project can have more than one reminder interleaved in the log and an agent/test-run has no equivalent identity to disambiguate by. A reminder entry whose status is "started" SHALL additionally show its recorded ETA (the reminder's local-time due time, `h:MM AM/PM`, as recorded on that entry - see the activity-log capability), rather than the elapsed duration shown for other completed entries, since a just-started reminder has no elapsed run to show. An agent entry ("started", "done", or "needs input") that was recorded with a last-prompt snapshot SHALL additionally show a snippet of that prompt text after its existing content, as a single line truncated with a trailing `...` rather than overflowing the row; an agent entry recorded with no last-prompt snapshot SHALL show no such snippet. A prompt-category entry's entire label, beyond its 🗣️ marker, SHALL be its recorded prompt text rendered as a single-line snippet, separated from the marker by a single space, truncated with a trailing `...` rather than overflowing the row - unlike an agent entry's snippet (appended after other status content), a prompt entry's snippet is its only content, and unlike an agent entry's snapshot, a prompt entry's snippet is never absent. Both kinds of snippet SHALL be sized against the STATUS column's actual rendered width at the current terminal size, the same way the Agents tab's AGENTS and REMINDERS columns and the details modal's Reminders pane size their own truncation against their real rendered width, rather than against a fixed character count chosen independently of how much room the column actually has - so a wider terminal shows more of the prompt instead of leaving the column's unused width blank. Whenever the list holds more entries than fit on a single page, the Logs tab SHALL display a Ratatui vertical `Scrollbar` widget along its right edge, positioned so the scrollbar's thumb reaches the very bottom of its track when the list's last page is being shown (and the very top when its first page is being shown), and its heading SHALL append a pagination keyboard-shortcut hint alongside its existing sort and filter hints; neither the scrollbar nor the pagination hint SHALL be shown when every (filtered) entry already fits on a single page.

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

#### Scenario: The Logs tab's timestamp column is the last column
- **WHEN** the Logs tab renders its header and rows
- **THEN** the event-time column appears after the project, category, and status columns, rather than before them

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

### Requirement: Status labels are prefixed by category outside dedicated panes
In the Agents tab and the Logs tab, each status label SHALL be prefixed with a category word - "agent" for an agent status, "tests" for a test-run status, or "reminder" for a reminder-category log entry - immediately after its emoji marker and before the status word (e.g. "🤖 agent done", "🧪 tests started", "⏰ reminder finished"), so a label read in a cross-category view is unambiguous about which kind of status it names. A prompt-category log entry is the one exception to this cross-category rule in the Logs tab and the details modal's Logs pane: it SHALL NOT include a category word or a status word at all - its label is its 🗣️ marker, then a single space, then its prompt snippet, per the "Logs tab shows an aggregated, paginated activity list" requirement - since the entry's single, always-"submitted" status carries no information the category word or a status word could usefully add, and the Logs tab's own CATEGORY column (or, in the details modal's Logs pane, the marker itself) already identifies it. The Agents tab's Reminders column is the one exception to this cross-category rule: it SHALL NOT include the category-word prefix, since the column's narrow width prioritizes the reminder's own name (the primary content of that cell) and the REMINDERS column header already establishes the category. In the details modal, the Agents pane, Tests pane, and Reminders pane SHALL NOT include this category-word prefix, since the pane itself already establishes the category (e.g. "🤖 done", "❌ failed"); the modal's Logs pane SHALL include the category-word prefix for agent, test-run, and reminder entries, consistent with the top-level Logs tab it mirrors. The top-level Reminders tab's Status column SHALL likewise NOT include this category-word prefix, since the tab already dedicates its entire view to reminders, consistent with how the modal's Agents and Tests panes omit it.

#### Scenario: The Agents tab prefixes an agent status with its category word
- **WHEN** the Agents tab renders an agent's status in its Agents column
- **THEN** the label reads the emoji, then "agent", then the status word (e.g. "🤖 agent done"), rather than omitting the category word

#### Scenario: The Agents tab prefixes a test-run status with its category word
- **WHEN** the Agents tab renders a project's test-run status in its Tests column
- **THEN** the label reads the emoji, then "tests", then the status word (e.g. "⏳ tests running"), rather than omitting the category word

#### Scenario: The Agents tab's Reminders column omits the category-word prefix
- **WHEN** the Agents tab renders a project's reminder status in its Reminders column
- **THEN** the label omits the category word (e.g. an emoji followed directly by the reminder's bold name), unlike its Agents and Tests columns

#### Scenario: The Logs tab prefixes entries with their category word
- **WHEN** the Logs tab renders an agent-category, test-run-category, or reminder-category entry
- **THEN** the label includes the category word ("agent", "tests", or "reminder") alongside its emoji and status, consistent with the Agents tab

#### Scenario: The Logs tab prefixes a prompt entry with its category word
- **WHEN** the Logs tab or the details modal's Logs pane renders a prompt-category entry
- **THEN** the label reads 🗣️, then a single space, then the entry's prompt snippet, with no "prompt" or "submitted" word - unlike every other category, a prompt entry's label carries no category or status word at all

#### Scenario: The details modal's Agents and Tests panes omit the category-word prefix
- **WHEN** the details modal renders a status in its Agents pane, Tests pane, or Reminders pane
- **THEN** the label omits the category word (e.g. "🤖 done", not "🤖 agent done")

#### Scenario: The details modal's Logs pane keeps the category-word prefix
- **WHEN** the details modal renders an agent-category, test-run-category, or reminder-category entry in its Logs pane
- **THEN** the label includes the category word, consistent with the top-level Logs tab

#### Scenario: The Reminders tab omits the category-word prefix
- **WHEN** the Reminders tab renders a reminder's status in its Status column
- **THEN** the label omits the category word (e.g. "⏳ running"), since the tab already dedicates its view to reminders
