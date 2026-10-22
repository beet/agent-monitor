## MODIFIED Requirements

### Requirement: Agent registry
The daemon SHALL maintain a registry of known agents keyed by session identifier, recording working directory, host context (nvim, standalone terminal, or desktop app), process id, current status, last-updated timestamp, a status-since timestamp marking when the agent most recently entered its current status, a run-started timestamp marking when the agent most recently transitioned into "running" (or was registered for the first time already "running"), and the text of its most recently submitted user prompt, if any (its "last prompt"). Every one of these timestamps SHALL be set from the triggering event's reported `fired_at_ms` - the time its reporting client captured the underlying hook firing - rather than from the time the daemon happens to process that event. The registry SHALL reject a "needs input" event for a session whose current status is already "done", since a finished session cannot legitimately need input again until a new "running" event is reported for it. The registry SHALL hold at most one entry per pid: when an event's session id is new but its pid matches an existing entry under a different session id, the registry SHALL remove that existing entry before registering the new one, since the same pid starting a new session id (for example, via `/clear`) means the old session is gone, not merely quiet. The status-since timestamp SHALL only be set to the triggering event's `fired_at_ms` when an update actually changes the agent's status; an event or internal transition (e.g. marking an agent stale) that leaves the status unchanged SHALL leave the status-since timestamp untouched. The run-started timestamp SHALL be set to the triggering event's `fired_at_ms` when the entry is created, and reset to the new triggering event's `fired_at_ms` again every time the entry's status subsequently transitions to "running" from a different status - including a transition from "done", since one tracked agent's pid persists across many started-running-done turns over the life of a session, unlike a test run's pid-scoped run-start timestamp. The run-started timestamp SHALL remain unchanged by any transition that leaves "running" or that moves between two non-running statuses (into idle, needs input, done, declined, or stale), so it continues to mark the beginning of the most recently started run of work until superseded by the next transition into "running". Replacing a pid's entry this way SHALL permanently supersede the session id it replaced: a later event that names that superseded session id SHALL be treated as unknown rather than as a session the registry has not seen before, so it cannot be re-registered as a new entry and, by the same-pid rule above, cannot evict the live session that replaced it.

An event carries prompt text only when it originates from a `UserPromptSubmit` hook event; every other hook-derived event carries none. Before a `UserPromptSubmit` hook payload's prompt text becomes an event's prompt text, the reporting client SHALL remove Claude Code's own harness-injected wrapper tags from it, so a slash-command expansion, a system-reminder injection, or a backgrounded task's completion notification is never mistaken for literal human-typed text: a `<command-message>`, `<system-reminder>`, `<task-notification>`, or `<local-command-stdout>` tag SHALL be removed together with its entire contents (nested tags included), since none of these ever carry human-authored text; a `<command-name>` or `<command-args>` tag SHALL have only its surrounding tag markup removed, keeping its contents, since these carry the literal slash command and arguments the user typed. Whatever text remains after this removal - trimmed of leading/trailing whitespace - becomes the event's prompt text. When nothing remains (only whitespace, or nothing at all), the event SHALL carry no prompt text, the same as an event derived from a hook that never carries one. A payload with no recognized wrapper tags is unaffected by this removal and becomes the event's prompt text exactly as submitted, as before. An event that carries prompt text SHALL set the agent's last prompt to that text, regardless of what status the event also reports. An event that carries no prompt text SHALL leave the agent's existing last prompt, if any, unchanged - it SHALL NOT be cleared just because a later event (a tool call, a completion, a needs-input prompt) reports no new prompt of its own. A newly created entry - whether the first event seen for a session, or a same-pid replacement per the rule above - SHALL start with no last prompt unless the triggering event itself carries one; a replacement entry SHALL NOT inherit the last prompt of the session id it replaced. Last-prompt text longer than 4,000 characters SHALL be truncated to that length, with a trailing truncation marker, so one agent's entry cannot grow unbounded regardless of how large a prompt the user submits.

#### Scenario: First event for a session registers a new agent
- **WHEN** the daemon receives an event for a session id it has not seen before, and its pid does not match any existing entry
- **THEN** it creates a new registry entry with the reported working directory, host context, pid, and status, and sets the last-updated, status-since, and run-started timestamps to that event's reported `fired_at_ms`

#### Scenario: Subsequent event updates the existing agent
- **WHEN** the daemon receives an event for a session id already in the registry
- **THEN** it updates that entry's status and sets its last-updated timestamp to that event's reported `fired_at_ms`, instead of creating a duplicate

#### Scenario: A same-status event does not reset the status-since timestamp
- **WHEN** the daemon receives an event for a session already in the registry and the event's status matches the agent's current status
- **THEN** the daemon updates the last-updated timestamp to that event's `fired_at_ms` but leaves the status-since timestamp unchanged

