# CLAUDE.md

Ratatui TUI for browsing and mutating Claude Code auto-memories
(`~/.claude/projects/*/memory/*.md`).

## Modules

- `src/memory.rs` — `scan()` walks the dir → flat `Library`. `parse_str()` is the
  filesystem-free parser (keep it that way for tests). MEMORY.md is included as
  type `index` with a fixed identity.
- `src/mutate.rs` — the only write path. Every mutation: backup first
  (`~/.claude/auto-memories-backups/`), atomic write (temp + rename), and keep
  MEMORY.md in sync on insert/delete/move. Keep all new writes behind these rules.
- `src/app.rs` — `App` state + `Mode` enum (Normal/Filter/ConfirmDelete/NewSlug/
  MoveProject/PickType). `current_memory_indices()` is the single place combining
  selected project + filter; ALL view (project index `0`) hides `index` entries.
- `src/ui.rs` — rendering + CRT palette; `type_color()` per type; stats strip in
  the memories pane; index bodies parsed via `parse_index_line()`.
- `src/main.rs` — event loop, per-mode key handlers, `$EDITOR` suspend/resume
  (`run_editor`: `ratatui::restore()` → spawn → `ratatui::init()`).

## Conventions

- Single dep: `ratatui` only. Use `ratatui::crossterm`; don't add a direct `crossterm` dep.
- No serde, no chrono. Frontmatter parsing and date formatting are hand-rolled;
  keep parsers lenient — tolerate malformed/missing input, never fail the parse.
- Mutations must go through `mutate.rs` (backup + atomic write + index sync).
- `index` entries are edit-only: block delete/move/retype on `mtype == "index"`.
- Scans sort dirs/files for stable ordering — preserve it.

## Test

`cargo test` — inline `#[cfg(test)]` in every module. UI tests use `TestBackend`
headless; mutation tests use per-test temp dirs; scan tests run against real
disk and must tolerate whatever memories exist. TUI behavior is verified
manually by Alex — don't try to drive the TUI through a pty in CI or locally.

## Releasing

Every time a change ships (feature or fix), cut a release:

1. Bump `version` in `Cargo.toml` (and let `Cargo.lock` update) — the crate
   version must stay in lockstep with the tag.
2. Commit and push.
3. `git tag vX.Y.Z && git push origin vX.Y.Z` — the tag triggers
   `.github/workflows/release.yml` (build → verify → publish macOS arm64 tarball).
4. Watch the run to green (`gh run watch`) and confirm the release asset exists
   (`gh release view vX.Y.Z`).

## Gotchas

- `short_label()` is lossy (`/` and `.` both → `-`); not a reversible decode.
- Absent `~/.claude/projects` → empty library, not a panic.
- "created" is file birthtime (APFS), not when the memory was learned; display
  is UTC since std can't get the local offset.
- Release: pushing a `v*` tag builds + publishes a macOS arm64 binary
  (`.github/workflows/release.yml`).
