# auto-memories

Read-only, CRT-styled TUI for browsing Claude Code's auto-memory files
(`~/.claude/projects/*/memory/*.md`). Never writes to disk.

```
DRIVES (projects)   CLIPS (memory list)   PLAYBACK (detail)
```

## Run

```sh
cargo run --release
```

Reads `$HOME/.claude/projects/*/memory/*.md`. Missing dir → empty library, not an error.

## Keys

| Key | Action |
| --- | --- |
| `j`/`k`, `↓`/`↑` | Move in focused pane |
| `h`/`l`, `←`/`→` | Focus projects / memories |
| `Tab` | Toggle pane |
| `PageUp`/`PageDown` | Scroll detail |
| `/` | Filter (name, description, slug, type, body) |
| `Esc` | Clear filter |
| `r` | Re-scan |
| `q` | Quit |

First project entry is a synthetic **ALL PROJECTS** flat view. `MEMORY.md` index files are excluded.

Memories are colored by `type`: feedback=amber, project=cyan, reference=green, user=magenta, unknown=grey.

## Layout

| File | Job |
| --- | --- |
| `src/memory.rs` | Scan + parse frontmatter/body |
| `src/app.rs` | State, selection, filtering |
| `src/ui.rs` | CRT rendering |
| `src/main.rs` | Event loop + keys |

## Notes

- Single dependency: `ratatui` 0.29 (use its re-exported `crossterm`).
- Hand-rolled, lenient frontmatter parser — no serde. Handles flat and `metadata:`-nested `type:`.
- Project labels are lossy: mangling replaces `/` and `.` with `-`, so `short_label()` is a best-effort `-code-` heuristic, not reversible.

## Test

```sh
cargo test
```
