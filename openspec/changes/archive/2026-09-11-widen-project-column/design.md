## Context

`render_agent_table` in `crates/agentmon/src/ui.rs` builds a ratatui `Table` with a fixed `widths` array: `[Constraint::Length(20), Constraint::Fill(1), Constraint::Length(19)]` for PROJECT, STATUS, UPDATED. `Constraint::Length` is an unconditional fixed width; ratatui's `Table` clips cell content that overflows its column's resolved width, with no ellipsis. See proposal.md - Why.

## Goals / Non-Goals

**Goals:**
- Let the PROJECT column grow on wider terminals so typical project names (well beyond 20 characters) render in full in common terminal widths.
- Keep STATUS as the primary claimant of extra space, since it can carry several status segments plus a test-run segment joined by " · ".

**Non-Goals:**
- Adding an explicit truncation indicator (e.g. ellipsis) for names that still don't fit - out of scope, existing clipping behavior is unchanged for the narrow-terminal case.
- Making column widths user-configurable.

## Decisions

**Use `Constraint::Fill` for both PROJECT and STATUS, weighted so STATUS still gets more room.** Ratatui resolves `Fill(n)` constraints by giving each a share of the space left over after fixed (`Length`/`Min`/`Percentage`) constraints are satisfied, proportional to `n`. Changing PROJECT from `Constraint::Length(20)` to `Constraint::Fill(2)` and STATUS from `Constraint::Fill(1)` to `Constraint::Fill(3)` keeps UPDATED's fixed `Length(19)`, and splits the remaining width 2:3 between PROJECT and STATUS - e.g. on a 100-column terminal (minus borders/padding), PROJECT gets roughly 2/5 of the leftover space instead of a hard 20-character cap, while STATUS keeps the larger share.

Alternative considered: `Constraint::Min(20)` for PROJECT (grow only when nothing else needs the space). Rejected because `Min` constraints yield space to `Fill` constraints last, so PROJECT would only widen when STATUS has no more use for space - in practice STATUS is `Fill`-based and would consume all slack, leaving PROJECT stuck at 20 in most real layouts. A `Fill` weight guarantees PROJECT actually grows as the terminal widens.

Alternative considered: swap the fixed cap for a larger fixed cap (e.g. `Length(40)`). Rejected because it doesn't scale with terminal width - it just moves the truncation point, which doesn't address "we have a lot more screen real estate" on very wide terminals, and wastes space on narrow ones.

## Risks / Trade-offs

- [On very narrow terminals, PROJECT's proportional share may be smaller than 20 characters, clipping names that previously fit] → Acceptable: the spec's narrow-terminal scenario already allows clipping when space is insufficient, and this only affects unusually narrow terminals.
