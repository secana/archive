//! Integration tests for archive extraction using generated test archives

use archive::{ArchiveExtractor, ArchiveFormat, ExtractedFile};
use std::fs;
use std::path::Path;

const TEST_ARCHIVES_DIR: &str = "test-archives";

/// Helper to read a test archive file
fn read_test_archive(filename: &str) -> Vec<u8> {
    let path = Path::new(TEST_ARCHIVES_DIR).join(filename);
    fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "Failed to read test archive '{}'. Did you run 'nix run .#generateTestArchives'? Error: {}",
            filename, e
        )
    })
}

/// Helper to check if extracted files contain expected content
fn assert_contains_file<'a>(files: &'a [ExtractedFile], path_contains: &str) -> &'a ExtractedFile {
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

#[test]
fn test_plain_tar() {
    let data = read_test_archive("archive.tar");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::Tar)
        .expect("Failed to extract archive.tar");

    assert!(!files.is_empty(), "Expected non-empty archive");
    assert_contains_file(&files, "hello.txt");
}

#[test]
fn test_tar_gz() {
    let data = read_test_archive("archive.tar.gz");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::TarGz)
        .expect("Failed to extract archive.tar.gz");

    assert!(!files.is_empty(), "Expected non-empty archive");
    assert_contains_file(&files, "hello.txt");
}

#[test]
fn test_tgz() {
    let data = read_test_archive("archive.tgz");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::TarGz)
        .expect("Failed to extract archive.tgz");

    assert!(!files.is_empty(), "Expected non-empty archive");
    assert_contains_file(&files, "hello.txt");
}

#[test]
fn test_tar_bz2() {
    let data = read_test_archive("archive.tar.bz2");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::TarBz2)
        .expect("Failed to extract archive.tar.bz2");

    assert!(!files.is_empty(), "Expected non-empty archive");
    assert_contains_file(&files, "hello.txt");
}

#[test]
fn test_tbz2() {
    let data = read_test_archive("archive.tbz2");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::TarBz2)
        .expect("Failed to extract archive.tbz2");

    assert!(!files.is_empty(), "Expected non-empty archive");
    assert_contains_file(&files, "hello.txt");
}

#[test]
fn test_tar_xz() {
    let data = read_test_archive("archive.tar.xz");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::TarXz)
        .expect("Failed to extract archive.tar.xz");

    assert!(!files.is_empty(), "Expected non-empty archive");
    assert_contains_file(&files, "hello.txt");
}

#[test]
fn test_txz() {
    let data = read_test_archive("archive.txz");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::TarXz)
        .expect("Failed to extract archive.txz");

    assert!(!files.is_empty(), "Expected non-empty archive");
    assert_contains_file(&files, "hello.txt");
}

#[test]
fn test_tar_zst() {
    let data = read_test_archive("archive.tar.zst");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::TarZst)
        .expect("Failed to extract archive.tar.zst");

    assert!(!files.is_empty(), "Expected non-empty archive");
    assert_contains_file(&files, "hello.txt");
}

#[test]
fn test_nested_tar_gz() {
    let data = read_test_archive("nested.tar.gz");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::TarGz)
        .expect("Failed to extract nested.tar.gz");

    assert!(!files.is_empty(), "Expected non-empty archive");

    // Should contain other archive files
    let nested_files: Vec<_> = files
        .iter()
        .filter(|f| f.path.ends_with(".zip") || f.path.ends_with(".tar.gz"))
        .collect();

    assert!(!nested_files.is_empty(), "Expected to find nested archives");
}

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

#[test]
fn test_all_tar_formats_produce_same_structure() {
    let extractor = ArchiveExtractor::new();

    let tar = read_test_archive("archive.tar");
    let tar_gz = read_test_archive("archive.tar.gz");
    let tar_bz2 = read_test_archive("archive.tar.bz2");
    let tar_xz = read_test_archive("archive.tar.xz");
    let tar_zst = read_test_archive("archive.tar.zst");

    let files_tar = extractor.extract(&tar, ArchiveFormat::Tar).unwrap();
    let files_tar_gz = extractor.extract(&tar_gz, ArchiveFormat::TarGz).unwrap();
    let files_tar_bz2 = extractor.extract(&tar_bz2, ArchiveFormat::TarBz2).unwrap();
    let files_tar_xz = extractor.extract(&tar_xz, ArchiveFormat::TarXz).unwrap();
    let files_tar_zst = extractor.extract(&tar_zst, ArchiveFormat::TarZst).unwrap();

    // All should have the same number of files
    assert_eq!(files_tar.len(), files_tar_gz.len());
    assert_eq!(files_tar.len(), files_tar_bz2.len());
    assert_eq!(files_tar.len(), files_tar_xz.len());
    assert_eq!(files_tar.len(), files_tar_zst.len());

    // All should contain the same files
    for file in &files_tar {
        assert!(
            files_tar_gz.iter().any(|f| f.path == file.path),
            "tar.gz missing file: {}",
            file.path
        );
        assert!(
            files_tar_bz2.iter().any(|f| f.path == file.path),
            "tar.bz2 missing file: {}",
            file.path
        );
        assert!(
            files_tar_xz.iter().any(|f| f.path == file.path),
            "tar.xz missing file: {}",
            file.path
        );
        assert!(
            files_tar_zst.iter().any(|f| f.path == file.path),
            "tar.zst missing file: {}",
            file.path
        );
    }
}
