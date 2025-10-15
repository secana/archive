//! Archive extraction implementations

use crate::error::{ArchiveError, Result};
use crate::format::ArchiveFormat;
use std::io::{Cursor, Read};

/// Represents a file extracted from an archive
#[derive(Debug, Clone)]
pub struct ExtractedFile {
    /// Original path in the archive
    pub path: String,
    /// File contents
    pub data: Vec<u8>,
    /// Whether this is a directory
    pub is_directory: bool,
}

/// Main extractor that handles all archive formats
#[derive(Debug, Clone)]
pub struct ArchiveExtractor {
    max_file_size: usize,
    max_total_size: usize,
}

impl Default for ArchiveExtractor {
    fn default() -> Self {
        Self {
            max_file_size: 100 * 1024 * 1024,   // 100 MB per file
            max_total_size: 1024 * 1024 * 1024, // 1 GB total
        }
    }
}

impl ArchiveExtractor {
    /// Create a new archive extractor with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum file size (in bytes)
    pub fn with_max_file_size(mut self, size: usize) -> Self {
        self.max_file_size = size;
        self
    }

    /// Set maximum total extraction size (in bytes)
    pub fn with_max_total_size(mut self, size: usize) -> Self {
        self.max_total_size = size;
        self
    }

    /// Extract an archive based on its format
    pub fn extract(&self, data: &[u8], format: ArchiveFormat) -> Result<Vec<ExtractedFile>> {
        match format {
            ArchiveFormat::Zip => self.extract_zip(data),
            ArchiveFormat::Tar => self.extract_tar(data),
            ArchiveFormat::TarGz => self.extract_tar_gz(data),
            ArchiveFormat::TarBz2 => self.extract_tar_bz2(data),
            ArchiveFormat::TarXz => self.extract_tar_xz(data),
            ArchiveFormat::TarZst => self.extract_tar_zst(data),
            ArchiveFormat::TarLz4 => self.extract_tar_lz4(data),
            ArchiveFormat::SevenZ => self.extract_7z(data),
            ArchiveFormat::Gz => self.extract_single_gz(data),
            ArchiveFormat::Bz2 => self.extract_single_bz2(data),
            ArchiveFormat::Xz => self.extract_single_xz(data),
            ArchiveFormat::Lz4 => self.extract_single_lz4(data),
            ArchiveFormat::Zst => self.extract_single_zst(data),
        }
    }

    fn extract_zip(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let reader = Cursor::new(data);
        let mut archive = zip::ZipArchive::new(reader)?;
        let mut files = Vec::new();
        let mut total_size = 0usize;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let is_directory = file.is_dir();

            if !is_directory {
                let size = file.size() as usize;
                if size > self.max_file_size {
                    return Err(ArchiveError::FileTooLarge {
                        size,
                        limit: self.max_file_size,
                    });
                }

                total_size += size;
                if total_size > self.max_total_size {
                    return Err(ArchiveError::TotalSizeTooLarge {
                        size: total_size,
                        limit: self.max_total_size,
                    });
                }

                let mut contents = Vec::new();
                file.read_to_end(&mut contents)?;

                files.push(ExtractedFile {
                    path: file.name().to_string(),
                    data: contents,
                    is_directory,
                });
            } else {
                files.push(ExtractedFile {
                    path: file.name().to_string(),
                    data: Vec::new(),
                    is_directory,
                });
            }
        }

