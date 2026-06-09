//! Application state and the filter/selection logic.

use std::path::PathBuf;

use ratatui::widgets::ListState;

use crate::memory::{scan, Library, Memory};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Focus {
    Projects,
    Memories,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Mode {
    Normal,
    Filter,
    ConfirmDelete,
    NewSlug,
    MoveProject,
    PickType,
}

/// The assignable memory types ("index" is reserved for MEMORY.md).
pub const TYPES: [&str; 4] = ["user", "feedback", "project", "reference"];

/// A request to suspend the TUI and open `$EDITOR` on a file.
pub struct EditorRequest {
    pub path: PathBuf,
    /// Newly created file: indexed on save, removed again on editor abort.
    pub is_new: bool,
    /// The memory dir owning `path`, for index updates.
    pub mem_dir: PathBuf,
}

pub struct App {
    pub library: Library,
    /// 0 = the synthetic "ALL PROJECTS" entry; 1.. = library.projects[i-1].
    pub selected_project: usize,
    pub proj_state: ListState,
    pub mem_state: ListState,
    pub focus: Focus,
    pub mode: Mode,
    pub filter: String,
    /// Text being typed in NewSlug mode.
    pub input: String,
    /// MoveProject cursor into the projects pane (1.. — 0/ALL is not a target).
    pub move_cursor: usize,
    /// PickType cursor into TYPES.
    pub type_cursor: usize,
    /// One-shot message shown in the status line until the next keypress.
    pub status: Option<String>,
    pub pending_editor: Option<EditorRequest>,
    pub detail_scroll: u16,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        let library = scan();
        let mut proj_state = ListState::default();
        proj_state.select(Some(0));
        let mut mem_state = ListState::default();
        mem_state.select(Some(0));
        App {
            library,
            selected_project: 0,
            proj_state,
            mem_state,
            focus: Focus::Projects,
            mode: Mode::Normal,
            filter: String::new(),
            input: String::new(),
            move_cursor: 1,
            type_cursor: 0,
            status: None,
            pending_editor: None,
            detail_scroll: 0,
            should_quit: false,
        }
    }

    /// Re-scan disk, preserving selection where possible.
    pub fn rescan(&mut self) {
        self.library = scan();
        self.clamp_selection();
    }

    /// Indices into `library.memories` for the current project + filter.
    pub fn current_memory_indices(&self) -> Vec<usize> {
        let base: Vec<usize> = if self.selected_project == 0 {
            // ALL view: one index per project is noise — hide them.
            (0..self.library.memories.len())
                .filter(|&i| self.library.memories[i].mtype != "index")
                .collect()
        } else {
            self.library.projects[self.selected_project - 1]
                .memory_idx
                .clone()
        };
        if self.filter.is_empty() {
            return base;
        }
        let q = self.filter.to_lowercase();
        base.into_iter()
            .filter(|&i| {
                let m = &self.library.memories[i];
                m.name.to_lowercase().contains(&q)
                    || m.description.to_lowercase().contains(&q)
                    || m.slug.to_lowercase().contains(&q)
                    || m.mtype.to_lowercase().contains(&q)
                    || m.body.to_lowercase().contains(&q)
            })
            .collect()
    }

    /// Snapshot of the memory under the cursor, if any.
    pub fn selected_memory(&self) -> Option<Memory> {
        let idxs = self.current_memory_indices();
        idxs.get(self.mem_state.selected().unwrap_or(0))
            .map(|&i| self.library.memories[i].clone())
    }

    fn project_count(&self) -> usize {
        self.library.projects.len() + 1 // +1 for ALL
    }

    pub fn next_project(&mut self) {
        if self.project_count() == 0 {
            return;
        }
        self.selected_project = (self.selected_project + 1) % self.project_count();
        self.proj_state.select(Some(self.selected_project));
        self.on_list_change();
    }

    pub fn prev_project(&mut self) {
        if self.project_count() == 0 {
            return;
        }
        self.selected_project =
            (self.selected_project + self.project_count() - 1) % self.project_count();
        self.proj_state.select(Some(self.selected_project));
        self.on_list_change();
    }

    pub fn next_memory(&mut self) {
        let len = self.current_memory_indices().len();
        if len == 0 {
            return;
        }
        let cur = self.mem_state.selected().unwrap_or(0);
        let next = (cur + 1) % len;
        self.mem_state.select(Some(next));
        self.detail_scroll = 0;
    }

    pub fn prev_memory(&mut self) {
        let len = self.current_memory_indices().len();
        if len == 0 {
            return;
        }
        let cur = self.mem_state.selected().unwrap_or(0);
        let prev = (cur + len - 1) % len;
        self.mem_state.select(Some(prev));
        self.detail_scroll = 0;
    }

    pub fn scroll_detail_down(&mut self) {
        self.detail_scroll = self.detail_scroll.saturating_add(4);
    }
    pub fn scroll_detail_up(&mut self) {
        self.detail_scroll = self.detail_scroll.saturating_sub(4);
    }

    /// Selecting a new project resets the memory cursor to the top.
    fn on_list_change(&mut self) {
        self.mem_state.select(Some(0));
        self.detail_scroll = 0;
    }

    /// After a filter edit or rescan, keep the memory cursor in range.
    pub fn clamp_selection(&mut self) {
        let len = self.current_memory_indices().len();
        if len == 0 {
            self.mem_state.select(Some(0));
        } else {
            let cur = self.mem_state.selected().unwrap_or(0);
            self.mem_state.select(Some(cur.min(len - 1)));
        }
        if self.selected_project >= self.project_count() {
            self.selected_project = 0;
            self.proj_state.select(Some(0));
        }
    }
}
