//! Common test utilities and helpers

use archive::ExtractedFile;
use std::fs;
use std::path::Path;

pub const TEST_ARCHIVES_DIR: &str = "test-archives";

/// Helper to read a test archive file
pub fn read_test_archive(filename: &str) -> Vec<u8> {
    let path = Path::new(TEST_ARCHIVES_DIR).join(filename);
    fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "Failed to read test archive '{}'. Did you run 'nix run .#generateTestArchives'? Error: {}",
            filename, e
        )
    })
}

/// Helper to check if extracted files contain expected content
pub fn assert_contains_file<'a>(
    files: &'a [ExtractedFile],
    path_contains: &str,
) -> &'a ExtractedFile {
    files
        .iter()
        .find(|f| f.path.contains(path_contains))
        .unwrap_or_else(|| {
            panic!(
                "Expected to find file containing '{}' in path",
                path_contains
            )
        })
}
