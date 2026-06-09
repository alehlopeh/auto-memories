//! Mutations on memory files. Rules:
//! - back up the affected file before any destructive op
//! - atomic writes (temp file + rename) — Claude Code may read these live
//! - keep the project's MEMORY.md index in sync on insert/delete/move

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Backups land in `~/.claude/auto-memories-backups/<project-dir>/<file>.<millis>`.
fn backup_root() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    Path::new(&home).join(".claude").join("auto-memories-backups")
}

/// Copy `path` into the backup root before mutating it.
pub fn backup(path: &Path) -> io::Result<PathBuf> {
    let proj = path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    let dir = backup_root().join(proj);
    fs::create_dir_all(&dir)?;
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_millis();
    let fname = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file.md");
    let dest = dir.join(format!("{fname}.{ts}"));
    fs::copy(path, &dest)?;
    Ok(dest)
}

/// Write via temp file + rename so readers never see a half-written file.
pub fn atomic_write(path: &Path, content: &str) -> io::Result<()> {
    let tmp = path.with_extension("md.tmp");
    fs::write(&tmp, content)?;
    fs::rename(&tmp, path)
}

/// Append a `- [name](slug.md) — description` pointer to the dir's MEMORY.md.
pub fn index_append(mem_dir: &Path, slug: &str, name: &str, description: &str) -> io::Result<()> {
    let p = mem_dir.join("MEMORY.md");
    let mut raw = if p.exists() {
        backup(&p)?;
        fs::read_to_string(&p)?
    } else {
        String::new()
    };
    if !raw.is_empty() && !raw.ends_with('\n') {
        raw.push('\n');
    }
    if description.is_empty() {
        raw.push_str(&format!("- [{name}]({slug}.md)\n"));
    } else {
        raw.push_str(&format!("- [{name}]({slug}.md) — {description}\n"));
    }
    atomic_write(&p, &raw)
}

/// Drop every index line that links to `slug.md`. No-op if there is no index.
pub fn index_remove(mem_dir: &Path, slug: &str) -> io::Result<()> {
    let p = mem_dir.join("MEMORY.md");
    if !p.exists() {
        return Ok(());
    }
    let raw = fs::read_to_string(&p)?;
    let needle = format!("({slug}.md)");
    let kept: Vec<&str> = raw.lines().filter(|l| !l.contains(&needle)).collect();
    backup(&p)?;
    let mut out = kept.join("\n");
    if raw.ends_with('\n') && !out.is_empty() {
        out.push('\n');
    }
    atomic_write(&p, &out)
}

/// Backup, remove the file, drop its index pointer.
pub fn delete_memory(path: &Path, slug: &str) -> io::Result<()> {
    backup(path)?;
    fs::remove_file(path)?;
    if let Some(dir) = path.parent() {
        index_remove(dir, slug)?;
    }
    Ok(())
}

/// Backup, rename into the target project's memory dir, fix both indexes.
pub fn move_memory(
    path: &Path,
    slug: &str,
    name: &str,
    description: &str,
    dst_dir: &Path,
) -> io::Result<()> {
    let fname = path
        .file_name()
        .ok_or_else(|| io::Error::other("bad path"))?;
    let dst = dst_dir.join(fname);
    if dst.exists() {
        return Err(io::Error::other("a file with that name exists in the target"));
    }
    backup(path)?;
    fs::rename(path, &dst)?;
    if let Some(src_dir) = path.parent() {
        index_remove(src_dir, slug)?;
    }
    index_append(dst_dir, slug, name, description)
}

/// Rewrite the frontmatter `type:` (flat or nested under `metadata:`) in place.
pub fn retype(path: &Path, new_type: &str) -> io::Result<()> {
    backup(path)?;
    let raw = fs::read_to_string(path)?;
    atomic_write(path, &rewrite_type(&raw, new_type))
}

/// Pure text transform behind `retype`, split out for tests.
fn rewrite_type(raw: &str, new_type: &str) -> String {
    if !raw.trim_start_matches('\u{feff}').starts_with("---") {
        return format!("---\ntype: {new_type}\n---\n\n{raw}");
    }
    let mut lines: Vec<String> = raw.lines().map(String::from).collect();
    let close = lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, l)| l.trim_end() == "---")
        .map(|(i, _)| i);
    let Some(close) = close else {
        // Unclosed fence: the parser treats it all as body; do the same.
        return format!("---\ntype: {new_type}\n---\n\n{raw}");
    };
    let mut replaced = false;
    for l in &mut lines[1..close] {
        let trimmed = l.trim_start();
        if trimmed.starts_with("type:") {
            let indent = &l[..l.len() - trimmed.len()];
            *l = format!("{indent}type: {new_type}");
            replaced = true;
            break;
        }
    }
    if !replaced {
        lines.insert(close, format!("type: {new_type}"));
    }
    let mut out = lines.join("\n");
    if raw.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Create `<slug>.md` from a frontmatter template; errors if it exists.
