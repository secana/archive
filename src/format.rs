//! Archive format identification

/// Supported archive formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    /// ZIP archive
    Zip,
    /// Plain TAR archive
    Tar,
    /// TAR with gzip compression
    TarGz,
    /// TAR with bzip2 compression
    TarBz2,
    /// TAR with xz compression
    TarXz,
    /// TAR with zstd compression
    TarZst,
    /// TAR with lz4 compression
    TarLz4,
    /// Single gzip compressed file
    Gz,
    /// Single bzip2 compressed file
    Bz2,
    /// Single xz compressed file
    Xz,
    /// Single lz4 compressed file
    Lz4,
    /// Single zstd compressed file
    Zst,
    /// 7-Zip archive
    SevenZ,
}

impl ArchiveFormat {
    /// Get the format name as a string
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
