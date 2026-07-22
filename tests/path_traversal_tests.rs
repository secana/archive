//! Tests that crafted, malicious archives are rejected rather than silently
//! producing entries whose paths (or symlink targets) escape the directory a
//! caller would extract into.

use archive::{ArchiveError, ArchiveExtractor, ArchiveFormat};
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
