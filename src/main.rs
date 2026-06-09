//! auto-memories — a CRT-styled browser and editor for Claude Code's
//! auto-memory files (`~/.claude/projects/*/memory/*.md`).

mod app;
mod memory;
mod mutate;
mod ui;

use std::io;
use std::process::Command;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};

use app::{App, EditorRequest, Focus, Mode, TYPES};

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new();

    let result = run(&mut terminal, &mut app);

    ratatui::restore();
    result
}

fn run(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
) -> io::Result<()> {
    while !app.should_quit {
        terminal.draw(|f| ui::render(f, app))?;

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        app.status = None;
        match app.mode {
            Mode::Normal => handle_nav_key(app, key.code),
            Mode::Filter => handle_filter_key(app, key.code),
            Mode::ConfirmDelete => handle_confirm_key(app, key.code),
            Mode::ConfirmDeleteProject => handle_confirm_project_key(app, key.code),
            Mode::NewSlug => handle_slug_key(app, key.code),
            Mode::MoveProject => handle_move_key(app, key.code),
            Mode::PickType => handle_type_key(app, key.code),
        }

        if let Some(req) = app.pending_editor.take() {
            run_editor(terminal, app, req)?;
        }
    }
    Ok(())
}

/// Suspend the TUI, open `$EDITOR` on the file, resume, rescan.
fn run_editor(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    req: EditorRequest,
) -> io::Result<()> {
    if !req.is_new {
        if let Err(e) = mutate::backup(&req.path) {
            app.status = Some(format!("backup failed, not editing: {e}"));
            return Ok(());
        }
    }

    ratatui::restore();
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    // sh -c '<editor> "$0"' <path> — lets $EDITOR carry its own flags.
    let status = Command::new("sh")
        .arg("-c")
        .arg(format!("{editor} \"$0\""))
        .arg(&req.path)
        .status();
    *terminal = ratatui::init();
    terminal.clear()?;

    match status {
        Ok(s) if s.success() => {
            if req.is_new {
                finish_new_memory(app, &req);
            } else {
                app.status = Some(format!("edited {}", req.path.display()));
            }
        }
        Ok(_) => {
            if req.is_new {
                // Editor aborted (e.g. :cq) — discard the unsaved template.
                let _ = std::fs::remove_file(&req.path);
                app.status = Some("new memory aborted".to_string());
            } else {
                app.status = Some("editor exited non-zero".to_string());
            }
        }
        Err(e) => app.status = Some(format!("failed to launch {editor}: {e}")),
    }

    app.rescan();
    Ok(())
}

/// A freshly created memory gets its pointer line appended to MEMORY.md.
fn finish_new_memory(app: &mut App, req: &EditorRequest) {
    if !req.path.exists() {
        app.status = Some("new memory discarded".to_string());
        return;
    }
    if let Some(m) = memory::parse_file(&req.path, "", "") {
        match mutate::index_append(&req.mem_dir, &m.slug, &m.name, &m.description) {
            Ok(()) => app.status = Some(format!("created {} + indexed", m.slug)),
            Err(e) => app.status = Some(format!("created, but index update failed: {e}")),
        }
    }
}

/// Keys while the search prompt is active.
fn handle_filter_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.filter.clear();
            app.clamp_selection();
        }
        KeyCode::Enter => {
            app.mode = Mode::Normal; // keep the filter, just leave input mode
        }
        KeyCode::Backspace => {
            app.filter.pop();
            app.clamp_selection();
        }
        KeyCode::Char(c) => {
            app.filter.push(c);
            app.clamp_selection();
        }
        _ => {}
    }
}