#### Scenario: A status transition resets the status-since timestamp
- **WHEN** the daemon receives an event for a session already in the registry and the event's status differs from the agent's current status
- **THEN** the daemon sets the status-since timestamp to that event's `fired_at_ms`, in addition to updating the status and last-updated timestamp

#### Scenario: A transition into running resets the run-started timestamp
- **WHEN** the daemon receives a "running" event for a session whose current status differs from "running" (idle, needs input, done, declined, or stale)
- **THEN** the daemon sets that entry's run-started timestamp to that event's `fired_at_ms`, marking the beginning of this new run, in addition to updating status and status-since timestamp

#### Scenario: A transition away from running leaves the run-started timestamp unchanged
- **WHEN** the daemon receives an event that transitions a "running" session to idle, needs input, done, declined, or stale
- **THEN** the daemon leaves that entry's run-started timestamp unchanged, so it still marks when the run that just ended began

#### Scenario: A late needs-input event cannot override a completed session
- **WHEN** the daemon receives a "needs input" event for a session whose current status is already "done"
- **THEN** the daemon leaves that agent's status as "done" and does not apply the "needs input" event, and does not change its status-since or run-started timestamp

#### Scenario: A new running event still clears a completed session's status
- **WHEN** the daemon receives a "running" event for a session whose current status is "done"
- **THEN** it updates that entry's status to "running" as normal, and resets both the status-since timestamp and the run-started timestamp to that event's `fired_at_ms`, since this begins a new run

#### Scenario: A new session id for an already-tracked pid replaces the old entry
- **WHEN** the daemon receives an event for a session id it has not seen before, and its pid matches an existing entry registered under a different session id
- **THEN** the daemon removes the existing entry for that pid and registers the new session id as the sole entry for that pid, with last-updated, status-since, and run-started timestamps all set to that event's reported `fired_at_ms` rather than inherited from the entry it replaced

#### Scenario: A late event for an already-superseded session id is ignored
- **WHEN** the daemon receives an event naming a session id that was previously removed because another session id took over its pid, and that replacement session id is still the pid's tracked entry
- **THEN** the daemon does not register the superseded session id as a new entry and does not remove or otherwise alter the pid's current entry

#### Scenario: A late event for a superseded session id whose pid has since moved on again
- **WHEN** the daemon receives an event naming a session id that was superseded, and the pid it belonged to is now tracked under yet another, different session id
- **THEN** the daemon still does not register the superseded session id as a new entry, and the pid's currently tracked entry is unaffected

#### Scenario: A prompt-submission event records the last prompt
- **WHEN** the daemon receives an event derived from a `UserPromptSubmit` hook event, carrying prompt text, for a tracked session
- **THEN** the daemon sets that entry's last prompt to the reported text

#### Scenario: A non-prompt-bearing event leaves the last prompt unchanged
- **WHEN** the daemon receives an event for an already-tracked session that carries no prompt text (e.g. derived from `PreToolUse`, `PostToolUse`, or `Stop`)
- **THEN** the entry's existing last prompt, if any, is left unchanged

#### Scenario: A same-pid replacement does not inherit the previous session's prompt
- **WHEN** the daemon registers a new session id that replaces an existing entry sharing its pid (per the same-pid dedup rule), and the triggering event carries no prompt text
- **THEN** the new entry has no last prompt, even though the entry it replaced had one

#### Scenario: An overlong prompt is truncated
- **WHEN** the daemon receives a prompt-bearing event whose prompt text exceeds 4,000 characters
- **THEN** the daemon stores only the first 4,000 characters of it, followed by a trailing truncation marker

#### Scenario: A task-notification wrapper with no human text carries no prompt
- **WHEN** a `UserPromptSubmit` hook payload's prompt text is entirely a `<task-notification>` tag (with a `<task-id>` and other nested tags inside it) and no text outside it
- **THEN** the reported event carries no prompt text, and the agent's existing last prompt (if any) is left unchanged, the same as for any other event that carries no prompt text

#### Scenario: A system-reminder injection is removed, leaving the user's own text
- **WHEN** a `UserPromptSubmit` hook payload's prompt text is a `<system-reminder>` tag followed by text the user actually typed
- **THEN** the reported event's prompt text is only the text the user typed, with the `<system-reminder>` tag and everything inside it removed

#### Scenario: A slash command's name and arguments survive, unwrapped
- **WHEN** a `UserPromptSubmit` hook payload's prompt text is a `<command-message>` tag followed by a `<command-name>` tag and a `<command-args>` tag
- **THEN** the reported event's prompt text is the `<command-name>` tag's contents followed by the `<command-args>` tag's contents, with the `<command-message>` tag and its contents removed and all three tags' own markup stripped

#### Scenario: A plain prompt with no recognized wrapper tags is unaffected
- **WHEN** a `UserPromptSubmit` hook payload's prompt text contains none of the recognized wrapper tags
- **THEN** the reported event's prompt text is exactly the submitted text, as before this change
