//! auto-memories — a read-only CRT-styled browser for Claude Code's
//! auto-memory files (`~/.claude/projects/*/memory/*.md`).

mod app;
mod memory;
mod ui;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};

use app::{App, Focus};

fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new();

    let result = run(&mut terminal, &mut app);

    ratatui::restore();
    result
}

fn run(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
) -> std::io::Result<()> {
    while !app.should_quit {
        terminal.draw(|f| ui::render(f, app))?;

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        if app.filtering {
            handle_filter_key(app, key.code);
        } else {
            handle_nav_key(app, key.code);
        }
    }
    Ok(())
}

/// Keys while the search prompt is active.
fn handle_filter_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => {
            app.filtering = false;
            app.filter.clear();
            app.clamp_selection();
        }
        KeyCode::Enter => {
            app.filtering = false; // keep the filter, just leave input mode
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
            app.filtering = true;
            app.filter.clear();
        }
        KeyCode::Char('r') => app.rescan(),

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
        app.filtering = true;
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
}