        Ok(files)
    }

    fn extract_tar(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let cursor = Cursor::new(data);
        let mut archive = tar::Archive::new(cursor);
        self.process_tar_entries(&mut archive)
    }

    fn extract_tar_gz(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let cursor = Cursor::new(data);
        let decoder = flate2::read::GzDecoder::new(cursor);
        let mut archive = tar::Archive::new(decoder);
        self.process_tar_entries(&mut archive)
    }

    fn extract_tar_bz2(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let cursor = Cursor::new(data);
        let decoder = bzip2::read::BzDecoder::new(cursor);
        let mut archive = tar::Archive::new(decoder);
        self.process_tar_entries(&mut archive)
    }

    fn extract_tar_xz(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let cursor = Cursor::new(data);
        let mut output = Vec::new();
        lzma_rs::xz_decompress(&mut cursor.clone(), &mut output)
            .map_err(|e| ArchiveError::InvalidArchive(e.to_string()))?;
        let cursor = Cursor::new(output);
        let mut archive = tar::Archive::new(cursor);
        self.process_tar_entries(&mut archive)
    }

    fn extract_tar_zst(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let cursor = Cursor::new(data);
        let decoder = zstd::stream::read::Decoder::new(cursor)?;
        let mut archive = tar::Archive::new(decoder);
        self.process_tar_entries(&mut archive)
    }

    fn extract_tar_lz4(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let cursor = Cursor::new(data);
        let decoder = lz4::Decoder::new(cursor)?;
        let mut archive = tar::Archive::new(decoder);
        self.process_tar_entries(&mut archive)
    }

    fn extract_7z(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        use std::io::Seek;

        let mut cursor = Cursor::new(data);
        let len = cursor.get_ref().len() as u64;

        let archive = sevenz_rust::SevenZReader::new(&mut cursor, len, "".into())
            .map_err(|e| ArchiveError::InvalidArchive(format!("7z error: {}", e)))?;

        let mut files = Vec::new();
        let mut total_size = 0usize;

        for entry in archive.archive().files.iter() {
            if entry.is_directory() {
                files.push(ExtractedFile {
                    path: entry.name().to_string(),
                    data: Vec::new(),
                    is_directory: true,
                });
            } else {
                let size = entry.size() as usize;
                if size > self.max_file_size {
                    return Err(ArchiveError::FileTooLarge {
                        size,
                        limit: self.max_file_size,
                    });
                }

                total_size += size;
                if total_size > self.max_total_size {
                    return Err(ArchiveError::TotalSizeTooLarge {
                        size: total_size,
                        limit: self.max_total_size,
                    });
                }

                files.push(ExtractedFile {
                    path: entry.name().to_string(),
                    data: Vec::new(), // Placeholder - we'll fill this below
                    is_directory: false,
                });
            }
        }

        // Re-read the archive to extract file contents
        cursor.seek(std::io::SeekFrom::Start(0))?;
        let mut archive = sevenz_rust::SevenZReader::new(&mut cursor, len, "".into())
            .map_err(|e| ArchiveError::InvalidArchive(format!("7z error: {}", e)))?;

        archive
            .for_each_entries(|entry, reader| {
                if !entry.is_directory() {
                    // Find the corresponding file in our list
                    if let Some(file) = files.iter_mut().find(|f| f.path == entry.name()) {
                        let mut contents = Vec::new();
                        reader.read_to_end(&mut contents)?;
                        file.data = contents;
                    }
                }
                Ok(true)
            })
            .map_err(|e| ArchiveError::InvalidArchive(format!("7z extraction error: {}", e)))?;

        Ok(files)
    }

    // Single-file decompression methods

    fn extract_single_gz(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let cursor = Cursor::new(data);
        let mut decoder = flate2::read::GzDecoder::new(cursor);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;

        if decompressed.len() > self.max_file_size {
            return Err(ArchiveError::FileTooLarge {
                size: decompressed.len(),
                limit: self.max_file_size,
            });
        }

        Ok(vec![ExtractedFile {
            path: "decompressed".to_string(),
            data: decompressed,
            is_directory: false,
        }])
    }

    fn extract_single_bz2(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let cursor = Cursor::new(data);
        let mut decoder = bzip2::read::BzDecoder::new(cursor);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;

        if decompressed.len() > self.max_file_size {
            return Err(ArchiveError::FileTooLarge {
                size: decompressed.len(),
                limit: self.max_file_size,
            });
        }

        Ok(vec![ExtractedFile {
            path: "decompressed".to_string(),
            data: decompressed,
            is_directory: false,
        }])
    }

    fn extract_single_xz(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let mut cursor = Cursor::new(data);
        let mut decompressed = Vec::new();
        lzma_rs::xz_decompress(&mut cursor, &mut decompressed)
            .map_err(|e| ArchiveError::InvalidArchive(e.to_string()))?;

        if decompressed.len() > self.max_file_size {
            return Err(ArchiveError::FileTooLarge {
                size: decompressed.len(),
                limit: self.max_file_size,
            });
        }

        Ok(vec![ExtractedFile {
            path: "decompressed".to_string(),
            data: decompressed,
            is_directory: false,
        }])
    }

    fn extract_single_lz4(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let cursor = Cursor::new(data);
        let mut decoder = lz4::Decoder::new(cursor)?;
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;

        if decompressed.len() > self.max_file_size {
            return Err(ArchiveError::FileTooLarge {
                size: decompressed.len(),
                limit: self.max_file_size,
            });
        }

        Ok(vec![ExtractedFile {
            path: "decompressed".to_string(),
            data: decompressed,
            is_directory: false,
        }])
    }

    fn extract_single_zst(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let cursor = Cursor::new(data);
        let mut decoder = zstd::stream::read::Decoder::new(cursor)?;
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;

        if decompressed.len() > self.max_file_size {
            return Err(ArchiveError::FileTooLarge {
                size: decompressed.len(),
                limit: self.max_file_size,
            });
        }

        Ok(vec![ExtractedFile {
            path: "decompressed".to_string(),
            data: decompressed,
            is_directory: false,
        }])
    }

    fn process_tar_entries<R: Read>(
        &self,
        archive: &mut tar::Archive<R>,
    ) -> Result<Vec<ExtractedFile>> {
        let mut files = Vec::new();
        let mut total_size = 0usize;

        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.to_string_lossy().to_string();
            let is_directory = entry.header().entry_type().is_dir();

            if !is_directory {
                let size = entry.size() as usize;
                if size > self.max_file_size {
                    return Err(ArchiveError::FileTooLarge {
                        size,
                        limit: self.max_file_size,
                    });
                }

                total_size += size;
                if total_size > self.max_total_size {
                    return Err(ArchiveError::TotalSizeTooLarge {
                        size: total_size,
                        limit: self.max_total_size,
                    });
                }

                let mut contents = Vec::new();
                entry.read_to_end(&mut contents)?;

                files.push(ExtractedFile {
                    path,
                    data: contents,
                    is_directory,
                });
            } else {
                files.push(ExtractedFile {
                    path,
                    data: Vec::new(),
                    is_directory,
                });
            }
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_limits() {
        let extractor = ArchiveExtractor::new();
        assert_eq!(extractor.max_file_size, 100 * 1024 * 1024);
        assert_eq!(extractor.max_total_size, 1024 * 1024 * 1024);
    }

    #[test]
    fn test_builder_pattern() {
        let extractor = ArchiveExtractor::new()
            .with_max_file_size(50 * 1024 * 1024)
            .with_max_total_size(500 * 1024 * 1024);

        assert_eq!(extractor.max_file_size, 50 * 1024 * 1024);
        assert_eq!(extractor.max_total_size, 500 * 1024 * 1024);
    }
}
