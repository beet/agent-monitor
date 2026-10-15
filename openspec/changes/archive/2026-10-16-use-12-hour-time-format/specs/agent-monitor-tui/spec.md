## MODIFIED Requirements

### Requirement: Live agent list
The TUI SHALL display tracked agents, test runs, and reminders grouped by working directory as one row per project: each distinct working directory forms exactly one row showing that project's name (derived from the working directory), its tracked agent(s)' status(es) in an Agents cell, that project's test run status (if any) in a Tests cell, and the most recent last-updated time among those members (agent, test run, or reminder) (in the system's local timezone, formatted `%Y-%m-%d %-I:%M:%S %p`), updating as the daemon reports changes. A project row SHALL NOT display per-agent host context or process id as separate columns; the daemon continues tracking that data, it is simply not rendered in the collapsed row. Project rows SHALL be ordered by the most recent last-updated time of any member (agent, test run, or reminder) within them, most recent first. An agent whose status is "running" SHALL contribute a live, counting-up duration to the row, computed from the current time minus that agent's status-since timestamp, formatted as a compact counter (e.g. `2m14s`) and kept current by the TUI's own periodic redraw rather than only refreshing when the daemon pushes an update. An agent whose status is "done" SHALL contribute a fixed duration to the row, computed from that agent's run-started timestamp (the beginning of the run that just completed) to its status-since timestamp (the time it completed) - unlike the "running" duration, this fixed duration does not change on further redraws. An agent in any other status (idle, needs input, stale, or declined) SHALL NOT contribute a duration. A test run whose status is "running" SHALL likewise contribute a live, counting-up duration computed from its run-start timestamp. Once a test run reaches "passed" or "failed", its row SHALL continue to show a duration - the total elapsed time from its run-start timestamp to its last-updated timestamp - rather than showing no duration. When the daemon pushes a removal for a session id, the TUI SHALL drop that session id from its tracked agents, so a pid whose session id has been superseded contributes at most one entry - the current, live session's - to its project row's Agents cell and to that project's Agents pane in the details modal, never a second, frozen entry left over from the superseded session. A reminder's last-updated time (its most recent started/stopped/finished event, or its creation time if it has none) SHALL count toward its project's most-recent last-updated time the same way an agent's or test run's does, even though the reminder's own status is not rendered in this row (see "Agents tab shows a Reminders column").

#### Scenario: A new agent starts
- **WHEN** the daemon reports a newly tracked agent
- **THEN** the TUI adds or updates that agent's project row - creating the row if this is the first member tracked for that working directory - without requiring the user to restart the TUI

#### Scenario: An agent's status changes
- **WHEN** the daemon pushes a status update for an agent already shown
- **THEN** the TUI updates that agent's contribution to its project row's Agents cell in place

#### Scenario: An agent goes stale
- **WHEN** the daemon marks a tracked agent as stale/disconnected
- **THEN** the project row reflects that the agent is no longer active rather than showing its last active status unchanged

#### Scenario: Agents are sorted by recency
- **WHEN** the TUI displays two or more project rows with different most-recent last-updated times among their members
- **THEN** the project with the most recent last-updated time is shown first, and the rest follow in descending order

#### Scenario: An update reorders the list
- **WHEN** a member of a project that is not currently shown first receives a status update
- **THEN** that project's row moves to the top, ahead of projects that have not updated as recently

#### Scenario: Last-updated time is shown in local time
- **WHEN** the TUI renders a project row
- **THEN** the row includes the most recent last-updated time among its members, converted to the system's local timezone and formatted as `%Y-%m-%d %-I:%M:%S %p` (e.g. `2026-09-01 4:32:07 PM`), without a UTC offset or timezone code

#### Scenario: A running agent shows its elapsed duration
- **WHEN** the TUI renders a project row whose agent's status is "running"
- **THEN** the row includes a duration computed from the current time minus that agent's status-since timestamp, formatted compactly (e.g. `9s`, `2m14s`, `1h03m`)

#### Scenario: A done agent shows its total duration
- **WHEN** the TUI renders a project row for an agent whose status is "done"
- **THEN** the row includes a fixed duration computed from that agent's run-started timestamp to its status-since timestamp, rather than showing no duration

#### Scenario: A non-running agent shows no duration
- **WHEN** the TUI renders a project row for an agent whose status is not "running" and not "done" (idle, needs input, stale, or declined)
- **THEN** the row does not include a duration for that agent

#### Scenario: The running duration counts up without a new daemon event
- **WHEN** an agent remains "running" and no new event arrives from the daemon for several seconds
- **THEN** the TUI's displayed duration for that agent still increases over that time, driven by the TUI's own periodic redraw

#### Scenario: A status transition resets the displayed duration
- **WHEN** an agent transitions out of and back into "running" status (for example, "running" to "needs input" and back to "running")
- **THEN** the TUI displays a duration counted from the new status-since timestamp, not accumulated from the earlier running period

