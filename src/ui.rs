//! 1995-sci-fi CRT rendering. Phosphor green + amber on black, double-struck
//! borders, a scanline header, blocky HUD chrome.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, Focus};

// --- palette -------------------------------------------------------------
const PHOSPHOR: Color = Color::Rgb(0, 255, 140); // primary text
const DIM: Color = Color::Rgb(0, 140, 80); // inactive / secondary
const AMBER: Color = Color::Rgb(255, 176, 0); // selection / accent
const CYAN: Color = Color::Rgb(0, 200, 200); // type: project
const MAGENTA: Color = Color::Rgb(220, 120, 255); // type: user
const GHOST: Color = Color::Rgb(70, 90, 80); // chrome / faint

/// Color a memory by its `type`.
fn type_color(t: &str) -> Color {
    match t {
        "feedback" => AMBER,
        "project" => CYAN,
        "reference" => PHOSPHOR,
        "user" => MAGENTA,
        _ => GHOST,
    }
}

pub fn render(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // header
            Constraint::Min(0),    // body
            Constraint::Length(1), // status
        ])
        .split(f.area());

    render_header(f, app, chunks[0]);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(30), // projects
            Constraint::Length(40), // memory list
            Constraint::Min(20),    // detail
        ])
        .split(chunks[1]);

    render_projects(f, app, cols[0]);
    render_memories(f, app, cols[1]);
    render_detail(f, app, cols[2]);
    render_status(f, app, chunks[2]);
}

fn retro_block(title: &str, focused: bool) -> Block<'static> {
    let border_color = if focused { AMBER } else { DIM };
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            format!("█ {title} "),
            Style::default()
                .fg(if focused { AMBER } else { PHOSPHOR })
                .add_modifier(Modifier::BOLD),
        ))
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let total = app.library.memories.len();
    let projs = app.library.projects.len();
    let title = Line::from(vec![
        Span::styled("▓▒░ ", Style::default().fg(DIM)),
        Span::styled(
            "AUTO-MEMORIES",
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ::  ", Style::default().fg(GHOST)),
        Span::styled(
            "MNEMONIC EXTRACTION UNIT",
            Style::default().fg(PHOSPHOR),
        ),
    ]);
    let sub = Line::from(Span::styled(
        format!("  {total} memories extracted across {projs} projects  —  read-only"),
        Style::default().fg(DIM),
    ));
    // A scanline strip for the CRT feel.
    let scan = Line::from(Span::styled(
        "▁".repeat(area.width.saturating_sub(2) as usize),
        Style::default().fg(GHOST),
    ));

    let p = Paragraph::new(vec![title, sub, scan]).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(CYAN)),
    );
    f.render_widget(p, area);
}