pub fn create_memory(mem_dir: &Path, slug: &str) -> io::Result<PathBuf> {
    let p = mem_dir.join(format!("{slug}.md"));
    if p.exists() {
        return Err(io::Error::other("file already exists"));
    }
    let template =
        format!("---\nname: {slug}\ndescription: \nmetadata:\n  type: project\n---\n\n");
    atomic_write(&p, &template)?;
    Ok(p)
}

/// Lowercase kebab-case; collapses runs of non-alphanumerics into single dashes.
pub fn sanitize_slug(input: &str) -> String {
    input
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tdir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "auto-memories-test-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn rewrite_type_flat() {
        let raw = "---\nname: x\ntype: feedback\n---\nbody\n";
        let out = rewrite_type(raw, "user");
        assert!(out.contains("type: user"));
        assert!(!out.contains("feedback"));
        assert!(out.ends_with("body\n"));
    }

    #[test]
    fn rewrite_type_nested() {
        let raw = "---\nname: x\nmetadata:\n  type: project\n---\nbody\n";
        let out = rewrite_type(raw, "reference");
        assert!(out.contains("  type: reference"));
    }

    #[test]
    fn rewrite_type_absent_inserts() {
        let raw = "---\nname: x\n---\nbody\n";
        let out = rewrite_type(raw, "user");
        assert!(out.contains("type: user"));
        assert!(out.contains("body"));
    }

    #[test]
    fn rewrite_type_no_frontmatter_prepends() {
        let out = rewrite_type("just body\n", "user");
        assert!(out.starts_with("---\ntype: user\n---\n"));
        assert!(out.contains("just body"));
    }

    #[test]
    fn index_append_and_remove() {
        let d = tdir("index");
        index_append(&d, "a-slug", "A Name", "the hook").unwrap();
        index_append(&d, "other", "Other", "").unwrap();
        let raw = fs::read_to_string(d.join("MEMORY.md")).unwrap();
        assert!(raw.contains("- [A Name](a-slug.md) — the hook"));
        assert!(raw.contains("- [Other](other.md)"));

        index_remove(&d, "a-slug").unwrap();
        let raw = fs::read_to_string(d.join("MEMORY.md")).unwrap();
        assert!(!raw.contains("a-slug"));
        assert!(raw.contains("other.md"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn delete_removes_file_and_index_line() {
        let d = tdir("delete");
        let p = create_memory(&d, "doomed").unwrap();
        index_append(&d, "doomed", "Doomed", "x").unwrap();
        delete_memory(&p, "doomed").unwrap();
        assert!(!p.exists());
        assert!(!fs::read_to_string(d.join("MEMORY.md"))
            .unwrap()
            .contains("doomed"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn move_updates_both_indexes() {
        let src = tdir("move-src");
        let dst = tdir("move-dst");
        let p = create_memory(&src, "wanderer").unwrap();
        index_append(&src, "wanderer", "Wanderer", "roams").unwrap();
        move_memory(&p, "wanderer", "Wanderer", "roams", &dst).unwrap();
        assert!(!p.exists());
        assert!(dst.join("wanderer.md").exists());
        assert!(!fs::read_to_string(src.join("MEMORY.md"))
            .unwrap()
            .contains("wanderer"));
        assert!(fs::read_to_string(dst.join("MEMORY.md"))
            .unwrap()
            .contains("wanderer"));
        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dst);
    }

    #[test]
    fn create_then_retype() {
        let d = tdir("create");
        let p = create_memory(&d, "fresh").unwrap();
        retype(&p, "user").unwrap();
        let raw = fs::read_to_string(&p).unwrap();
        assert!(raw.contains("  type: user"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn slug_sanitizes() {
        assert_eq!(sanitize_slug("  My Cool Memory! "), "my-cool-memory");
        assert_eq!(sanitize_slug("a--b__c"), "a-b-c");
        assert_eq!(sanitize_slug("***"), "");
    }
}
