//! Tests that crafted, malicious archives are rejected rather than silently
//! producing entries whose paths (or symlink targets) escape the directory a
//! caller would extract into.

use archive::{ArchiveEntry, ArchiveError, ArchiveExtractor, ArchiveFormat};
use std::io::Write;
use tar::{Builder, EntryType, Header};

fn build_tar_with_header(configure: impl FnOnce(&mut Header)) -> Vec<u8> {
    let mut builder = Builder::new(Vec::new());
    let mut header = Header::new_gnu();
    header.set_size(0);
    header.set_cksum();
    configure(&mut header);
    header.set_cksum();
    builder.append(&header, std::io::empty()).unwrap();
    builder.into_inner().unwrap()
}

#[test]
fn tar_rejects_parent_dir_traversal_path() {
    // `Header::set_path` refuses `..` components itself, but a hand-crafted
    // malicious archive isn't built with this crate's writer - it just has
    // the raw name bytes on disk. Write the name field directly to simulate
    // that and confirm the *reader* side still rejects it.
    let data = build_tar_with_header(|h| {
        h.set_entry_type(EntryType::Regular);
        let name_field = &mut h.as_mut_bytes()[0..100];
        let name = b"../../etc/passwd";
        name_field[..name.len()].copy_from_slice(name);
    });

    let err = ArchiveExtractor::new()
        .extract(&data, ArchiveFormat::Tar)
        .unwrap_err();
    assert!(matches!(err, ArchiveError::UnsafePath(_)), "{err:?}");
}

#[test]
fn tar_rejects_absolute_path() {
    let data = build_tar_with_header(|h| {
        h.set_entry_type(EntryType::Regular);
        h.set_path_absolute("/etc/passwd").unwrap();
    });

    let err = ArchiveExtractor::new()
        .extract(&data, ArchiveFormat::Tar)
        .unwrap_err();
    assert!(matches!(err, ArchiveError::UnsafePath(_)), "{err:?}");
}

#[test]
fn tar_rejects_symlink_with_traversal_target() {
    let mut builder = Builder::new(Vec::new());
    let mut header = Header::new_gnu();
    header.set_size(0);
    header.set_entry_type(EntryType::Symlink);
    header.set_cksum();
    builder
        .append_link(&mut header, "innocuous-name", "../../etc/passwd")
        .unwrap();
    let data = builder.into_inner().unwrap();

    let err = ArchiveExtractor::new()
        .extract(&data, ArchiveFormat::Tar)
        .unwrap_err();
    assert!(matches!(err, ArchiveError::UnsafePath(_)), "{err:?}");
}

#[test]
fn tar_accepts_and_flags_safe_relative_symlink() {
    let mut builder = Builder::new(Vec::new());
    let mut header = Header::new_gnu();
    header.set_size(0);
    header.set_entry_type(EntryType::Symlink);
    header.set_cksum();
    builder
        .append_link(&mut header, "link.txt", "target.txt")
        .unwrap();
    let data = builder.into_inner().unwrap();

    let files = ArchiveExtractor::new()
        .extract(&data, ArchiveFormat::Tar)
        .unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].is_symlink());
    assert!(!files[0].is_directory());
    assert!(
        matches!(&files[0], archive::ArchiveEntry::Symlink { target, .. } if target == "target.txt")
    );
}

#[test]
fn tar_accepts_normal_archive() {
    let mut builder = Builder::new(Vec::new());
    let mut header = Header::new_gnu();
    let content = b"hello world";
    header.set_size(content.len() as u64);
    header.set_entry_type(EntryType::Regular);
    header.set_path("safe/dir/file.txt").unwrap();
    header.set_cksum();
    builder.append(&header, &content[..]).unwrap();
    let data = builder.into_inner().unwrap();

    let files = ArchiveExtractor::new()
        .extract(&data, ArchiveFormat::Tar)
        .unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path(), "safe/dir/file.txt");
    assert!(!files[0].is_symlink());
    assert_eq!(files[0].data(), Some(&content[..]));
}

#[test]
fn zip_rejects_parent_dir_traversal_path() {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut buf);
        let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
        writer.start_file("../../etc/passwd", options).unwrap();
        writer.write_all(b"pwned").unwrap();
        writer.finish().unwrap();
    }

    let err = ArchiveExtractor::new()
        .extract(buf.get_ref(), ArchiveFormat::Zip)
        .unwrap_err();
    assert!(matches!(err, ArchiveError::UnsafePath(_)), "{err:?}");
}

#[test]
fn zip_accepts_normal_archive() {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut buf);
        let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
        writer.start_file("safe/file.txt", options).unwrap();
        writer.write_all(b"hello world").unwrap();
        writer.finish().unwrap();
    }

    let files = ArchiveExtractor::new()
        .extract(buf.get_ref(), ArchiveFormat::Zip)
        .unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path(), "safe/file.txt");
    assert!(!files[0].is_symlink());
}

fn append_link(builder: &mut Builder<Vec<u8>>, kind: EntryType, path: &str, target: &str) {
    let mut header = Header::new_gnu();
    header.set_size(0);
    header.set_entry_type(kind);
    header.set_cksum();
    builder.append_link(&mut header, path, target).unwrap();
}

fn append_file(builder: &mut Builder<Vec<u8>>, path: &str, content: &[u8]) {
    let mut header = Header::new_gnu();
    header.set_size(content.len() as u64);
    header.set_entry_type(EntryType::Regular);
    header.set_path(path).unwrap();
    header.set_cksum();
    builder.append(&header, content).unwrap();
}

