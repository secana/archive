//! Tests for verifying extracted file contents and structure

mod common;

use archive::{ArchiveExtractor, ArchiveFormat};
use common::{assert_contains_file, read_test_archive};

#[test]
fn test_verify_file_contents() {
    let data = read_test_archive("basic.zip");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::Zip)
        .expect("Failed to extract basic.zip");

    // Find hello.txt and verify its content
    let hello_file = assert_contains_file(&files, "hello.txt");
    let content = String::from_utf8_lossy(&hello_file.data);

    assert_eq!(content.trim(), "Hello, World!", "Unexpected file content");
}

#[test]
fn test_nested_directory_structure() {
    let data = read_test_archive("basic.zip");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::Zip)
        .expect("Failed to extract basic.zip");

    // Check for deeply nested file
    assert_contains_file(&files, "nested/deep/path/deep-file.txt");
}

#[test]
fn test_binary_file_extraction() {
    let data = read_test_archive("basic.zip");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::Zip)
        .expect("Failed to extract basic.zip");

    // Find binary.bin
    let binary_file = assert_contains_file(&files, "binary.bin");

    // Should be 10KB of random data
    assert_eq!(
        binary_file.data.len(),
        10 * 1024,
        "Expected 10KB binary file"
    );
}

#[test]
fn test_large_file_extraction() {
    let data = read_test_archive("basic.zip");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::Zip)
        .expect("Failed to extract basic.zip");

    // Find large-file.bin
    let large_file = assert_contains_file(&files, "large-file.bin");

    // Should be 1MB of random data
    assert_eq!(large_file.data.len(), 1024 * 1024, "Expected 1MB file");
}
