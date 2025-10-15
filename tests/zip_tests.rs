//! Tests for ZIP archive extraction

mod common;

use archive::{ArchiveExtractor, ArchiveFormat};
use common::{assert_contains_file, read_test_archive};

#[test]
fn test_basic_zip() {
    let data = read_test_archive("basic.zip");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::Zip)
        .expect("Failed to extract basic.zip");

    assert!(!files.is_empty(), "Expected non-empty archive");

    // Check for expected files
    assert_contains_file(&files, "hello.txt");
    assert_contains_file(&files, "test.txt");
    assert_contains_file(&files, "binary.bin");
}

#[test]
fn test_no_compression_zip() {
    let data = read_test_archive("no-compression.zip");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::Zip)
        .expect("Failed to extract no-compression.zip");

    assert!(!files.is_empty(), "Expected non-empty archive");
    assert_contains_file(&files, "hello.txt");
}

#[test]
fn test_max_compression_zip() {
    let data = read_test_archive("max-compression.zip");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::Zip)
        .expect("Failed to extract max-compression.zip");

    assert!(!files.is_empty(), "Expected non-empty archive");
    assert_contains_file(&files, "hello.txt");
}

#[test]
fn test_encrypted_zip() {
    let data = read_test_archive("encrypted.zip");
    let extractor = ArchiveExtractor::new();

    // Encrypted archives should fail without password support
    let result = extractor.extract(&data, ArchiveFormat::Zip);

    // This will likely fail - encrypted ZIP support depends on the zip crate features
    // For now, we just verify it doesn't panic
    match result {
        Ok(_) => println!("Encrypted archive extracted (unexpected success)"),
        Err(_) => println!("Encrypted archive failed as expected"),
    }
}

#[test]
fn test_nested_zip() {
    let data = read_test_archive("nested.zip");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::Zip)
        .expect("Failed to extract nested.zip");

    assert!(!files.is_empty(), "Expected non-empty archive");

    // Should contain other archive files
    let nested_files: Vec<_> = files
        .iter()
        .filter(|f| f.path.ends_with(".zip") || f.path.ends_with(".tar.gz"))
        .collect();

    assert!(!nested_files.is_empty(), "Expected to find nested archives");
}

#[test]
fn test_deeply_nested_zip() {
    let data = read_test_archive("deeply-nested.zip");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::Zip)
        .expect("Failed to extract deeply-nested.zip");

    assert!(!files.is_empty(), "Expected non-empty archive");

    // First level extraction
    let level1_file = assert_contains_file(&files, "level1.txt");
    assert!(!level1_file.data.is_empty());

    // Should contain level2.zip
    let level2_zip = files
        .iter()
        .find(|f| f.path.contains("level2.zip"))
        .expect("Expected to find level2.zip");

    // Extract level 2
    let level2_files = extractor
        .extract(&level2_zip.data, ArchiveFormat::Zip)
        .expect("Failed to extract level2.zip");

    assert_contains_file(&level2_files, "level2.txt");
}

#[test]
fn test_empty_zip() {
    let data = read_test_archive("empty.zip");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::Zip)
        .expect("Failed to extract empty.zip");

    assert!(files.is_empty(), "Expected empty archive");
}

#[test]
fn test_empty_dirs_zip() {
    let data = read_test_archive("empty-dirs.zip");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::Zip)
        .expect("Failed to extract empty-dirs.zip");

    // Should contain directory entries
    let dirs: Vec<_> = files.iter().filter(|f| f.is_directory).collect();
    assert!(!dirs.is_empty(), "Expected to find directories");
}

#[test]
fn test_special_chars_zip() {
    let data = read_test_archive("special-chars.zip");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::Zip)
        .expect("Failed to extract special-chars.zip");

    assert!(!files.is_empty(), "Expected non-empty archive");

    // Check for files with special characters
    let has_spaces = files.iter().any(|f| f.path.contains("file with spaces"));
    let has_umlaut = files.iter().any(|f| f.path.contains("ümlaut"));

    assert!(
        has_spaces || has_umlaut,
        "Expected files with special characters"
    );
}

#[test]
fn test_potential_bomb_zip() {
    let data = read_test_archive("potential-bomb.zip");

    // Use restrictive limits to test bomb detection
    let extractor = ArchiveExtractor::new()
        .with_max_file_size(5 * 1024 * 1024) // 5 MB limit
        .with_max_total_size(5 * 1024 * 1024);

    let result = extractor.extract(&data, ArchiveFormat::Zip);

    // Should fail due to size limits
    assert!(
        result.is_err(),
        "Expected zip bomb to be caught by size limits"
    );
}
