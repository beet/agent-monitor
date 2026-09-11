## ADDED Requirements

### Requirement: Project name column scales with terminal width
The TUI's PROJECT column SHALL NOT be capped at a fixed 20-character width. Its width SHALL scale with the terminal's available width, growing on wider terminals instead of always truncating project names at the same fixed length. The STATUS column SHALL continue to receive the majority of any additional space beyond what PROJECT and UPDATED need, since status segments are typically the widest content in a row. The UPDATED column's width is unaffected by this requirement.

#### Scenario: A wide terminal shows more of a long project name
- **WHEN** the TUI renders in a terminal wide enough to fit a project name longer than 20 characters alongside the STATUS and UPDATED columns
- **THEN** the PROJECT column renders more than 20 characters of that name, rather than clipping it at 20

#### Scenario: A narrow terminal still clips names that don't fit
- **WHEN** the TUI renders in a terminal too narrow to fit a project name in full alongside the STATUS and UPDATED columns
- **THEN** the PROJECT column clips the name to the space available, consistent with existing table-rendering behavior
