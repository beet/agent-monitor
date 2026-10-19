## MODIFIED Requirements

### Requirement: Navigation and quit do not affect tracked agents
The TUI SHALL support quitting the application via the `q` key, and quitting the TUI SHALL NOT stop the daemon or any tracked Claude Code agent. `q` SHALL quit immediately without confirmation, from anywhere it is pressed (including while the quit-confirmation dialog described below is open), unless the user is currently editing free-text input - the Agents tab's `/`-search prompt, the project rename form, or the reminder creation/editing form - in which case `q` is treated as that input's literal text instead of quitting; this is unchanged from the search prompt's and rename form's existing precedence over every other key binding, and the reminder form now follows the same rule.

When no modal, form, dialog, or search prompt is open, pressing `Esc` SHALL open a quit-confirmation dialog instead of quitting immediately. While the quit-confirmation dialog is open, pressing `Enter` SHALL quit the TUI the same way `q` does, and pressing `Esc` SHALL dismiss the dialog and return to the active tab without quitting or altering any agent, test-run, or reminder state. This top-level `Esc` behavior change SHALL NOT alter `Esc`'s existing behavior anywhere else it is already bound - closing the details modal, the help modal, the reminder creation/editing form, the delete-reminder confirmation dialog, or cancelling the Agents-tab search prompt all continue to take priority over opening the quit-confirmation dialog, since in each of those contexts a modal, form, dialog, or search prompt is already open.

#### Scenario: User quits the TUI
- **WHEN** the user presses `q`
- **THEN** the TUI process exits while the daemon keeps running and continues tracking agents

#### Scenario: q is treated as literal text in the reminder form
- **WHEN** the reminder creation/editing form is open and the user presses `q` while the Name field has focus
- **THEN** `q` is appended to the Name field and the TUI keeps running, rather than quitting

#### Scenario: Esc at the top level opens a quit-confirmation dialog instead of quitting
- **WHEN** the user presses `Esc` while no modal, form, dialog, or search prompt is open
- **THEN** the TUI opens a quit-confirmation dialog and remains running

#### Scenario: Confirming the quit dialog exits the TUI
- **WHEN** the quit-confirmation dialog is open and the user presses `Enter`
- **THEN** the TUI process exits while the daemon keeps running and continues tracking agents

#### Scenario: Cancelling the quit dialog keeps the TUI running
- **WHEN** the quit-confirmation dialog is open and the user presses `Esc`
- **THEN** the dialog closes, the TUI returns to the tab that was active before `Esc` opened it, and no agent, test-run, or reminder state is altered

#### Scenario: q bypasses the quit-confirmation dialog
- **WHEN** the quit-confirmation dialog is open and the user presses `q`
- **THEN** the TUI process exits immediately, the same as pressing `q` anywhere else

#### Scenario: Esc still closes other overlays instead of opening the quit-confirmation dialog
- **WHEN** the user presses `Esc` while the details modal, the help modal, the reminder form, the delete-reminder confirmation dialog, or the Agents-tab search prompt is open
- **THEN** that modal, form, dialog, or search prompt closes or cancels as it already did, and the quit-confirmation dialog does not open
