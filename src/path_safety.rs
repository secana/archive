//! Validation helpers that protect against path traversal and unsafe link
//! targets embedded in archive entries.
//!
//! Archive formats let each entry declare an arbitrary path (and, for links,
//! an arbitrary target). A malicious archive can use `..` components or
//! absolute paths to try to escape the directory a caller intends to extract
//! into, or to point a link at a file outside that directory. This module
//! centralizes the checks used by every format's extraction path so callers
//! can trust that [`crate::ArchiveEntry::path`] and link targets stay inside
//! the extraction root.

use crate::error::{ArchiveError, Result};
use crate::extractor::ArchiveEntry;
use std::collections::HashSet;

/// Validates that an archive-supplied path (entry name or hard link target) is
/// safe to join onto an extraction directory.
///
/// Rejects:
/// - empty paths
/// - paths containing a NUL byte
/// - absolute paths (`/foo`, `\foo`, `C:\foo`, `C:/foo`)
/// - paths with any `..` component
pub fn validate_path(path: &str) -> Result<()> {
    validate_relative(path)?;

    if components(path).any(|component| component == "..") {
        return Err(unsafe_path(path));
    }

    Ok(())
}

/// Validates that no entry is placed beneath a symlink and that no link
/// target resolves outside the extraction root.
///
/// Symlink targets are resolved relative to the directory containing the
/// symlink, so `..` components are allowed as long as the target stays
/// inside the extraction root. A `..` that follows a component which is
/// itself a symlink is rejected, because the real location of that
/// component depends on where the symlink points.
///
/// Hard link targets are relative to the extraction root and are expected to
/// have passed [`validate_path`] already. They must not pass through a
/// symlink either.
pub(crate) fn validate_links(entries: &[ArchiveEntry]) -> Result<()> {
    let symlinks: HashSet<String> = entries
        .iter()
        .filter(|entry| entry.is_symlink())
        .map(|entry| normalize(entry.path()).join("/"))
        .collect();

    for entry in entries {
        let path = normalize(entry.path());
        if has_symlink_ancestor(&path, &symlinks) {
            return Err(unsafe_path(entry.path()));
        }

        match entry {
            ArchiveEntry::Symlink { target, .. } => {
                let parent = &path[..path.len().saturating_sub(1)];
                validate_symlink_target(parent, target, &symlinks)?;
            }
            ArchiveEntry::HardLink { target, .. } => {
                let target_path = normalize(target);
                if target_path.is_empty()
                    || symlinks.contains(&target_path.join("/"))
                    || has_symlink_ancestor(&target_path, &symlinks)
                {
                    return Err(unsafe_path(target));
                }
            }
            _ => {}
        }
    }

    Ok(())
}

/// Resolves a symlink `target` relative to the symlink's `parent` directory
/// and fails if the resolution leaves the extraction root.
fn validate_symlink_target(
    parent: &[&str],
    target: &str,
    symlinks: &HashSet<String>,
) -> Result<()> {
    validate_relative(target)?;

    let mut resolved: Vec<&str> = parent.to_vec();
    let mut through_symlink = false;

    for component in components(target) {
        if component == ".." {
            if through_symlink || resolved.pop().is_none() {
                return Err(unsafe_path(target));
            }
        } else {
            resolved.push(component);
            through_symlink |= symlinks.contains(&resolved.join("/"));
        }
    }

    Ok(())
}

/// Rejects empty paths, NUL bytes and absolute paths.
fn validate_relative(path: &str) -> Result<()> {
    if path.is_empty() || path.contains('\0') {
        return Err(unsafe_path(path));
    }

    if path.starts_with('/') || path.starts_with('\\') {
        return Err(unsafe_path(path));
    }

    // Windows drive-letter path, e.g. "C:\foo", "C:/foo" or "C:foo".
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return Err(unsafe_path(path));
    }

    Ok(())
}

fn has_symlink_ancestor(path: &[&str], symlinks: &HashSet<String>) -> bool {
    (1..path.len()).any(|len| symlinks.contains(&path[..len].join("/")))
}

