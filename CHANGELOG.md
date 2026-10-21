# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0](https://github.com/secana/archive/compare/v0.4.0...v1.0.0) - 2026-10-21

### Fixed

- [**breaking**] bound entry memory, allow in-root symlinks and add HardLink variant

### Other

- *(deps)* [**breaking**] bump zip from 8.6.0 to 9.0.5

## [0.4.0](https://github.com/secana/archive/compare/v0.3.2...v0.4.0) - 2026-10-21

### Added

- [**breaking**] harden extraction against path traversal and decompression bombs ([#20](https://github.com/secana/archive/pull/20))

### Fixed

- fix possible size bombs by misidentified header size
- repair broken doc link and apply cargo fmt

### Other

- update flake.nix
- *(deps)* bump zstd from 0.14.0 to 0.14.2
- *(deps)* bump thiserror from 2.0.20 to 2.0.21
- *(deps)* bump zstd from 0.13.3 to 0.14.0
- *(deps)* bump flate2 from 1.1.9 to 1.1.10
- *(deps)* bump thiserror from 2.0.19 to 2.0.20
- *(deps)* bump thiserror from 2.0.18 to 2.0.19
- *(deps)* bump actions/checkout from 6 to 7
- *(deps)* bump tar from 0.4.44 to 0.4.46
- *(deps)* bump bzip2 from 0.6.0 to 0.6.1
- *(deps)* bump flate2 from 1.1.4 to 1.1.9
- *(deps)* bump zip from 8.5.1 to 8.6.0
- *(deps)* bump thiserror from 2.0.17 to 2.0.18

## [0.3.2](https://github.com/secana/archive/compare/v0.3.1...v0.3.2) - 2026-06-14

### Other

- auto-approve and auto-merge Dependabot PRs

## [0.3.1](https://github.com/secana/archive/compare/v0.3.0...v0.3.1) - 2026-04-25

### Other

- dependency update

## [0.3.0](https://github.com/secana/archive/compare/v0.2.1...v0.3.0) - 2025-12-28

### Other

- Cleanup old artifacts
- Add unix archive (`.ar`) and Debian package (`.deb`) support ([#3](https://github.com/secana/archive/pull/3))
- Bump actions/checkout from 4 to 6 ([#6](https://github.com/secana/archive/pull/6))
- Bump actions/checkout from 4 to 6
- Add release-plz