#### Scenario: A directory with no tracked agent still shows its test run
- **WHEN** a test run is tracked in a working directory with no tracked agent
- **THEN** the TUI displays a row for that project containing only the test run's status and duration in its Tests cell, with no agent status contributed

#### Scenario: A test run appears in an agent's group
- **WHEN** a test-run event is reported for a working directory with tracked agent(s)
- **THEN** the TUI updates that project's single row to include the test run's status and duration in its Tests cell alongside its agent(s)' statuses in its Agents cell, without creating a duplicate row

#### Scenario: A project's row omits host and process id
- **WHEN** the TUI renders a project row
- **THEN** the row does not include separate host-context or process-id columns

#### Scenario: A started test run shows a live elapsed duration
- **WHEN** the TUI renders a project row whose test run status is "running"
- **THEN** the row includes a duration computed from the current time minus that test run's run-start timestamp, formatted compactly like an agent's running duration and kept current by the TUI's own periodic redraw

#### Scenario: A finished test run keeps showing its total duration
- **WHEN** the TUI renders a project row whose test run status is "passed" or "failed"
- **THEN** the row includes a fixed duration computed from that test run's run-start timestamp to its last-updated timestamp, rather than showing no duration

#### Scenario: A new test run resets the displayed duration
- **WHEN** a project's previously finished test run is replaced by a newly reported test run (a different process)
- **THEN** the TUI displays a duration counted from the new run's run-start timestamp, not accumulated from the previous run

#### Scenario: A removed session id no longer renders as a duplicate row
- **WHEN** the daemon pushes a removal for a session id whose pid is already tracked under a different, newer session id
- **THEN** the TUI drops the removed session id from its tracked agents, so that pid's project row and the details modal's Agents pane each show only the live session's entry, not a second one with a different duration

#### Scenario: A project's reminder activity affects its recency ordering
- **WHEN** a project's most recent reminder event (started, stopped, or finished) is more recent than any of its agents' or test runs' last-updated times
- **THEN** that project's row is ordered using the reminder's last-updated time, and sorts ahead of projects whose most recent activity (of any kind) is older

#### Scenario: The updated column reflects reminder activity
- **WHEN** a project's most recent activity is a reminder event rather than an agent or test-run update
- **THEN** the row's displayed last-updated time is that reminder's last-updated time

