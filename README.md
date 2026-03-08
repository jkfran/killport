<div align="center">

# killport

A command-line tool to kill processes and containers running on specified ports.

[![Crates.io](https://img.shields.io/crates/v/killport)](https://crates.io/crates/killport)
[![GitHub Stars](https://img.shields.io/github/stars/jkfran/killport)](https://github.com/jkfran/killport/stargazers)
[![License](https://img.shields.io/github/license/jkfran/killport)](LICENSE)
[![CI](https://github.com/jkfran/killport/actions/workflows/ci.yml/badge.svg)](https://github.com/jkfran/killport/actions/workflows/ci.yml)

</div>

## Features

- Kill processes and containers by port number (single or multiple ports)
- **Container runtime support** — works with Docker, OrbStack, Podman, Colima, and any OCI-compatible runtime
- **Dry-run mode** — preview what would be killed without doing it
- **Signal control** — send any signal (SIGTERM, SIGKILL, SIGINT, etc.)
- **Shell completions** — built-in support for bash, zsh, and fish
- **Cross-platform** — Linux, macOS, and Windows

## Installation

### Homebrew

```sh
brew install killport
```

### Cargo

```sh
cargo install killport
```

### Shell script

```sh
curl -sL https://bit.ly/killport | sh
```

Add `$HOME/.local/bin` to your `PATH` if it's not already there.

### Binary releases

Download pre-built binaries from the [releases page](https://github.com/jkfran/killport/releases). Each release includes shell completions and a man page.

## Usage

```sh
killport [OPTIONS] <ports>...
```

### Examples

Kill whatever is using port 8080:

```sh
killport 8080
```

Kill multiple ports at once:

```sh
killport 3000 8080 9090
```

Send SIGTERM instead of SIGKILL:

```sh
killport -s sigterm 8080
```

Dry run — see what would be killed:

```sh
killport --dry-run 8080
# Would kill process 'node' listening on port 8080
```

Kill only containers (skip native processes):

```sh
killport --mode container 8080
```

Suppress exit code 2 when nothing is found (useful in scripts):

```sh
killport --no-fail 8080
```

### Options

| Flag | Description |
|---|---|
| `-m, --mode <MODE>` | Target mode: `auto` (default), `process`, or `container` |
| `-s, --signal <SIG>` | Signal to send (default: `SIGKILL`). See [supported signals](SIGNALS.md) |
| `--dry-run` | Preview what would be killed without terminating anything |
| `--no-fail` | Exit 0 even when no matching process or container is found |
| `-v, --verbose` | Increase verbosity (repeat for more: `-vv`, `-vvv`) |
| `-q, --quiet` | Decrease verbosity (repeat for less: `-qq`) |
| `-h, --help` | Show help |
| `-V, --version` | Show version |

### Exit codes

| Code | Meaning |
|---|---|
| `0` | Target(s) found and killed (or `--no-fail` was used) |
| `1` | Error (permission denied, internal failure, etc.) |
| `2` | No matching process or container found on the specified port(s) |

### How it works

In **auto mode** (default), killport:

1. Checks if a container runtime is available
2. If containers are found on the port, kills only the containers (port forwarder processes like `docker-proxy` or `OrbStack Helper` are automatically skipped)
3. If no containers are found, falls back to killing native processes

Use `--mode process` or `--mode container` to target only one type.

## Shell completions

Shell completions are included in release tarballs under `completions/`. To install:

**bash** — copy to `~/.local/share/bash-completion/completions/killport`

**zsh** — copy to a directory in your `$fpath` (e.g., `~/.zsh/completions/_killport`)

**fish** — copy to `~/.config/fish/completions/killport.fish`

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## License

[MIT](LICENSE)
