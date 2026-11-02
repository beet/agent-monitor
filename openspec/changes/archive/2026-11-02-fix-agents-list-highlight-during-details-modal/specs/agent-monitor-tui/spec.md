## MODIFIED Requirements

### Requirement: Agents tab supports row selection
The Agents tab SHALL support moving a selection cursor over its project rows using the `j`/`down` (next row) and `k`/`up` (previous row) keys, so a specific project can be chosen for its details view. The selection SHALL be visually distinguished from unselected rows.

While the details modal is open, the Agents tab's selection SHALL continue to identify the same project the modal is showing, even if a background update (a new agent, an agent status change, an agent's removal, or a test-run update) changes the recency-sort order of the Agents tab's rows while the modal is open. If that re-sort moves the modal's project to a different row, the selection SHALL move with it rather than staying at the same row index, so closing the modal leaves the highlight on the project the user was just viewing. If the modal's project is no longer present among the Agents tab's visible rows (for example, it has no more tracked members, or is excluded by an applied search filter), the selection SHALL fall back to being clamped to the visible rows' length, as it is when no modal is open.

#### Scenario: Moving the selection down
- **WHEN** the user presses `j` or the down arrow while the Agents tab is active
- **THEN** the selection cursor moves to the next project row, if one exists

#### Scenario: Moving the selection up
- **WHEN** the user presses `k` or the up arrow while the Agents tab is active
- **THEN** the selection cursor moves to the previous project row, if one exists

#### Scenario: A background update reorders the list while the details modal is open
- **WHEN** the details modal is open for a project, and a background update (to a different project's agent or test run) changes the Agents tab's recency-sort order such that the modal's project would move to a different row index
- **THEN** the Agents tab's selection moves to that new row index, so it still identifies the modal's project

#### Scenario: Closing the modal after a reorder leaves the right row highlighted
- **WHEN** the details modal is open for a project, a background update reorders the Agents tab while it is open, and the user then closes the modal
- **THEN** the Agents tab shows that same project's row as selected, not whichever row now occupies the index that was selected before the modal opened

#### Scenario: The modal's project drops out of the visible list while the modal is open
- **WHEN** the details modal is open for a project, and that project is no longer present among the Agents tab's visible (filtered) rows while the modal is open
- **THEN** the Agents tab's selection is clamped to the length of the visible rows, the same as when no modal is open
