//! Tests for AR archive extraction

mod common;

use archive::{ArchiveExtractor, ArchiveFormat};
use common::{assert_contains_file, read_test_archive};

#[test]
fn test_ar() {
    let data = read_test_archive("archive.ar");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::Ar)
        .expect("Failed to extract archive.ar");

    assert!(!files.is_empty(), "Expected non-empty archive");
    assert_contains_file(&files, "hello.txt");
}