/// Keys during normal navigation.
fn handle_nav_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Esc if !app.filter.is_empty() => {
            app.filter.clear();
            app.clamp_selection();
        }
        KeyCode::Char('/') => {
            app.mode = Mode::Filter;
            app.filter.clear();
        }
        KeyCode::Char('r') => app.rescan(),

        // Mutations.
        KeyCode::Char('e') => {
            if let Some(m) = app.selected_memory() {
                let mem_dir = m.path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
                app.pending_editor = Some(EditorRequest {
                    path: m.path,
                    is_new: false,
                    mem_dir,
                });
            }
        }
        KeyCode::Char('n') => {
            if app.selected_project == 0 {
                app.status = Some("select a project first (n needs a target)".to_string());
            } else {
                app.mode = Mode::NewSlug;
                app.input.clear();
            }
        }
        // In the projects pane `d` targets the whole project.
        KeyCode::Char('d') if app.focus == Focus::Projects => {
            if app.selected_project == 0 {
                app.status = Some("select a project, not ALL".to_string());
            } else {
                app.mode = Mode::ConfirmDeleteProject;
            }
        }
        KeyCode::Char('d') => match app.selected_memory() {
            Some(m) if m.mtype == "index" => {
                app.status = Some("the index can only be edited".to_string());
            }
            Some(_) => app.mode = Mode::ConfirmDelete,
            None => {}
        },
        KeyCode::Char('m') => match app.selected_memory() {
            Some(m) if m.mtype == "index" => {
                app.status = Some("the index can only be edited".to_string());
            }
            Some(_) if app.library.projects.len() < 2 => {
                app.status = Some("no other project to move to".to_string());
            }
            Some(_) => {
                app.mode = Mode::MoveProject;
                app.move_cursor = 1;
            }
            None => {}
        },
        KeyCode::Char('t') => match app.selected_memory() {
            Some(m) if m.mtype == "index" => {
                app.status = Some("the index can only be edited".to_string());
            }
            Some(m) => {
                app.mode = Mode::PickType;
                app.type_cursor = TYPES.iter().position(|t| *t == m.mtype).unwrap_or(0);
            }
            None => {}
        },

        // Pane switch.
        KeyCode::Tab | KeyCode::BackTab => {
            app.focus = match app.focus {
                Focus::Projects => Focus::Memories,
                Focus::Memories => Focus::Projects,
            };
        }
        KeyCode::Left | KeyCode::Char('h') => app.focus = Focus::Projects,
        KeyCode::Right | KeyCode::Char('l') => app.focus = Focus::Memories,

        // Vertical movement in the focused pane.
        KeyCode::Down | KeyCode::Char('j') => match app.focus {
            Focus::Projects => app.next_project(),
            Focus::Memories => app.next_memory(),
        },
        KeyCode::Up | KeyCode::Char('k') => match app.focus {
            Focus::Projects => app.prev_project(),
            Focus::Memories => app.prev_memory(),
        },

        // Detail scroll.
        KeyCode::PageDown => app.scroll_detail_down(),
        KeyCode::PageUp => app.scroll_detail_up(),

        _ => {}
    }
}

/// y/n on the delete prompt.
fn handle_confirm_key(app: &mut App, code: KeyCode) {
    if code == KeyCode::Char('y') {
        if let Some(m) = app.selected_memory() {
            match mutate::delete_memory(&m.path, &m.slug) {
                Ok(()) => app.status = Some(format!("deleted {} (backed up)", m.slug)),
                Err(e) => app.status = Some(format!("delete failed: {e}")),
            }
            app.rescan();
        }
    }
    app.mode = Mode::Normal;
}

/// y/n on the delete-whole-project prompt.
fn handle_confirm_project_key(app: &mut App, code: KeyCode) {
    if code == KeyCode::Char('y') && app.selected_project > 0 {
        let p = &app.library.projects[app.selected_project - 1];
        let label = p.label.clone();
        match mutate::delete_project(&p.mem_dir) {
            Ok(n) => app.status = Some(format!("deleted {label}: {n} files (backed up)")),
            Err(e) => app.status = Some(format!("delete failed: {e}")),
        }
        app.rescan();
    }
    app.mode = Mode::Normal;
}

