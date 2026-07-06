# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [2.0.1] - 2026-07-06

### Dependencies

- build(deps): bump bollard from 0.20.2 to 0.21.0 (#48)
- build(deps): bump clap_mangen from 0.2.31 to 0.3.0 (#49)
- build(deps): bump the cargo-minor-patch group with 10 updates (#47)
- Removing android-tzdata v0.1.1
- Updating anstyle v1.0.13 -> v1.0.14
- Removing anyhow v1.0.102
- Updating autocfg v1.5.0 -> v1.5.1
- Removing bitflags v2.11.0
- Adding bitflags v1.3.2
- Adding bitflags v2.13.0
- Updating bstr v1.9.1 -> v1.12.3
- Updating bumpalo v3.16.0 -> v3.20.3
- Updating bytes v1.11.1 -> v1.12.0
- Updating cc v1.2.56 -> v1.2.66
- Updating chrono v0.4.38 -> v0.4.45
- Updating clang-sys v1.7.0 -> v1.8.1
- Updating clap_lex v1.0.0 -> v1.1.0
- Updating colorchoice v1.0.4 -> v1.0.5
- Adding defmt v1.1.1
- Adding defmt-macros v1.1.1
- Adding defmt-parser v1.0.0
- Updating displaydoc v0.2.5 -> v0.2.6
- Updating either v1.15.0 -> v1.16.0
- Removing equivalent v1.0.2
- Updating fastrand v2.3.0 -> v2.4.1
- Removing foldhash v0.1.5
- Updating getrandom v0.4.2 -> v0.4.3
- Updating glob v0.3.1 -> v0.3.3
- Removing hashbrown v0.15.5
- Removing hashbrown v0.16.1
- Updating http v1.4.0 -> v1.4.2
- Updating hyper v1.8.1 -> v1.10.1
- Updating iana-time-zone v0.1.60 -> v0.1.65
- Updating icu_collections v2.1.1 -> v2.2.0
- Updating icu_locale_core v2.1.1 -> v2.2.0
- Updating icu_normalizer v2.1.1 -> v2.2.0
- Updating icu_normalizer_data v2.1.1 -> v2.2.0
- Updating icu_properties v2.1.2 -> v2.2.0
- Updating icu_properties_data v2.1.2 -> v2.2.0
- Updating icu_provider v2.1.1 -> v2.2.0
- Removing id-arena v2.3.0
- Updating idna_adapter v1.2.1 -> v1.2.2
- ...and 72 more transitive updates

## [2.0.0] - 2026-03-08

### Added
- Shell completions for bash, zsh, and fish (generated at build time)
- Man page generation at build time
- `--no-fail` flag to exit successfully even when no process is found
- SIGPIPE handling for clean pipe behavior (e.g., `killport 8080 | head`)
- PID deduplication on Linux and macOS (prevents duplicate kill attempts)
- IPv6 and UDP test coverage on macOS
- CHANGELOG.md

### Changed
- **Breaking**: Exit code 2 when no matching process or container is found (was 0). Use `--no-fail` to restore previous behavior
- Updated Cargo.toml keywords for better discoverability
- Renamed Docker-specific types to generic container types (works with OrbStack, Podman, etc.)
- Container runtime detection: skip native processes when containers own the port (fixes OrbStack crash)

### Fixed
- Duplicate process entries when a process listens on both IPv4 and IPv6
- Killing container runtime's port forwarder process (e.g., OrbStack Helper) instead of the container

## [1.1.0] - 2024-11-20

### Added
- Docker container support: kill containers by port
- `--mode` flag: `auto`, `process`, `container`
- `--dry-run` flag for safe inspection
- Docker daemon protection (filters docker-proxy, dockerd processes)
- Verbosity control with `-v`/`-q` flags
- Integration tests with mock process compilation

### Changed
- Default signal changed to SIGKILL (was SIGTERM)

## [1.0.0] - 2024-06-15

### Added
- Windows support via `windows-sys` crate
- Windows process enumeration using `GetExtendedTcpTable`/`GetExtendedUdpTable`
- Cross-platform signal abstraction (`KillportSignal`)
- macOS CI testing

### Changed
- Restructured codebase with platform-specific modules

## [0.9.2] - 2023-11-10

### Fixed
- Security: bumped rustix dependency

### Changed
- Updated snapcraft configuration

## [0.9.1] - 2023-10-15

### Changed
- Updated all dependencies
- Windows memory management improvements (proper buffer reallocation)
- Replaced repetitive `get_processes` code with macro

## [0.9.0] - 2023-09-20

### Added
- Windows support (initial)
- Parent process traversal on Windows
- Windows release builds in CI

## [0.8.0] - 2023-07-10

### Fixed
- IPv6 error handling for TCP/UDP entries (avoid unwrap panics)

## [0.7.0] - 2023-05-15

### Added
- Signal specification support (`-s`/`--signal` flag)
- Support for SIGTERM, SIGINT, SIGHUP, and other Unix signals

## [0.6.0] - 2023-03-20

### Changed
- Improved release packaging
- Removed deb packaging

## [0.5.0] - 2023-02-15

### Added
- macOS support via `libproc` crate
- UDP port detection
- Install script for macOS users

## [0.1.0] - 2023-01-10

### Added
- Initial release
- Linux process detection via `/proc` filesystem
- TCP port killing
- Snap package support
- Basic CI/CD pipeline

[Unreleased]: https://github.com/jkfran/killport/compare/v2.0.0...HEAD
[2.0.0]: https://github.com/jkfran/killport/compare/v1.1.0...v2.0.0
[1.1.0]: https://github.com/jkfran/killport/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/jkfran/killport/compare/v0.9.2...v1.0.0
[0.9.2]: https://github.com/jkfran/killport/compare/v0.9.1...v0.9.2
[0.9.1]: https://github.com/jkfran/killport/compare/v0.9.0...v0.9.1
[0.9.0]: https://github.com/jkfran/killport/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/jkfran/killport/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/jkfran/killport/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/jkfran/killport/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/jkfran/killport/compare/v0.1.0...v0.5.0
[0.1.0]: https://github.com/jkfran/killport/releases/tag/v0.1.0
