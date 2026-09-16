## MODIFIED Requirements

### Requirement: Live agent list
The TUI SHALL display tracked agents and test runs grouped by working directory as one row per project: each distinct working directory forms exactly one row showing that project's name (derived from the working directory), its tracked agent(s)' status(es) in an Agents cell, that project's test run status (if any) in a Tests cell, and the most recent last-updated time among those members (in the system's local timezone, formatted `%Y-%m-%d %H:%M:%S`), updating as the daemon reports changes. A project row SHALL NOT display per-agent host context or process id as separate columns; the daemon continues tracking that data, it is simply not rendered in the collapsed row. Project rows SHALL be ordered by the most recent last-updated time of any member (agent or test run) within them, most recent first. An agent whose status is "running" SHALL contribute a live, counting-up duration to the row, computed from the current time minus that agent's status-since timestamp, formatted as a compact counter (e.g. `2m14s`) and kept current by the TUI's own periodic redraw rather than only refreshing when the daemon pushes an update. An agent whose status is "done" SHALL contribute a fixed duration to the row, computed from that agent's run-started timestamp (the beginning of the run that just completed) to its status-since timestamp (the time it completed) - unlike the "running" duration, this fixed duration does not change on further redraws. An agent in any other status (idle, needs input, stale, or declined) SHALL NOT contribute a duration. A test run whose status is "running" SHALL likewise contribute a live, counting-up duration computed from its run-start timestamp. Once a test run reaches "passed" or "failed", its row SHALL continue to show a duration - the total elapsed time from its run-start timestamp to its last-updated timestamp - rather than showing no duration. When the daemon pushes a removal for a session id, the TUI SHALL drop that session id from its tracked agents, so a pid whose session id has been superseded contributes at most one entry - the current, live session's - to its project row's Agents cell and to that project's Agents pane in the details modal, never a second, frozen entry left over from the superseded session.

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
- **THEN** the row includes the most recent last-updated time among its members, converted to the system's local timezone and formatted as `%Y-%m-%d %H:%M:%S` (e.g. `2026-09-01 16:32:07`), without a UTC offset or timezone code

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

### Requirement: Status is visually distinguishable
The TUI SHALL visually distinguish agent statuses (e.g. running, idle, needs input, done, stale, declined) and test-run statuses (running, passed, failed) from one another so the user can scan the list and immediately identify agents or test runs needing attention. Each agent status SHALL be prefixed with a distinct emoji marker in addition to any color/style distinction: running with 🔧, idle with 💤, needs input with 🔔, done with ✅, stale with 👻, and declined with 🚫. Each test-run status SHALL be prefixed with a distinct emoji marker: running with ⏳, passed with ✅, and failed with ❌. When a project's Agents cell reflects more than one tracked agent in different statuses, the cell SHALL show each distinct status present rather than collapsing them into a single "winning" status, so no status needing attention is hidden behind another. On the currently-selected row in the Agents tab or the Logs tab, the TUI SHALL override every status's own foreground color with a single fixed foreground color chosen to stay legible against the row-highlight background, rather than requiring the row-highlight background to avoid every status's own color.

#### Scenario: An agent needs input
- **WHEN** an agent's status is "needs input"
- **THEN** that status's contribution to the row displays the 🔔 marker and is visually distinguished (e.g. color) from other statuses shown, using bold colored text rather than a solid background fill

#### Scenario: Each status has a distinct emoji marker
- **WHEN** the TUI renders an agent's status within a project row
- **THEN** that status is prefixed with the emoji for that status (🔧 running, 💤 idle, 🔔 needs input, ✅ done, 👻 stale, 🚫 declined)

#### Scenario: The stale marker renders full-width
- **WHEN** the TUI renders an agent's "stale" status
- **THEN** it uses the 👻 marker rather than 🕸️, since 👻 renders reliably full-width across terminals and does not corrupt the row-highlight background the way 🕸️ did

#### Scenario: A declined permission is visually distinguished
- **WHEN** an agent's status is "declined"
- **THEN** that status's contribution to the row displays the 🚫 marker and is visually distinguished (e.g. color) from other statuses shown

#### Scenario: A failing test run is visually distinguished
- **WHEN** a test run's status is "failed"
- **THEN** that status's contribution to the row displays the ❌ marker and is visually distinguished (e.g. color) from other statuses shown

#### Scenario: Each test-run status has a distinct emoji marker
- **WHEN** the TUI renders a test run's status within a project row
- **THEN** that status is prefixed with the emoji for that status (⏳ running, ✅ passed, ❌ failed)

#### Scenario: A project with multiple agent statuses shows each one
- **WHEN** a project has two or more tracked agents whose statuses differ from one another
- **THEN** the row's Agents cell lists each distinct status present among those agents (for example, both running and needs input), rather than showing only one

#### Scenario: An agent status and a test-run status are shown together
- **WHEN** a project has both a tracked agent and a test run whose statuses differ
- **THEN** the row's Agents cell shows the agent's status and the row's Tests cell shows the test run's status, each in its own column rather than sharing one

#### Scenario: A running status stays legible on the selected row
- **WHEN** a project row showing the "running" status is the currently-selected (highlighted) row in the Agents tab
- **THEN** the status's label and emoji remain visible, rendered in the row's fixed selected-row foreground color rather than the status's own color

#### Scenario: A selected row's text overrides every status's own color
- **WHEN** any row in the Agents tab or the Logs tab is the currently-selected (highlighted) row, regardless of which status or statuses it shows
- **THEN** all of that row's status text renders in the same fixed selected-row foreground color, rather than in each status's own color

### Requirement: Status labels are prefixed by category outside dedicated panes
In the Agents tab and the Logs tab, each status label SHALL be prefixed with a category word - "agent" for an agent status or "tests" for a test-run status - immediately after its emoji marker and before the status word (e.g. "✅ agent done", "⏳ tests started"), so a label read in a cross-category view is unambiguous about which kind of status it names. In the details modal, the Agents pane and Tests pane SHALL NOT include this category-word prefix, since the pane itself already establishes the category (e.g. "✅ done", "❌ failed"); the modal's Logs pane SHALL include the category-word prefix, consistent with the top-level Logs tab it mirrors.

#### Scenario: The Agents tab prefixes an agent status with its category word
- **WHEN** the Agents tab renders an agent's status in its Agents column
- **THEN** the label reads the emoji, then "agent", then the status word (e.g. "✅ agent done"), rather than omitting the category word

#### Scenario: The Agents tab prefixes a test-run status with its category word
- **WHEN** the Agents tab renders a project's test-run status in its Tests column
- **THEN** the label reads the emoji, then "tests", then the status word (e.g. "⏳ tests running"), rather than omitting the category word

#### Scenario: The Logs tab prefixes entries with their category word
- **WHEN** the Logs tab renders an entry
- **THEN** the label includes the category word ("agent" or "tests") alongside its emoji and status, consistent with the Agents tab

#### Scenario: The details modal's Agents and Tests panes omit the category-word prefix
- **WHEN** the details modal renders a status in its Agents pane or Tests pane
- **THEN** the label omits the category word (e.g. "✅ done", not "✅ agent done")

#### Scenario: The details modal's Logs pane keeps the category-word prefix
- **WHEN** the details modal renders an entry in its Logs pane
- **THEN** the label includes the category word, consistent with the top-level Logs tab
