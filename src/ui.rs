//! 1995-sci-fi CRT rendering. Phosphor green + amber on black, double-struck
//! borders, a scanline header, blocky HUD chrome.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, Focus, Mode, TYPES};

// --- palette -------------------------------------------------------------
const PHOSPHOR: Color = Color::Rgb(0, 255, 140); // primary text
const DIM: Color = Color::Rgb(0, 140, 80); // inactive / secondary
const AMBER: Color = Color::Rgb(255, 176, 0); // selection / accent
const CYAN: Color = Color::Rgb(0, 200, 200); // type: project
const MAGENTA: Color = Color::Rgb(220, 120, 255); // type: user
const GHOST: Color = Color::Rgb(70, 90, 80); // chrome / faint
const PAPER: Color = Color::Rgb(210, 230, 220); // type: index

/// Color a memory by its `type`.
fn type_color(t: &str) -> Color {
    match t {
        "feedback" => AMBER,
        "project" => CYAN,
        "reference" => PHOSPHOR,
        "user" => MAGENTA,
        "index" => PAPER,
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

    // Two panes on top (projects | memories), full-width detail below.
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Min(8)])
        .split(chunks[1]);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(32), Constraint::Min(20)])
        .split(rows[0]);

    render_projects(f, app, cols[0]);
    render_memories(f, app, cols[1]);
    render_detail(f, app, rows[1]);
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

