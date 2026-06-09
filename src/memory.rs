//! Discovery and parsing of Claude Code auto-memory files.
//!
//! Layout on disk:
//!   ~/.claude/projects/<mangled-project-path>/memory/
//!     ├── MEMORY.md     # human-curated index — NOT a memory, excluded here
//!     └── <slug>.md     # one atomic memory each, with YAML-ish frontmatter

use std::fs;
use std::path::{Path, PathBuf};

/// A single extracted memory (one `<slug>.md` file).
#[derive(Clone)]
pub struct Memory {
    pub project: String,      // best-effort short label for the owning project
    pub project_dir: String,  // raw mangled dir name, for disambiguation
    pub path: PathBuf,        // absolute path to the .md file
    pub slug: String,         // file stem
    pub name: String,         // frontmatter `name`, falls back to slug
    pub description: String,  // frontmatter `description`
    pub mtype: String,        // frontmatter `type` (or metadata.type); "?" if absent
    pub body: String,         // markdown after the frontmatter block
}

/// A project that owns one or more memories.
pub struct Project {
    pub label: String,        // best-effort short name
    pub memory_idx: Vec<usize>, // indices into the flat memory vec
}

/// Result of a full scan: every memory, flat, plus the per-project grouping.
pub struct Library {
    pub memories: Vec<Memory>,
    pub projects: Vec<Project>,
}

/// Walk `~/.claude/projects/*/memory/*.md` and parse every memory.
/// Returns an empty library (not an error) if the projects dir is absent.
pub fn scan() -> Library {
    let home = std::env::var("HOME").unwrap_or_default();
    let root = Path::new(&home).join(".claude").join("projects");

    let mut memories: Vec<Memory> = Vec::new();
    let mut projects: Vec<Project> = Vec::new();

    let Ok(entries) = fs::read_dir(&root) else {
        return Library { memories, projects };
    };

    // Stable ordering so the UI doesn't reshuffle between runs.
    let mut project_dirs: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    project_dirs.sort();

    for proj_path in project_dirs {
        let mem_dir = proj_path.join("memory");
        if !mem_dir.is_dir() {
            continue;
        }
        let dir_name = proj_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let label = short_label(&dir_name);

        let Ok(files) = fs::read_dir(&mem_dir) else {
            continue;
        };
        let mut md_files: Vec<PathBuf> = files
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
            .filter(|p| {
                // Exclude the index; it is not an atomic memory.
                !p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.eq_ignore_ascii_case("MEMORY.md"))
                    .unwrap_or(false)
            })
            .collect();
        md_files.sort();

        let mut idxs = Vec::new();
        for path in md_files {
            if let Some(mem) = parse_memory(&path, &label, &dir_name) {
                idxs.push(memories.len());
                memories.push(mem);
            }
        }

        if !idxs.is_empty() {
            projects.push(Project {
                label,
                memory_idx: idxs,
            });
        }
    }

    Library { memories, projects }
}

/// Best-effort short name from a mangled dir like
/// `-Users-jane-code-my-app` -> `my-app`.
///
/// The mangling replaces both `/` and `.` with `-`, so it is lossy and cannot
/// be perfectly reversed. We take everything after the last `-code-` segment
/// when present (the common case here), otherwise the raw name with the leading
/// dash stripped.
fn short_label(dir: &str) -> String {
    if let Some(pos) = dir.rfind("-code-") {
        return dir[pos + "-code-".len()..].to_string();
    }
    dir.trim_start_matches('-').to_string()
}

/// Parse one memory file. Returns None only if the file can't be read.
fn parse_memory(path: &Path, project: &str, project_dir: &str) -> Option<Memory> {
    let raw = fs::read_to_string(path).ok()?;
    let slug = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    Some(parse_str(&raw, &slug, project, project_dir, path.to_path_buf()))
}

