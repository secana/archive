//! Tests for TAR archive extraction (all compression variants)

mod common;

use archive::{ArchiveExtractor, ArchiveFormat};
use common::{assert_contains_file, read_test_archive};

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
