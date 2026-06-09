# CLAUDE.md

Read-only ratatui TUI browsing Claude Code auto-memories
(`~/.claude/projects/*/memory/*.md`). Never add a write path.

## Modules

- `src/memory.rs` — `scan()` walks the dir → flat `Library`. `parse_str()` is the
  filesystem-free parser (keep it that way for tests). `MEMORY.md` excluded.
- `src/app.rs` — `App` state. `current_memory_indices()` is the single place
  combining selected project + filter; both list and detail render off it.
  Project index `0` = synthetic "ALL PROJECTS".
- `src/ui.rs` — rendering + CRT palette, color by `type` via `type_color()`.
- `src/main.rs` — event loop + key handlers.

## Conventions

- Single dep: `ratatui` only. Use `ratatui::crossterm`; don't add a direct `crossterm` dep.
- No serde. Frontmatter parsing is hand-rolled and lenient — keep it tolerant of
  malformed/missing input, don't make it fail the parse.
- Keep parsing filesystem-free where possible (`parse_str` model).
- Scans sort dirs/files for stable ordering — preserve it.

## Test

`cargo test` (inline in `memory.rs`/`main.rs`). UI tests use `TestBackend`
headless. Scan tests run against real disk — must tolerate any existing memories.

## Gotchas

- `short_label()` is lossy (`/` and `.` both → `-`); not a reversible decode.
- Absent `~/.claude/projects` → empty library, not a panic.