/// Slug input for a new memory.
fn handle_slug_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.mode = Mode::Normal,
        KeyCode::Backspace => {
            app.input.pop();
        }
        KeyCode::Char(c) => app.input.push(c),
        KeyCode::Enter => {
            let slug = mutate::sanitize_slug(&app.input);
            app.mode = Mode::Normal;
            if slug.is_empty() {
                app.status = Some("empty slug".to_string());
                return;
            }
            let dir = app.library.projects[app.selected_project - 1]
                .mem_dir
                .clone();
            match mutate::create_memory(&dir, &slug) {
                Ok(path) => {
                    app.pending_editor = Some(EditorRequest {
                        path,
                        is_new: true,
                        mem_dir: dir,
                    });
                }
                Err(e) => app.status = Some(format!("create failed: {e}")),
            }
        }
        _ => {}
    }
}

/// Pick a target project for the move.
fn handle_move_key(app: &mut App, code: KeyCode) {
    let n = app.library.projects.len();
    match code {
        KeyCode::Esc => app.mode = Mode::Normal,
        KeyCode::Down | KeyCode::Char('j') => {
            app.move_cursor = if app.move_cursor >= n { 1 } else { app.move_cursor + 1 };
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_cursor = if app.move_cursor <= 1 { n } else { app.move_cursor - 1 };
        }
        KeyCode::Enter => {
            app.mode = Mode::Normal;
            let Some(m) = app.selected_memory() else {
                return;
            };
            let target = &app.library.projects[app.move_cursor - 1];
            if Some(target.mem_dir.as_path()) == m.path.parent() {
                app.status = Some("already in that project".to_string());
                return;
            }
            let label = target.label.clone();
            let dst = target.mem_dir.clone();
            match mutate::move_memory(&m.path, &m.slug, &m.name, &m.description, &dst) {
                Ok(()) => app.status = Some(format!("moved {} → {label}", m.slug)),
                Err(e) => app.status = Some(format!("move failed: {e}")),
            }
            app.rescan();
        }
        _ => {}
    }
}

/// Pick a new `type:` for the memory.
fn handle_type_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.mode = Mode::Normal,
        KeyCode::Left | KeyCode::Char('h') | KeyCode::Up | KeyCode::Char('k') => {
            app.type_cursor = (app.type_cursor + TYPES.len() - 1) % TYPES.len();
        }
        KeyCode::Right | KeyCode::Char('l') | KeyCode::Down | KeyCode::Char('j') => {
            app.type_cursor = (app.type_cursor + 1) % TYPES.len();
        }
        KeyCode::Enter => {
            app.mode = Mode::Normal;
            if let Some(m) = app.selected_memory() {
                let t = TYPES[app.type_cursor];
                match mutate::retype(&m.path, t) {
                    Ok(()) => app.status = Some(format!("{} type → {t}", m.slug)),
                    Err(e) => app.status = Some(format!("retype failed: {e}")),
                }
                app.rescan();
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    /// Render full UI into an in-memory buffer (no tty) against real disk data.
    /// Exercises every pane, plus the filter and ALL-projects paths.
    #[test]
    fn renders_without_panicking() {
        let mut app = App::new();
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();

        terminal.draw(|f| ui::render(f, &mut app)).unwrap();

        // Switch to a concrete project + apply a filter, render again.
        app.next_project();
        app.focus = Focus::Memories;
        app.mode = Mode::Filter;
        for c in "test".chars() {
            handle_filter_key(&mut app, KeyCode::Char(c));
        }
        terminal.draw(|f| ui::render(f, &mut app)).unwrap();

        // A filter that matches nothing -> NO SIGNAL path.
        for c in "zzzqqq_nomatch".chars() {
            handle_filter_key(&mut app, KeyCode::Char(c));
        }
        terminal.draw(|f| ui::render(f, &mut app)).unwrap();
    }

    /// Render each modal mode headlessly.
    #[test]
    fn renders_modal_modes() {
        let mut app = App::new();
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();

        for mode in [
            Mode::ConfirmDelete,
            Mode::ConfirmDeleteProject,
            Mode::NewSlug,
            Mode::MoveProject,
            Mode::PickType,
        ] {
            app.mode = mode;
            terminal.draw(|f| ui::render(f, &mut app)).unwrap();
        }

        app.mode = Mode::Normal;
        app.status = Some("a status message".to_string());
        terminal.draw(|f| ui::render(f, &mut app)).unwrap();
    }
}