### Requirement: Logs tab shows an aggregated, paginated activity list
The Logs tab SHALL display the activity log entries received from the daemon, aggregated across all projects, as a paginated list ordered by recency (most recent first) by default. Each entry SHALL show at least its project, event category, status, and event time, with event time as the last (rightmost) column, consistent with the Agents tab's UPDATED column. Each entry's status SHALL use the same emoji marker and color styling as that status uses in the Agents tab, so status is visually distinguishable the same way it already is there. A test-run entry whose status is "passed" or "failed", an agent entry whose status is "done", or a reminder entry whose status is "finished" or "stopped", SHALL additionally show the elapsed duration of that run/task/reminder, computed from the most recent preceding "started" entry of the same category logged for the same project - for a reminder entry, additionally matched to the "started" entry recording the same reminder name, since a project can have more than one reminder interleaved in the log and an agent/test-run has no equivalent identity to disambiguate by. A reminder entry whose status is "started" SHALL additionally show its recorded ETA (the reminder's local-time due time, `h:MM AM/PM`, as recorded on that entry - see the activity-log capability), rather than the elapsed duration shown for other completed entries, since a just-started reminder has no elapsed run to show. An agent entry ("started", "done", or "needs input") that was recorded with a last-prompt snapshot SHALL additionally show a truncated, single-line snippet of that prompt text after its existing content, truncated with a trailing `...` rather than overflowing the row; an agent entry recorded with no last-prompt snapshot SHALL show no such snippet. Whenever the list holds more entries than fit on a single page, the Logs tab SHALL display a Ratatui vertical `Scrollbar` widget along its right edge, positioned so the scrollbar's thumb reaches the very bottom of its track when the list's last page is being shown (and the very top when its first page is being shown), and its heading SHALL append a pagination keyboard-shortcut hint alongside its existing sort and filter hints; neither the scrollbar nor the pagination hint SHALL be shown when every (filtered) entry already fits on a single page.

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
- **THEN** the row shows a truncated, single-line snippet of that prompt after its existing status/duration text, with a trailing `...` if it does not fit

#### Scenario: An agent entry with no recorded prompt shows no snippet
- **WHEN** the Logs tab renders an agent-category entry recorded with no last-prompt snapshot
- **THEN** the row shows no prompt snippet, rather than empty quotes or placeholder text

### Requirement: Reminders tab shows an aggregated, paginated reminder list
The Reminders tab SHALL display every project's reminders, aggregated across all projects, as a paginated list ordered by recency (the last time each reminder completed a run - stopped or naturally finished - most recent first) by default. Each row SHALL show the reminder's Project, Name, Duration (in minutes), Status, and Updated columns. The Status column SHALL be blank when the reminder has no started/stopped/finished log event yet (it is still not-yet-started). While a reminder is running, the Status column SHALL show its live elapsed duration and its estimated time of completion, formatted like "Running 1m23s, ETA: 12:34 PM", prefixed with the ⏳ marker, and kept current by the TUI's own periodic redraw, consistent with a running agent's or test run's duration. Once a reminder is done (stopped or finished), the Status column SHALL show its elapsed duration formatted like "Done 10m", prefixed with the ✅ marker. The Updated column SHALL show the time of the reminder's most recent started, stopped, or finished log event, or, if it has none, the time the reminder was created. Whenever the list holds more reminders than fit on a single page, the Reminders tab SHALL display a Ratatui vertical `Scrollbar` widget along its right edge, positioned so the scrollbar's thumb reaches the very bottom of its track when the list's last page is being shown (and the very top when its first page is being shown), and its heading SHALL append a pagination keyboard-shortcut hint alongside its existing sort and filter hints; neither the scrollbar nor the pagination hint SHALL be shown when every (filtered) reminder already fits on a single page.

#### Scenario: Reminders tab lists reminders across projects
- **WHEN** the user switches to the Reminders tab
- **THEN** the TUI displays reminders from every project, most recent first by their last completed run, split into pages

#### Scenario: A reminder with no runs shows a blank status
- **WHEN** the Reminders tab renders a reminder that is still not-yet-started
- **THEN** its Status column is blank

#### Scenario: A running reminder shows its live duration and ETA
- **WHEN** the Reminders tab renders a reminder whose status is running
- **THEN** its Status column shows the ⏳ marker, the current elapsed duration, and the estimated completion time (e.g. "⏳ Running 1m23s, ETA: 12:34 PM"), updating as the TUI redraws

#### Scenario: A done reminder shows its total duration
- **WHEN** the Reminders tab renders a reminder whose status is done
- **THEN** its Status column shows the ✅ marker and its elapsed duration (e.g. "✅ Done 10m")

#### Scenario: The Updated column falls back to creation time
- **WHEN** the Reminders tab renders a reminder with no started/stopped/finished log event
- **THEN** its Updated column shows the time the reminder was created

#### Scenario: A scrollbar appears once reminders overflow a page
- **WHEN** the Reminders tab's (filtered) reminders hold more rows than fit on a single page
- **THEN** the TUI renders a vertical scrollbar along the pane's right edge, reflecting the current page's position within the full list

#### Scenario: No scrollbar when everything fits on one page
- **WHEN** the Reminders tab's (filtered) reminders all fit within a single page
- **THEN** no scrollbar is rendered

### Requirement: Agents tab shows a Reminders column
The Agents tab SHALL display a fourth column, **Reminders**, alongside PROJECT, Agents, Tests, and UPDATED. A project with a running reminder SHALL show a status emoji per the "Status is visually distinguishable" requirement, then that reminder's name (bold), then its live elapsed duration and ETA, formatted like "⏳ **Check the thing** 1m23s, ETA 12:34 PM" - without the category-word prefix the Agents and Tests columns carry, per the "Status labels are prefixed by category outside dedicated panes" requirement's exception for this column. A project whose most recent reminder activity is a stop or a natural finish, with no reminder currently running, SHALL show its emoji, name, and outcome and duration, formatted like "✅ **Check the thing** completed/stopped, 10m". A project with two or more reminders with activity to show SHALL present them separated by a bullet, ordered by recency, the same convention the Agents column already uses for multiple agents. A project with no reminders SHALL leave the Reminders column empty. If either the Agents column's or the Reminders column's content exceeds its available column width, the TUI SHALL truncate it with a trailing `...` rather than wrapping or overflowing into adjacent columns.

#### Scenario: A project with a running reminder
- **WHEN** the Agents tab renders a project row with a running reminder
- **THEN** its Reminders column shows the reminder's name, its live elapsed duration, and its ETA, updating as the TUI redraws

#### Scenario: A project with a completed reminder and none running
- **WHEN** the Agents tab renders a project row whose most recent reminder activity is a stop or finish, with none currently running
- **THEN** its Reminders column shows that reminder's name, its outcome (completed or stopped), and its duration

#### Scenario: A project with multiple reminders
- **WHEN** a project has two or more reminders with activity to show
- **THEN** its Reminders column lists them bullet-separated, ordered by recency

#### Scenario: A project with no reminders leaves the column empty
- **WHEN** a project has no reminders
- **THEN** its Reminders column is empty

#### Scenario: Overflowing content is truncated with an ellipsis
- **WHEN** the Agents column's or the Reminders column's content would exceed its available column width
- **THEN** the TUI truncates that cell's content with a trailing `...` instead of wrapping or overflowing