fn render_projects(f: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Focus::Projects;
    let total = app.library.memories.len();

    let mut items: Vec<ListItem> = Vec::new();
    // Synthetic "ALL" entry at index 0.
    items.push(ListItem::new(Line::from(vec![
        Span::styled("◆ ", Style::default().fg(AMBER)),
        Span::styled("ALL PROJECTS", Style::default().fg(PHOSPHOR)),
        Span::styled(format!("  ({total})"), Style::default().fg(DIM)),
    ])));
    for p in &app.library.projects {
        items.push(ListItem::new(Line::from(vec![
            Span::styled("▪ ", Style::default().fg(DIM)),
            Span::styled(p.label.clone(), Style::default().fg(PHOSPHOR)),
            Span::styled(
                format!("  ({})", p.memory_idx.len()),
                Style::default().fg(DIM),
            ),
        ])));
    }

    let list = List::new(items)
        .block(retro_block("PROJECTS", focused))
        .highlight_style(
            Style::default()
                .bg(AMBER)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    f.render_stateful_widget(list, area, &mut app.proj_state);
}

fn render_memories(f: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Focus::Memories;
    let idxs = app.current_memory_indices();
    let show_proj = app.selected_project == 0;

    let title = if app.filter.is_empty() {
        format!("MEMORIES [{}]", idxs.len())
    } else {
        format!("MEMORIES [{}] /{}", idxs.len(), app.filter)
    };

    if idxs.is_empty() {
        let p = Paragraph::new(Line::from(Span::styled(
            "── NO SIGNAL ──",
            Style::default().fg(GHOST),
        )))
        .alignment(Alignment::Center)
        .block(retro_block(&title, focused));
        f.render_widget(p, area);
        return;
    }

    let items: Vec<ListItem> = idxs
        .iter()
        .map(|&i| {
            let m = &app.library.memories[i];
            let mut spans = vec![
                Span::styled("● ", Style::default().fg(type_color(&m.mtype))),
                Span::styled(m.name.clone(), Style::default().fg(type_color(&m.mtype))),
            ];
            if show_proj {
                spans.push(Span::styled(
                    format!("  ·{}", m.project),
                    Style::default().fg(GHOST),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .block(retro_block(&title, focused))
        .highlight_style(
            Style::default()
                .bg(AMBER)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    f.render_stateful_widget(list, area, &mut app.mem_state);
}

fn render_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let idxs = app.current_memory_indices();
    let sel = app.mem_state.selected().unwrap_or(0);

    let block = retro_block("DETAIL", false);

    let Some(&mi) = idxs.get(sel) else {
        let p = Paragraph::new(Line::from(Span::styled(
            "no clip selected",
            Style::default().fg(GHOST),
        )))
        .block(block);
        f.render_widget(p, area);
        return;
    };
    let m = &app.library.memories[mi];

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        m.name.clone(),
        Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(vec![
        Span::styled("[", Style::default().fg(GHOST)),
        Span::styled(
            m.mtype.clone(),
            Style::default()
                .fg(type_color(&m.mtype))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("]  ", Style::default().fg(GHOST)),
        Span::styled(m.project.clone(), Style::default().fg(DIM)),
        Span::styled(format!("  ·  {}.md", m.slug), Style::default().fg(GHOST)),
    ]));
    if !m.description.is_empty() {
        lines.push(Line::from(Span::styled(
            m.description.clone(),
            Style::default().fg(PHOSPHOR).add_modifier(Modifier::ITALIC),
        )));
    }
    lines.push(Line::from(Span::styled(
        "─".repeat(area.width.saturating_sub(4) as usize),
        Style::default().fg(GHOST),
    )));
    for raw in m.body.lines() {
        lines.push(Line::from(Span::styled(
            raw.to_string(),
            Style::default().fg(PHOSPHOR),
        )));
    }
    // On-disk provenance footer.
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "─".repeat(area.width.saturating_sub(4) as usize),
        Style::default().fg(GHOST),
    )));
    lines.push(Line::from(Span::styled(
        format!("◇ {}", m.project_dir),
        Style::default().fg(GHOST),
    )));
    lines.push(Line::from(Span::styled(
        format!("⊳ {}", m.path.display()),
        Style::default().fg(GHOST),
    )));

    let p = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.detail_scroll, 0));
    f.render_widget(p, area);
}

fn render_status(f: &mut Frame, app: &App, area: Rect) {
    let content = if app.filtering {
        Line::from(vec![
            Span::styled(" SEARCH ", Style::default().bg(AMBER).fg(Color::Black)),
            Span::styled(
                format!(" /{}", app.filter),
                Style::default().fg(AMBER),
            ),
            Span::styled("▏", Style::default().fg(AMBER)),
            Span::styled("   [enter] apply  [esc] cancel", Style::default().fg(DIM)),
        ])
    } else {
        Line::from(vec![
            Span::styled(" ", Style::default()),
            key("↑↓/jk"),
            sep("move"),
            key("←→/tab"),
            sep("pane"),
            key("pgup/pgdn"),
            sep("scroll"),
            key("/"),
            sep("search"),
            key("r"),
            sep("rescan"),
            key("q"),
            sep("quit"),
        ])
    };
    f.render_widget(Paragraph::new(content), area);
}

fn key(k: &str) -> Span<'static> {
    Span::styled(
        format!(" {k} "),
        Style::default().fg(Color::Black).bg(PHOSPHOR),
    )
}
fn sep(label: &str) -> Span<'static> {
    Span::styled(format!(" {label}   "), Style::default().fg(DIM))
}
