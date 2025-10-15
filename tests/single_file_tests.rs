//! Tests for single-file decompression (gz, bz2, xz, lz4, zst)

mod common;

use archive::{ArchiveExtractor, ArchiveFormat};
use common::read_test_archive;

#[test]
fn test_single_gz_decompression() {
    let data = read_test_archive("hello.txt.gz");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::Gz)
        .expect("Failed to decompress hello.txt.gz");

    assert_eq!(files.len(), 1, "Expected single decompressed file");
    let content = String::from_utf8_lossy(&files[0].data);
    assert_eq!(content.trim(), "Hello, World!");
}

#[test]
fn test_single_bz2_decompression() {
    let data = read_test_archive("hello.txt.bz2");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::Bz2)
        .expect("Failed to decompress hello.txt.bz2");

    assert_eq!(files.len(), 1, "Expected single decompressed file");
    let content = String::from_utf8_lossy(&files[0].data);
    assert_eq!(content.trim(), "Hello, World!");
}

#[test]
fn test_single_xz_decompression() {
    let data = read_test_archive("hello.txt.xz");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::Xz)
        .expect("Failed to decompress hello.txt.xz");

    assert_eq!(files.len(), 1, "Expected single decompressed file");
    let content = String::from_utf8_lossy(&files[0].data);
    assert_eq!(content.trim(), "Hello, World!");
}

#[test]
fn test_single_lz4_decompression() {
    let data = read_test_archive("hello.txt.lz4");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::Lz4)
        .expect("Failed to decompress hello.txt.lz4");

    assert_eq!(files.len(), 1, "Expected single decompressed file");
    let content = String::from_utf8_lossy(&files[0].data);
    assert_eq!(content.trim(), "Hello, World!");
}

#[test]
fn test_single_zst_decompression() {
    let data = read_test_archive("hello.txt.zst");
    let extractor = ArchiveExtractor::new();

    let files = extractor
        .extract(&data, ArchiveFormat::Zst)
        .expect("Failed to decompress hello.txt.zst");

    assert_eq!(files.len(), 1, "Expected single decompressed file");
    let content = String::from_utf8_lossy(&files[0].data);
    assert_eq!(content.trim(), "Hello, World!");
}
