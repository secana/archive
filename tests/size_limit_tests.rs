//! Tests for size limit enforcement (max file size, max total size)

mod common;

use archive::{ArchiveExtractor, ArchiveFormat};
use common::read_test_archive;

#[test]
fn test_max_file_size_limit() {
    let data = read_test_archive("basic.zip");

    // Set a very small file size limit
    let extractor = ArchiveExtractor::new().with_max_file_size(1024); // 1KB limit

    let result = extractor.extract(&data, ArchiveFormat::Zip);

    // Should fail because binary.bin is 10KB
    assert!(result.is_err(), "Expected to hit file size limit");
}

#[test]
fn test_max_total_size_limit() {
    let data = read_test_archive("basic.zip");

    // Set a small total size limit
    let extractor = ArchiveExtractor::new().with_max_total_size(50 * 1024); // 50KB limit

    let result = extractor.extract(&data, ArchiveFormat::Zip);

    // Should fail because total is > 1MB
    assert!(result.is_err(), "Expected to hit total size limit");
}
