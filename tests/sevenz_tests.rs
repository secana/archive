//! Tests for 7-Zip archive extraction

mod common;

use archive::{ArchiveExtractor, ArchiveFormat};
use common::{assert_contains_file, read_test_archive};

#[test]
fn test_7z_archive() {
    let data = read_test_archive("archive.7z");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::SevenZ)
        .expect("Failed to extract archive.7z");

    assert!(!files.is_empty(), "Expected non-empty archive");
    assert_contains_file(&files, "hello.txt");
    assert_contains_file(&files, "test.txt");
}