/// Memories that aren't the MEMORY.md index entry.
fn real_count(app: &App, idxs: &[usize]) -> usize {
    idxs.iter()
        .filter(|&&i| app.library.memories[i].mtype != "index")
        .count()
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let total = app
        .library
        .memories
        .iter()
        .filter(|m| m.mtype != "index")
        .count();
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
        format!("  {total} memories across {projs} projects"),
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
    let moving = app.mode == Mode::MoveProject;
    let focused = app.focus == Focus::Projects || moving;
    let total = app
        .library
        .memories
        .iter()
        .filter(|m| m.mtype != "index")
        .count();

    let mut items: Vec<ListItem> = Vec::new();
    // Synthetic "ALL" entry at index 0.
    items.push(ListItem::new(Line::from(vec![
        Span::styled("◆ ", Style::default().fg(AMBER)),
        Span::styled("ALL PROJECTS", Style::default().fg(PHOSPHOR)),
        Span::styled(format!("  ({total})"), Style::default().fg(DIM)),
    ])));
    for p in &app.library.projects {
        let count = real_count(app, &p.memory_idx);
        items.push(ListItem::new(Line::from(vec![
            Span::styled("▪ ", Style::default().fg(DIM)),
            Span::styled(p.label.clone(), Style::default().fg(PHOSPHOR)),
            Span::styled(format!("  ({count})"), Style::default().fg(DIM)),
        ])));
    }

    let (title, highlight_bg) = if moving {
        ("MOVE TO", CYAN)
    } else {
        ("PROJECTS", AMBER)
    };
    let list = List::new(items)
        .block(retro_block(title, focused))
        .highlight_style(
            Style::default()
                .bg(highlight_bg)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    if moving {
        let mut state = ListState::default();
        state.select(Some(app.move_cursor));
        f.render_stateful_widget(list, area, &mut state);
    } else {
        f.render_stateful_widget(list, area, &mut app.proj_state);
    }
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

    // Block first, then scope-wide stats strip above the list inside it.
    let block = retro_block(&title, focused);
    let inner = block.inner(area);
    f.render_widget(block, area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(inner);
    f.render_widget(
        Paragraph::new(stats_lines(app, &idxs, inner.width)),
        rows[0],
    );
    let list_area = rows[1];

    if idxs.is_empty() {
        let p = Paragraph::new(Line::from(Span::styled(
            "── NO SIGNAL ──",
            Style::default().fg(GHOST),
        )))
        .alignment(Alignment::Center);
        f.render_widget(p, list_area);
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
        .highlight_style(
            Style::default()
                .bg(AMBER)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    f.render_stateful_widget(list, list_area, &mut app.mem_state);
}

fn render_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let idxs = app.current_memory_indices();
    let sel = app.mem_state.selected().unwrap_or(0);

    let block = retro_block("DETAIL", false);

    let Some(&mi) = idxs.get(sel) else {
        let p = Paragraph::new(Line::from(Span::styled(
            "no memory selected",
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
        Span::styled(
            format!(
                "  ·  created {}",
                m.created.map(fmt_ts).unwrap_or_else(|| "?".to_string())
            ),
            Style::default().fg(DIM),
        ),
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
    if m.mtype == "index" {
        render_index_body(&app.library, m, &mut lines);
    } else {
        for raw in m.body.lines() {
            lines.push(Line::from(Span::styled(
                raw.to_string(),
                Style::default().fg(PHOSPHOR),
            )));
        }
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
    let created = m.created.map(fmt_ts).unwrap_or_else(|| "?".to_string());
    let modified = m.modified.map(fmt_ts).unwrap_or_else(|| "?".to_string());
    lines.push(Line::from(Span::styled(
        format!("✦ created {created}  ·  modified {modified}"),
        Style::default().fg(GHOST),
    )));

    let p = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.detail_scroll, 0));
    f.render_widget(p, area);
}

fn render_status(f: &mut Frame, app: &App, area: Rect) {
    let content = match app.mode {
        Mode::Filter => Line::from(vec![
            Span::styled(" SEARCH ", Style::default().bg(AMBER).fg(Color::Black)),
            Span::styled(format!(" /{}", app.filter), Style::default().fg(AMBER)),
            Span::styled("▏", Style::default().fg(AMBER)),
            Span::styled("   [enter] apply  [esc] cancel", Style::default().fg(DIM)),
        ]),
        Mode::ConfirmDelete => {
            let slug = app
                .selected_memory()
                .map(|m| m.slug)
                .unwrap_or_default();
            Line::from(vec![
                Span::styled(" DELETE ", Style::default().bg(AMBER).fg(Color::Black)),
                Span::styled(
                    format!(" {slug}.md — sure? "),
                    Style::default().fg(AMBER),
                ),
                Span::styled("[y] delete  [esc/n] cancel", Style::default().fg(DIM)),
            ])
        }
        Mode::ConfirmDeleteProject => {
            let (label, count) = if app.selected_project > 0 {
                let p = &app.library.projects[app.selected_project - 1];
                (p.label.clone(), p.memory_idx.len())
            } else {
                (String::new(), 0)
            };
            Line::from(vec![
                Span::styled(" DELETE PROJECT ", Style::default().bg(AMBER).fg(Color::Black)),
                Span::styled(
                    format!(" {label} — all {count} files — sure? "),
                    Style::default().fg(AMBER),
                ),
                Span::styled("[y] delete  [esc/n] cancel", Style::default().fg(DIM)),
            ])
        }
        Mode::NewSlug => Line::from(vec![
            Span::styled(" NEW ", Style::default().bg(PHOSPHOR).fg(Color::Black)),
            Span::styled(
                format!(" slug: {}", app.input),
                Style::default().fg(PHOSPHOR),
            ),
            Span::styled("▏", Style::default().fg(PHOSPHOR)),
            Span::styled(
                "   [enter] create + edit  [esc] cancel",
                Style::default().fg(DIM),
            ),
        ]),
        Mode::MoveProject => Line::from(vec![
            Span::styled(" MOVE ", Style::default().bg(CYAN).fg(Color::Black)),
            Span::styled(
                "  [j/k] pick target  [enter] move  [esc] cancel",
                Style::default().fg(DIM),
            ),
        ]),
        Mode::PickType => {
            let mut spans = vec![
                Span::styled(" TYPE ", Style::default().bg(CYAN).fg(Color::Black)),
                Span::raw("  "),
            ];
            for (i, t) in TYPES.iter().enumerate() {
                let style = if i == app.type_cursor {
                    Style::default()
                        .bg(type_color(t))
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(type_color(t))
                };
                spans.push(Span::styled(format!(" {t} "), style));
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled(
                "  [h/l] pick  [enter] apply  [esc] cancel",
                Style::default().fg(DIM),
            ));
            Line::from(spans)
        }
        Mode::Normal => {
            if let Some(msg) = &app.status {
                Line::from(vec![
                    Span::styled(" » ", Style::default().fg(GHOST)),
                    Span::styled(msg.clone(), Style::default().fg(AMBER)),
                ])
            } else {
                Line::from(vec![
                    Span::styled(" ", Style::default()),
                    key("↑↓/jk"),
                    sep("move"),
                    key("tab"),
                    sep("pane"),
                    key("/"),
                    sep("search"),
                    key("e"),
                    sep("edit"),
                    key("n"),
                    sep("new"),
                    key("d"),
                    sep("del"),
                    key("m"),
                    sep("move"),
                    key("t"),
                    sep("type"),
                    key("r"),
                    sep("rescan"),
                    key("q"),
                    sep("quit"),
                ])
            }
        }
    };
    f.render_widget(Paragraph::new(content), area);
}

/// Aggregate stats for the memories currently in scope (selection + filter),
/// rendered as a strip at the top of the detail pane.
fn stats_lines(app: &App, idxs: &[usize], width: u16) -> Vec<Line<'static>> {
    let mems: Vec<&crate::memory::Memory> = idxs
        .iter()
        .map(|&i| &app.library.memories[i])
        .filter(|m| m.mtype != "index")
        .collect();

    let mut counts = vec![
        Span::styled(
            format!("{} ", mems.len()),
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
        ),
        Span::styled("· ", Style::default().fg(GHOST)),
    ];
    for t in TYPES.iter().chain(["?"].iter()) {
        let n = mems.iter().filter(|m| m.mtype == *t).count();
        if n > 0 {
            counts.push(Span::styled(
                format!("{t} {n}  "),
                Style::default().fg(type_color(t)),
            ));
        }
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let within = |days: u64| {
        mems.iter()
            .filter(|m| m.created.is_some_and(|c| now.saturating_sub(c) <= days * 86400))
            .count()
    };
    let oldest = mems.iter().filter_map(|m| m.created).min();
    let newest = mems.iter().filter_map(|m| m.created).max();
    let span = match (oldest, newest) {
        (Some(o), Some(n)) => format!("{} → {}", fmt_date(o), fmt_date(n)),
        _ => "no timestamps".to_string(),
    };
    let recency = Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(
            format!("new 7d: {}  30d: {}", within(7), within(30)),
            Style::default().fg(PHOSPHOR),
        ),
        Span::styled(format!("  ·  {span}"), Style::default().fg(DIM)),
    ]);

    vec![
        Line::from(counts),
        recency,
        Line::from(Span::styled(
            "═".repeat(width.saturating_sub(4) as usize),
            Style::default().fg(GHOST),
        )),
    ]
}

/// Index lines all follow `- [Title](file.md) — hook`; render them as a
/// clean ledger instead of raw markdown. Headers and stray lines pass through.
fn render_index_body(
    library: &crate::memory::Library,
    index: &crate::memory::Memory,
    lines: &mut Vec<Line<'static>>,
) {
    let dir = index.path.parent();
    for raw in index.body.lines() {
        if let Some((name, slug, desc)) = parse_index_line(raw) {
            // Color the entry by the linked memory's type, when it resolves.
            let color = library
                .memories
                .iter()
                .find(|m| m.slug == slug && m.path.parent() == dir)
                .map(|m| type_color(&m.mtype))
                .unwrap_or(GHOST);
            let mut spans = vec![
                Span::styled("▪ ", Style::default().fg(color)),
                Span::styled(name, Style::default().fg(color).add_modifier(Modifier::BOLD)),
            ];
            if !desc.is_empty() {
                spans.push(Span::styled(
                    format!("  {desc}"),
                    Style::default().fg(DIM),
                ));
            }
            lines.push(Line::from(spans));
        } else if let Some(h) = raw.strip_prefix('#') {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                h.trim_start_matches('#').trim().to_uppercase(),
                Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                raw.to_string(),
                Style::default().fg(PHOSPHOR),
            )));
        }
    }
}

/// `- [Title](file.md) — hook` -> (Title, file-stem, hook). Hook is optional.
fn parse_index_line(line: &str) -> Option<(String, String, String)> {
    let rest = line.trim_start().strip_prefix("- [")?;
    let (name, rest) = rest.split_once("](")?;
    let (target, rest) = rest.split_once(')')?;
    let slug = target.strip_suffix(".md")?;
    let desc = rest
        .trim_start()
        .trim_start_matches(['—', '-', '–'])
        .trim()
        .to_string();
    Some((name.to_string(), slug.to_string(), desc))
}

/// Unix secs -> "YYYY-MM-DD HH:MM" (UTC). Hand-rolled to stay dependency-free,
/// via Howard Hinnant's civil_from_days.
fn fmt_ts(secs: u64) -> String {
    let (y, mo, d, hh, mi) = civil_from_unix(secs);
    format!("{y:04}-{mo:02}-{d:02} {hh:02}:{mi:02}")
}

/// Unix secs -> "YYYY-MM-DD" (UTC).
fn fmt_date(secs: u64) -> String {
    let (y, mo, d, _, _) = civil_from_unix(secs);
    format!("{y:04}-{mo:02}-{d:02}")
}

fn civil_from_unix(secs: u64) -> (i64, u32, u32, u32, u32) {
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = yoe as i64 + era * 400 + i64::from(m <= 2);
    (y, m, d, (rem / 3600) as u32, ((rem % 3600) / 60) as u32)
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

#[cfg(test)]
mod tests {
    use super::{fmt_ts, parse_index_line};

    #[test]
    fn formats_unix_timestamps() {
        assert_eq!(fmt_ts(0), "1970-01-01 00:00");
        assert_eq!(fmt_ts(1_000_000_000), "2001-09-09 01:46");
        // Cross-checked with `date -u -r 1780012800`.
        assert_eq!(fmt_ts(1_780_012_800), "2026-05-29 00:00");
    }

    #[test]
    fn index_line_with_hook() {
        let (name, slug, desc) =
            parse_index_line("- [Pre-commit builds dist](precommit-builds-dist.md) — hook text")
                .unwrap();
        assert_eq!(name, "Pre-commit builds dist");
        assert_eq!(slug, "precommit-builds-dist");
        assert_eq!(desc, "hook text");
    }

    #[test]
    fn index_line_without_hook() {
        let (name, slug, desc) = parse_index_line("- [Thing](thing.md)").unwrap();
        assert_eq!(name, "Thing");
        assert_eq!(slug, "thing");
        assert_eq!(desc, "");
    }

    #[test]
    fn non_pointer_lines_pass_through() {
        assert!(parse_index_line("## Rules").is_none());
        assert!(parse_index_line("- plain bullet, no link").is_none());
        assert!(parse_index_line("").is_none());
    }
}
