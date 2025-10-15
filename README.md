# archive

[![Crates.io](https://img.shields.io/crates/v/archive.svg)](https://crates.io/crates/archive)
[![Documentation](https://docs.rs/archive/badge.svg)](https://docs.rs/archive)
[![CI](https://github.com/secana/archive/workflows/Archive%20CI/badge.svg)](https://github.com/secana/archive/actions)
[![License: MIT OR Apache-2.0](https://img.shields.io/crates/l/archive.svg)](#license)

A unified, pure-Rust interface for extracting common archive formats in-memory.

    This crate is currently in development and should not be used in production.
    The API may change in future releases.

## Features

- **Unified API**: Single interface for all archive formats
- **In-memory extraction**: No disk I/O required
- **Safety limits**: Protection against zip bombs and resource exhaustion
- **Pure Rust**: Minimal C dependencies (only bzip2)
- **Cross-platform**: Works on Linux, macOS, Windows (x86_64, ARM64)

### Supported Formats

| Format | Extensions | Description |
|--------|------------|-------------|
| **ZIP** | `.zip` | ZIP archives with various compression levels |
| **TAR** | `.tar` | Uncompressed TAR archives |
| **TAR.GZ** | `.tar.gz`, `.tgz` | TAR with gzip compression |
| **TAR.BZ2** | `.tar.bz2`, `.tbz2` | TAR with bzip2 compression |
| **TAR.XZ** | `.tar.xz`, `.txz` | TAR with xz/LZMA compression |
| **TAR.ZST** | `.tar.zst` | TAR with Zstandard compression |
| **TAR.LZ4** | `.tar.lz4` | TAR with LZ4 compression |
| **7-Zip** | `.7z` | 7-Zip archives |
| **Single-file** | `.gz`, `.bz2`, `.xz`, `.lz4`, `.zst` | Individual compressed files |

## Publish new version

To publish a new version of the crate, checkout the `main` branch and run:

```sh
nix run .#publish <version>

# Example
nix run .#publish 0.4.0
```

This will:
- Update the version in `Cargo.toml` and `Cargo.lock`
- Run tests and build the documentation
- Create a git tag for the new version
- Trigger a GitHub Actions workflow to publish the crate to crates.io 
