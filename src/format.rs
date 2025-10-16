//! Archive format identification.
//!
//! This module defines the supported archive and compression formats.

/// Supported archive and compression formats.
///
/// This enum represents all archive and compression formats that can be extracted
/// by this crate. It includes multi-file archives (ZIP, TAR, 7-Zip) and single-file
/// compression formats (gzip, bzip2, etc.).
///
/// # Examples
///
/// ```no_run
/// use archive::{ArchiveExtractor, ArchiveFormat};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let extractor = ArchiveExtractor::new();
///
/// // Extract a ZIP archive
/// # let zip_data = vec![0u8; 100];
/// let files = extractor.extract(&zip_data, ArchiveFormat::Zip)?;
///
/// // Extract a gzip-compressed TAR archive
/// # let targz_data = vec![0u8; 100];
/// let files = extractor.extract(&targz_data, ArchiveFormat::TarGz)?;
///
/// // Decompress a single gzip file
/// # let gz_data = vec![0u8; 100];
/// let files = extractor.extract(&gz_data, ArchiveFormat::Gz)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    /// ZIP archive format (`.zip`).
    ///
    /// ZIP is a widely-used archive format that supports multiple compression
    /// methods and can store multiple files with directory structure.
    ///
    /// Supports various compression levels including store (no compression),
    /// deflate, and others.
    Zip,

    /// Plain TAR archive (`.tar`).
    ///
    /// TAR (Tape Archive) is a file format for collecting multiple files into
    /// a single archive file. This variant is uncompressed.
    Tar,

    /// TAR archive with gzip compression (`.tar.gz`, `.tgz`).
    ///
    /// Combines TAR archiving with gzip compression. This is one of the most
    /// common formats on Unix-like systems.
    TarGz,

    /// TAR archive with bzip2 compression (`.tar.bz2`, `.tbz2`).
    ///
    /// Combines TAR archiving with bzip2 compression, which typically provides
    /// better compression ratios than gzip but is slower.
    TarBz2,

    /// TAR archive with XZ/LZMA compression (`.tar.xz`, `.txz`).
    ///
    /// Combines TAR archiving with XZ compression (based on LZMA), which provides
    /// excellent compression ratios but requires more memory and CPU time.
    TarXz,

    /// TAR archive with Zstandard compression (`.tar.zst`).
    ///
    /// Combines TAR archiving with Zstandard compression, which offers a good
    /// balance between compression ratio and speed.
    TarZst,

    /// TAR archive with LZ4 compression (`.tar.lz4`).
    ///
    /// Combines TAR archiving with LZ4 compression, which prioritizes speed
    /// over compression ratio. Useful for fast decompression.
    TarLz4,

    /// Single file compressed with gzip (`.gz`).
    ///
    /// A single file compressed using the gzip algorithm. If the gzip header
    /// contains the original filename, it will be preserved during extraction;
    /// otherwise, the file will be named "data".
    Gz,

    /// Single file compressed with bzip2 (`.bz2`).
    ///
    /// A single file compressed using the bzip2 algorithm. The extracted file
    /// will be named "data" as bzip2 doesn't store original filenames.
    Bz2,

    /// Single file compressed with XZ/LZMA (`.xz`).
    ///
    /// A single file compressed using the XZ algorithm (based on LZMA).
    /// The extracted file will be named "data" as XZ doesn't store original filenames.
    Xz,

    /// Single file compressed with LZ4 (`.lz4`).
    ///
    /// A single file compressed using the LZ4 algorithm. The extracted file
    /// will be named "data" as LZ4 doesn't store original filenames.
    Lz4,

    /// Single file compressed with Zstandard (`.zst`).
    ///
    /// A single file compressed using the Zstandard algorithm. The extracted file
    /// will be named "data" as Zstandard doesn't store original filenames by default.
    Zst,

    /// 7-Zip archive format (`.7z`).
    ///
    /// 7-Zip is a high-compression archive format that supports multiple
    /// compression algorithms and can achieve excellent compression ratios.
    SevenZ,
}

impl ArchiveFormat {
    /// Returns the human-readable name of the archive format.
    ///
    /// This method returns a string representation of the format, suitable
    /// for display purposes.
    ///
    /// # Examples
    ///
    /// ```
    /// use archive::ArchiveFormat;
    ///
    /// assert_eq!(ArchiveFormat::Zip.name(), "ZIP");
    /// assert_eq!(ArchiveFormat::TarGz.name(), "TAR.GZ");
    /// assert_eq!(ArchiveFormat::SevenZ.name(), "7Z");
    /// ```
    pub fn name(&self) -> &'static str {
        match self {
            Self::Zip => "ZIP",
            Self::Tar => "TAR",
            Self::TarGz => "TAR.GZ",
            Self::TarBz2 => "TAR.BZ2",
            Self::TarXz => "TAR.XZ",
            Self::TarZst => "TAR.ZST",
            Self::TarLz4 => "TAR.LZ4",
            Self::Gz => "GZIP",
            Self::Bz2 => "BZIP2",
            Self::Xz => "XZ",
            Self::Lz4 => "LZ4",
            Self::Zst => "ZSTD",
            Self::SevenZ => "7Z",
        }
    }
}
