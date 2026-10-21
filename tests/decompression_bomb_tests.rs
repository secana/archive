//! Regression tests: small, highly-compressible single-file archives that
//! decompress far beyond the configured max_file_size must be rejected via
//! ArchiveError::FileTooLarge without ever fully buffering the bomb.
//!
//! Bombs are generated with the real system CLI tools (200 MB of zeros
//! compressed down to a few hundred bytes to a few hundred KB depending on
//! the format) rather than each format's Rust encoder, since some of those
//! encoders (e.g. `lzma_rs`) don't compress repeated data well enough to
//! produce a realistic bomb.

use archive::{ArchiveError, ArchiveExtractor, ArchiveFormat};
use std::io::Write;
use std::process::{Command, Stdio};

/// Runs `cmd args... < 200MB of zeros`, returning the compressed stdout, or
/// `None` if the tool isn't installed (in which case the test is skipped).
///
/// The 200 MB write to stdin happens on a separate thread, concurrently
/// with reading stdout on this one: writing it all up front and only then
/// reading stdout would deadlock once the child's compressed output
/// exceeds the OS pipe buffer (~64 KB on Linux) before it's consumed all
/// of its input — the child blocks writing output while we're still
/// blocked writing input.
fn compress_zeros_with(cmd: &str, args: &[&str]) -> Option<Vec<u8>> {
    let mut child = match Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => {
            eprintln!("skipping: `{cmd}` CLI not available");
            return None;
        }
    };

    let mut stdin = child.stdin.take().unwrap();
    let writer = std::thread::spawn(move || {
        // Well beyond the default 100 MB max_file_size.
        let zeros = vec![0u8; 200 * 1024 * 1024];
        stdin.write_all(&zeros)
    });

    let output = child.wait_with_output().expect("failed to run {cmd}");
    writer
        .join()
        .expect("writer thread panicked")
        .expect("failed to write to child stdin");

    Some(output.stdout)
}

fn assert_bomb_rejected(compressed: &[u8], format: ArchiveFormat) {
    // Sanity: the compressed bomb is much smaller than the decompressed size.
    assert!(
        compressed.len() < 10 * 1024 * 1024,
        "bomb wasn't actually small: {} bytes",
        compressed.len()
    );

    let extractor = ArchiveExtractor::new(); // default max_file_size = 100MB
    let err = extractor.extract(compressed, format).unwrap_err();
    assert!(matches!(err, ArchiveError::FileTooLarge { .. }), "{err:?}");
}

// Fastest compression level for each tool: an all-zero input compresses
// down to next-to-nothing regardless of effort, so there's no need to pay
// for a high compression ratio here — it just slows the test suite down.

#[test]
fn xz_decompression_bomb_is_rejected_without_full_buffering() {
    if let Some(compressed) = compress_zeros_with("xz", &["-0", "-c"]) {
        assert_bomb_rejected(&compressed, ArchiveFormat::Xz);
    }
}

#[test]
fn gz_decompression_bomb_is_rejected_without_full_buffering() {
    if let Some(compressed) = compress_zeros_with("gzip", &["-1", "-c"]) {
        assert_bomb_rejected(&compressed, ArchiveFormat::Gz);
    }
}

#[test]
fn bz2_decompression_bomb_is_rejected_without_full_buffering() {
    if let Some(compressed) = compress_zeros_with("bzip2", &["-1", "-c"]) {
        assert_bomb_rejected(&compressed, ArchiveFormat::Bz2);
    }
}

#[test]
fn lz4_decompression_bomb_is_rejected_without_full_buffering() {
    if let Some(compressed) = compress_zeros_with("lz4", &["-1", "-c"]) {
        assert_bomb_rejected(&compressed, ArchiveFormat::Lz4);
    }
}

#[test]
fn zst_decompression_bomb_is_rejected_without_full_buffering() {
    if let Some(compressed) = compress_zeros_with("zstd", &["-1", "-c"]) {
        assert_bomb_rejected(&compressed, ArchiveFormat::Zst);
    }
}
