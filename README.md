# auto-memories

CRT-styled TUI for browsing and managing Claude Code's auto-memory files
(`~/.claude/projects/*/memory/*.md`).

```
projects | memories (+ scope stats)
-----------------------------------
detail (full width)
```

## Install

```sh
# from a release (Apple Silicon)
curl -sL https://github.com/alehlopeh/auto-memories/releases/latest/download/auto-memories-aarch64-apple-darwin.tar.gz | tar xz
mv auto-memories-aarch64-apple-darwin/auto-memories ~/bin/

# or build from source
cargo install --git https://github.com/alehlopeh/auto-memories
```

## Keys

| Key | Action |
| --- | --- |
| `j`/`k`, `↓`/`↑` | Move in focused pane |
| `h`/`l`, `←`/`→`, `Tab` | Switch pane |
| `PageUp`/`PageDown` | Scroll detail |
| `/` | Filter (name, description, slug, type, body) |
| `Esc` | Clear filter / cancel prompt |
| `e` | Edit in `$EDITOR` |
| `n` | New memory (asks for slug; needs a project selected) |
| `d` | Delete memory — or the whole project when the projects pane is focused (with confirm) |
| `m` | Move to another project |
| `t` | Change type (user/feedback/project/reference) |
| `o` | Reveal memory file in Finder — or open the project's memory dir when the projects pane is focused |
| `r` | Re-scan |
| `q` | Quit |

## Behavior

- First project entry is a synthetic **ALL PROJECTS** flat view (hides index files).
- Each project's `MEMORY.md` shows as a type-`index` entry, rendered as a parsed
  ledger; it can only be edited, not deleted/moved/retyped.
- Memories are colored by `type`: feedback=amber, project=cyan, reference=green,
  user=magenta, index=white, unknown=grey.
- Stats strip in the memories pane: totals by type, created in last 7/30 days,
  oldest → newest. Timestamps are file birthtime/mtime, UTC.

## Safety

- Every destructive op (edit/delete/move/retype) first copies the file to
  `~/.claude/auto-memories-backups/<project>/<file>.<millis>`.
- All writes are atomic (temp file + rename) — Claude Code may read these live.
- Insert/delete/move keep the project's `MEMORY.md` index in sync.

## Layout

| File | Job |
| --- | --- |
| `src/memory.rs` | Scan + parse frontmatter/body |
| `src/mutate.rs` | Backups, atomic writes, index sync, mutations |
| `src/app.rs` | State, selection, filtering, modes |
| `src/ui.rs` | CRT rendering |
| `src/main.rs` | Event loop, keys, `$EDITOR` integration |

## Notes

- Single dependency: `ratatui` 0.29 (use its re-exported `crossterm`).
- Hand-rolled, lenient frontmatter parser — no serde. Handles flat and `metadata:`-nested `type:`.
- Project labels are lossy: mangling replaces `/` and `.` with `-`, so `short_label()` is a best-effort `-code-` heuristic, not reversible.

## Test

```sh
cargo test
```
