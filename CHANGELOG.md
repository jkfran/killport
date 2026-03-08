# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Shell completions for bash, zsh, and fish (generated at build time)
- Man page generation at build time
- `--no-fail` flag to exit successfully even when no process is found
- SIGPIPE handling for clean pipe behavior (e.g., `killport 8080 | head`)
- PID deduplication on Linux and macOS (prevents duplicate kill attempts)
- IPv6 and UDP test coverage on macOS
- CHANGELOG.md

### Changed
- Exit code 2 when no matching process or container is found (was 0)
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

[Unreleased]: https://github.com/jkfran/killport/compare/v1.1.0...HEAD
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