fn extract_tar(build: impl FnOnce(&mut Builder<Vec<u8>>)) -> archive::Result<Vec<ArchiveEntry>> {
    let mut builder = Builder::new(Vec::new());
    build(&mut builder);
    let data = builder.into_inner().unwrap();
    ArchiveExtractor::new().extract(&data, ArchiveFormat::Tar)
}

#[test]
fn tar_accepts_symlink_with_parent_dir_inside_root() {
    let files = extract_tar(|b| {
        append_file(b, "lib/libfoo.so.1", b"elf");
        append_link(b, EntryType::Symlink, "bin/libfoo.so", "../lib/libfoo.so.1");
    })
    .unwrap();

    assert!(matches!(
        &files[1],
        ArchiveEntry::Symlink { target, .. } if target == "../lib/libfoo.so.1"
    ));
}

#[test]
fn tar_rejects_symlink_chain_escaping_root() {
    // "x" points at the root, so "y" -> "x/.." points at its parent.
    let err = extract_tar(|b| {
        append_link(b, EntryType::Symlink, "x", ".");
        append_link(b, EntryType::Symlink, "y", "x/..");
    })
    .unwrap_err();
    assert!(matches!(err, ArchiveError::UnsafePath(_)), "{err:?}");
}

#[test]
fn tar_rejects_file_beneath_symlink() {
    let err = extract_tar(|b| {
        append_link(b, EntryType::Symlink, "x", "sub");
        append_file(b, "x/evil", b"pwned");
    })
    .unwrap_err();
    assert!(matches!(err, ArchiveError::UnsafePath(_)), "{err:?}");
}

#[test]
fn tar_rejects_symlink_without_target() {
    let data = build_tar_with_header(|h| {
        h.set_entry_type(EntryType::Symlink);
        h.set_path("link").unwrap();
    });

    let err = ArchiveExtractor::new()
        .extract(&data, ArchiveFormat::Tar)
        .unwrap_err();
    assert!(matches!(err, ArchiveError::InvalidArchive(_)), "{err:?}");
}

#[test]
fn tar_returns_hard_link_entries() {
    let files = extract_tar(|b| {
        append_file(b, "dir/file.txt", b"hello");
        append_link(b, EntryType::Link, "other/link.txt", "dir/file.txt");
    })
    .unwrap();

    assert_eq!(files.len(), 2);
    assert!(files[1].is_hard_link());
    assert!(!files[1].is_symlink());
    assert!(matches!(
        &files[1],
        ArchiveEntry::HardLink { path, target } if path == "other/link.txt" && target == "dir/file.txt"
    ));
}

#[test]
fn tar_rejects_hard_link_with_parent_dir_target() {
    let err = extract_tar(|b| {
        append_link(b, EntryType::Link, "link", "../etc/passwd");
    })
    .unwrap_err();
    assert!(matches!(err, ArchiveError::UnsafePath(_)), "{err:?}");
}

#[test]
fn tar_rejects_hard_link_through_symlink() {
    let err = extract_tar(|b| {
        append_link(b, EntryType::Symlink, "x", "sub");
        append_link(b, EntryType::Link, "link", "x/file");
    })
    .unwrap_err();
    assert!(matches!(err, ArchiveError::UnsafePath(_)), "{err:?}");
}

fn build_zip_with_symlink(path: &str, target: &str) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut buf);
        let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
        writer.add_symlink(path, target, options).unwrap();
        writer.finish().unwrap();
    }
    buf.into_inner()
}

#[test]
fn zip_detects_symlink_entries() {
    let data = build_zip_with_symlink("dir/link", "../file.txt");

    let files = ArchiveExtractor::new()
        .extract(&data, ArchiveFormat::Zip)
        .unwrap();
    assert_eq!(files.len(), 1);
    assert!(matches!(
        &files[0],
        ArchiveEntry::Symlink { path, target } if path == "dir/link" && target == "../file.txt"
    ));
}

#[test]
fn zip_rejects_symlink_escaping_root() {
    let data = build_zip_with_symlink("link", "../../etc/passwd");

    let err = ArchiveExtractor::new()
        .extract(&data, ArchiveFormat::Zip)
        .unwrap_err();
    assert!(matches!(err, ArchiveError::UnsafePath(_)), "{err:?}");
}

#[test]
fn ar_rejects_parent_dir_traversal_path() {
    let mut builder = ar::Builder::new(Vec::new());
    let content = b"pwned";
    let header = ar::Header::new(b"../evil".to_vec(), content.len() as u64);
    builder.append(&header, &content[..]).unwrap();
    let data = builder.into_inner().unwrap();

    let err = ArchiveExtractor::new()
        .extract(&data, ArchiveFormat::Ar)
        .unwrap_err();
    assert!(matches!(err, ArchiveError::UnsafePath(_)), "{err:?}");
}

#[test]
fn sevenz_rejects_parent_dir_traversal_path() {
    let mut writer = sevenz_rust::SevenZWriter::new(std::io::Cursor::new(Vec::new())).unwrap();
    let mut entry = sevenz_rust::SevenZArchiveEntry::new();
    entry.name = "../../evil".to_string();
    entry.has_stream = true;
    writer
        .push_archive_entry(entry, Some(&b"pwned"[..]))
        .unwrap();
    let data = writer.finish().unwrap().into_inner();

    let err = ArchiveExtractor::new()
        .extract(&data, ArchiveFormat::SevenZ)
        .unwrap_err();
    assert!(matches!(err, ArchiveError::UnsafePath(_)), "{err:?}");
}
