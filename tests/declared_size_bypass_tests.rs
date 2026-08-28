//! Regression test for a size-limit bypass: a zip entry's declared
//! "uncompressed size" (in its local file header / central directory) is
//! untrusted archive metadata. Before this fix, `extract_zip` checked that
//! declared size against `max_file_size` but then read the entry's *actual*
//! decompressed bytes unbounded, so a small declared size with a genuinely
//! huge compressed payload sailed straight past the configured limit.
//!
//! This crafts such a zip by hand (the `zip` crate's own writer always
//! computes an honest size, so it can't be used to build the malicious
//! case) and confirms extraction is now rejected via `FileTooLarge`.

use archive::{ArchiveError, ArchiveExtractor, ArchiveFormat};
use flate2::write::DeflateEncoder;
use flate2::Compression;
use std::io::Write;

/// Builds a single-entry zip where the declared uncompressed size lies:
/// it claims `declared_size` bytes, but the compressed data actually
/// inflates to `real_data.len()` bytes.
fn build_zip_with_lying_size(name: &str, real_data: &[u8], declared_size: u32) -> Vec<u8> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(real_data).unwrap();
    let compressed = encoder.finish().unwrap();

    let crc = crc32fast::hash(real_data);
    let name_bytes = name.as_bytes();

    let mut local_header = Vec::new();
    local_header.extend_from_slice(&0x04034b50u32.to_le_bytes()); // local file header signature
    local_header.extend_from_slice(&20u16.to_le_bytes()); // version needed
    local_header.extend_from_slice(&0u16.to_le_bytes()); // flags
    local_header.extend_from_slice(&8u16.to_le_bytes()); // method: deflate
    local_header.extend_from_slice(&0u16.to_le_bytes()); // mod time
    local_header.extend_from_slice(&0u16.to_le_bytes()); // mod date
    local_header.extend_from_slice(&crc.to_le_bytes());
    local_header.extend_from_slice(&(compressed.len() as u32).to_le_bytes()); // compressed size (honest)
    local_header.extend_from_slice(&declared_size.to_le_bytes()); // uncompressed size (THE LIE)
    local_header.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
    local_header.extend_from_slice(&0u16.to_le_bytes()); // extra field length
    local_header.extend_from_slice(name_bytes);

    let mut central_dir = Vec::new();
    central_dir.extend_from_slice(&0x02014b50u32.to_le_bytes()); // central dir signature
    central_dir.extend_from_slice(&20u16.to_le_bytes()); // version made by
    central_dir.extend_from_slice(&20u16.to_le_bytes()); // version needed
    central_dir.extend_from_slice(&0u16.to_le_bytes()); // flags
    central_dir.extend_from_slice(&8u16.to_le_bytes()); // method: deflate
    central_dir.extend_from_slice(&0u16.to_le_bytes()); // mod time
    central_dir.extend_from_slice(&0u16.to_le_bytes()); // mod date
    central_dir.extend_from_slice(&crc.to_le_bytes());
    central_dir.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
    central_dir.extend_from_slice(&declared_size.to_le_bytes()); // THE LIE, again
    central_dir.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
    central_dir.extend_from_slice(&0u16.to_le_bytes()); // extra field length
    central_dir.extend_from_slice(&0u16.to_le_bytes()); // comment length
    central_dir.extend_from_slice(&0u16.to_le_bytes()); // disk number
    central_dir.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
    central_dir.extend_from_slice(&0u32.to_le_bytes()); // external attrs
    central_dir.extend_from_slice(&0u32.to_le_bytes()); // local header offset
    central_dir.extend_from_slice(name_bytes);

    let mut eocd = Vec::new();
    eocd.extend_from_slice(&0x06054b50u32.to_le_bytes());
    eocd.extend_from_slice(&0u16.to_le_bytes()); // disk number
    eocd.extend_from_slice(&0u16.to_le_bytes()); // central dir disk number
    eocd.extend_from_slice(&1u16.to_le_bytes()); // entries on this disk
    eocd.extend_from_slice(&1u16.to_le_bytes()); // total entries
    eocd.extend_from_slice(&(central_dir.len() as u32).to_le_bytes());
    eocd.extend_from_slice(&((local_header.len() + compressed.len()) as u32).to_le_bytes());
    eocd.extend_from_slice(&0u16.to_le_bytes()); // comment length

    let mut zip = Vec::new();
    zip.extend_from_slice(&local_header);
    zip.extend_from_slice(&compressed);
    zip.extend_from_slice(&central_dir);
    zip.extend_from_slice(&eocd);
    zip
}

#[test]
fn zip_lying_declared_size_is_rejected_via_actual_bytes_read() {
    // 50 MB of zeros compresses to a tiny deflate stream, but the header
    // claims the entry is only 100 bytes uncompressed.
    let real_data = vec![0u8; 50 * 1024 * 1024];
    let zip = build_zip_with_lying_size("lie.txt", &real_data, 100);

    // Well above the lie (100 bytes), well below the real size (50MB).
    let extractor = ArchiveExtractor::new().with_max_file_size(10 * 1024 * 1024);
    let err = extractor.extract(&zip, ArchiveFormat::Zip).unwrap_err();
    assert!(matches!(err, ArchiveError::FileTooLarge { .. }), "{err:?}");
}

#[test]
fn zip_honest_declared_size_still_extracts_normally() {
    let real_data = b"hello world".to_vec();
    let zip = build_zip_with_lying_size("hello.txt", &real_data, real_data.len() as u32);

    let extractor = ArchiveExtractor::new();
    let files = extractor.extract(&zip, ArchiveFormat::Zip).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].data(), Some(&real_data[..]));
}