/// Splits a path on both `/` and `\`, dropping empty and `.` components.
fn components(path: &str) -> impl Iterator<Item = &str> {
    path.split(['/', '\\'])
        .filter(|component| !component.is_empty() && *component != ".")
}

fn normalize(path: &str) -> Vec<&str> {
    components(path).collect()
}

fn unsafe_path(path: &str) -> ArchiveError {
    ArchiveError::UnsafePath(path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn symlink(path: &str, target: &str) -> ArchiveEntry {
        ArchiveEntry::Symlink {
            path: path.to_string(),
            target: target.to_string(),
        }
    }

    fn hard_link(path: &str, target: &str) -> ArchiveEntry {
        ArchiveEntry::HardLink {
            path: path.to_string(),
            target: target.to_string(),
        }
    }

    fn file(path: &str) -> ArchiveEntry {
        ArchiveEntry::File {
            path: path.to_string(),
            data: Vec::new(),
        }
    }

    #[test]
    fn accepts_normal_relative_paths() {
        assert!(validate_path("foo/bar.txt").is_ok());
        assert!(validate_path("foo.txt").is_ok());
        assert!(validate_path("./foo/bar.txt").is_ok());
        assert!(validate_path("a/b/c/d.txt").is_ok());
    }

    #[test]
    fn rejects_parent_dir_components() {
        assert!(validate_path("../etc/passwd").is_err());
        assert!(validate_path("foo/../../etc/passwd").is_err());
        assert!(validate_path("foo/bar/..").is_err());
        assert!(validate_path("..\\..\\windows\\system32").is_err());
    }

    #[test]
    fn rejects_absolute_paths() {
        assert!(validate_path("/etc/passwd").is_err());
        assert!(validate_path("\\Windows\\System32").is_err());
        assert!(validate_path("C:\\Windows\\System32").is_err());
        assert!(validate_path("C:/Windows/System32").is_err());
    }

    #[test]
    fn rejects_empty_and_nul() {
        assert!(validate_path("").is_err());
        assert!(validate_path("foo\0bar").is_err());
    }

    #[test]
    fn accepts_symlink_with_parent_dir_inside_root() {
        let entries = [
            file("lib/libfoo.so.1"),
            symlink("bin/libfoo.so", "../lib/libfoo.so.1"),
        ];
        assert!(validate_links(&entries).is_ok());
    }

    #[test]
    fn rejects_symlink_escaping_root() {
        assert!(validate_links(&[symlink("link", "../outside")]).is_err());
        assert!(validate_links(&[symlink("a/link", "../../outside")]).is_err());
        assert!(validate_links(&[symlink("a/link", "b/../../../outside")]).is_err());
    }

    #[test]
    fn rejects_absolute_symlink_target() {
        assert!(validate_links(&[symlink("link", "/etc/passwd")]).is_err());
        assert!(validate_links(&[symlink("link", "C:\\Windows")]).is_err());
        assert!(validate_links(&[symlink("link", "")]).is_err());
    }

    #[test]
    fn rejects_parent_dir_after_symlink_component() {
        // "x" points at the root, so "x/.." is the parent of the root.
        let entries = [symlink("x", "."), symlink("y", "x/..")];
        assert!(validate_links(&entries).is_err());
    }

    #[test]
    fn rejects_entry_beneath_symlink() {
        let entries = [symlink("x", "sub"), file("x/evil")];
        assert!(validate_links(&entries).is_err());

        let entries = [symlink("x", "sub"), symlink("x/link", "file")];
        assert!(validate_links(&entries).is_err());
    }

    #[test]
    fn accepts_hard_link_inside_root() {
        let entries = [file("dir/file"), hard_link("other/link", "dir/file")];
        assert!(validate_links(&entries).is_ok());
    }

    #[test]
    fn rejects_hard_link_through_symlink() {
        let entries = [symlink("x", "sub"), hard_link("link", "x/file")];
        assert!(validate_links(&entries).is_err());

        let entries = [symlink("x", "sub"), hard_link("link", "x")];
        assert!(validate_links(&entries).is_err());
    }
}