/// Core parse: frontmatter + body -> Memory. Filesystem-free so it is unit-testable.
fn parse_str(raw: &str, slug: &str, project: &str, project_dir: &str, path: PathBuf) -> Memory {
    let (front, body) = split_frontmatter(raw);

    let mut name = String::new();
    let mut description = String::new();
    let mut mtype = String::new();

    // Lenient line-based parse. Handles flat `type:` and the nested form:
    //   metadata:
    //     type: feedback
    let mut in_metadata = false;
    for line in front.lines() {
        let indented = line.starts_with(' ') || line.starts_with('\t');
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some((key, val)) = trimmed.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let val = val.trim().trim_matches('"').trim_matches('\'');

        match key {
            "metadata" if val.is_empty() => in_metadata = true,
            "name" if !indented => name = val.to_string(),
            "description" if !indented => description = val.to_string(),
            "type" => {
                // Accept top-level `type:` or `type:` nested under metadata.
                if !indented || in_metadata {
                    mtype = val.to_string();
                }
            }
            _ => {
                if !indented {
                    in_metadata = false;
                }
            }
        }
    }

    if name.is_empty() {
        name = slug.to_string();
    }
    if mtype.is_empty() {
        mtype = "?".to_string();
    }

    Memory {
        project: project.to_string(),
        project_dir: project_dir.to_string(),
        path,
        slug: slug.to_string(),
        name,
        description,
        mtype,
        body: body.trim().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_frontmatter() {
        let raw = "---\nname: Don't sleep\ndescription: just run it\ntype: feedback\noriginSessionId: abc\n---\nbody line one\n\nbody line two\n";
        let m = parse_inline(raw, "no_sleep");
        assert_eq!(m.name, "Don't sleep");
        assert_eq!(m.description, "just run it");
        assert_eq!(m.mtype, "feedback");
        assert!(m.body.starts_with("body line one"));
        assert!(m.body.contains("body line two"));
    }

    #[test]
    fn nested_metadata_type() {
        let raw =
            "---\nname: A fact\ndescription: d\nmetadata:\n  type: project\n---\nthe body\n";
        let m = parse_inline(raw, "slug");
        assert_eq!(m.mtype, "project");
        assert_eq!(m.name, "A fact");
    }

    #[test]
    fn missing_frontmatter_falls_back_to_slug() {
        let m = parse_inline("just some text, no fences", "my_slug");
        assert_eq!(m.name, "my_slug");
        assert_eq!(m.mtype, "?");
        assert!(m.body.contains("just some text"));
    }

    #[test]
    fn short_label_strips_to_project() {
        assert_eq!(short_label("-Users-jane-code-webapp"), "webapp");
        assert_eq!(short_label("-Users-jane-code-my-app"), "my-app");
    }

    // Thin wrapper over the real parser, no filesystem.
    fn parse_inline(raw: &str, slug: &str) -> Memory {
        parse_str(raw, slug, "p", "d", PathBuf::from("x.md"))
    }

    #[test]
    fn scans_real_disk_without_panicking() {
        // Smoke test against whatever is actually on this machine.
        let lib = scan();
        // Every grouped memory index must be valid.
        for p in &lib.projects {
            for &i in &p.memory_idx {
                assert!(i < lib.memories.len());
            }
        }
        // No memory should be the index file.
        for m in &lib.memories {
            assert_ne!(m.slug.to_lowercase(), "memory");
        }
    }
}

/// Split a `---`-fenced YAML frontmatter block from the markdown body.
/// If there is no frontmatter, returns ("", whole-input).
fn split_frontmatter(raw: &str) -> (String, String) {
    let trimmed = raw.trim_start_matches('\u{feff}'); // tolerate a BOM
    if !trimmed.starts_with("---") {
        return (String::new(), raw.to_string());
    }
    // Find the closing fence: a line that is exactly `---` after the first.
    let mut lines = trimmed.lines();
    lines.next(); // opening ---
    let mut front = String::new();
    let mut body = String::new();
    let mut closed = false;
    for line in lines {
        if !closed && line.trim_end() == "---" {
            closed = true;
            continue;
        }
        if closed {
            body.push_str(line);
            body.push('\n');
        } else {
            front.push_str(line);
            front.push('\n');
        }
    }
    if !closed {
        // No closing fence — treat the whole thing as body to avoid eating it.
        return (String::new(), raw.to_string());
    }
    (front, body)
}
