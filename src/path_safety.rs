//! Validation helpers that protect against path traversal and unsafe symlink
//! targets embedded in archive entries.
//!
//! Archive formats let each entry declare an arbitrary path (and, for
//! symlinks, an arbitrary target). A malicious archive can use `..`
//! components or absolute paths to try to escape the directory a caller
//! intends to extract into, or to point a symlink at a file outside that
//! directory. This module centralizes the checks used by every format's
//! extraction path so callers can trust that [`crate::ExtractedFile::path`]
//! (and symlink targets) never contain such components.

use crate::error::{ArchiveError, Result};

/// Validates that an archive-supplied path (entry name or symlink target) is
/// safe to join onto an extraction directory.
///
/// Rejects:
/// - empty paths
/// - paths containing a NUL byte
/// - absolute paths (`/foo`, `\foo`, `C:\foo`, `C:/foo`)
/// - paths with any `..` component
pub fn validate_path(path: &str) -> Result<()> {
    if path.is_empty() {
        return Err(ArchiveError::UnsafePath(path.to_string()));
    }

    if path.contains('\0') {
        return Err(ArchiveError::UnsafePath(path.to_string()));
    }

    if path.starts_with('/') || path.starts_with('\\') {
        return Err(ArchiveError::UnsafePath(path.to_string()));
    }

    // Windows drive-letter absolute path, e.g. "C:\foo" or "C:/foo".
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return Err(ArchiveError::UnsafePath(path.to_string()));
    }

    for component in path.split(['/', '\\']) {
        if component == ".." {
            return Err(ArchiveError::UnsafePath(path.to_string()));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
