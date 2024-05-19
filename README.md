<div align="center">
<a href="https://github.com/jkfran/jkfran.com/stargazers"><img src="https://img.shields.io/github/stars/jkfran/killport" alt="Stars Badge"/></a>
<a href="https://github.com/jkfran/jkfran.com/network/members"><img src="https://img.shields.io/github/forks/jkfran/killport" alt="Forks Badge"/></a>
<a href="https://github.com/jkfran/jkfran.com/pulls"><img src="https://img.shields.io/github/issues-pr/jkfran/killport" alt="Pull Requests Badge"/></a>
<a href="https://github.com/jkfran/jkfran.com/issues"><img src="https://img.shields.io/github/issues/jkfran/killport" alt="Issues Badge"/></a>
<a href="https://github.com/jkfran/jkfran.com/graphs/contributors"><img alt="GitHub contributors" src="https://img.shields.io/github/contributors/jkfran/killport?color=2b9348"></a>
<a href="https://github.com/jkfran/jkfran.com/blob/master/LICENSE"><img src="https://img.shields.io/github/license/jkfran/killport?color=2b9348" alt="License Badge"/></a>
</div>
<br>

# killport

`killport` is a command-line utility designed for efficiently terminating processes and containers listening on specified ports. It supports both single and multiple port operations, enhancing system management across Linux, macOS, and Windows platforms. Built with Rust, `killport` combines flexibility with performance in process management tasks.

## Features

- Terminate processes or containers on specified ports with support for single or multiple ports.
- Mode of operation options to target either processes, containers, or both.
- Dry-run capability for safe operations without actual termination.
- Adjustable verbosity for detailed logging and quiet operation for minimal output.
- Comprehensive signal support for fine-grained control over the termination signals sent to processes or containers.
- Cross-platform compatibility: Linux, macOS, and Windows.

## Installation

### Using Homebrew

Run the following command to install [killport](https://formulae.brew.sh/formula/killport) using [Homebrew](https://brew.sh/).

```sh
brew install killport
```

### Using install.sh

Run the following command to automatically download and install `killport`:

```sh
curl -sL https://bit.ly/killport | sh
```

Don't forget to add `$HOME/.local/bin` to your `PATH` environment variable, if it's not already present.

### Using cargo

Run the following command to install killport using cargo. If you don't have cargo, follow the [official Rust installation guide](https://www.rust-lang.org/tools/install).

```sh
cargo install killport
```

### Binary Releases

You can download the binary releases for different architectures from the [releases page](https://github.com/jkfran/killport/releases) and manually install them.

## Usage

```sh
killport [OPTIONS] <ports>...
```

### Flags

- `-m, --mode <MODE>`: Select mode of operation (process, container, or both).
- `-s, --signal <SIG>`: Specify the signal to send (default: SIGKILL).
- `-v, --verbose`: Increase verbosity level (use multiple times for more detail).
- `-q, --quiet`: Decrease verbosity level (use multiple times for less detail).
- `--dry-run`: Preview which processes or containers would be terminated.
- `-h, --help`: Display help message.
- `-V, --version`: Display version information.

### Examples

Kill a single process on port 8080:

```sh
killport 8080
```

Kill processes on multiple ports with a specific signal:

```sh
killport -s sigterm 8045 8046 8080
```

Perform a dry run to check what would be killed on port 8080:

```sh
killport --dry-run 8080
```

Supported Signals:

1. **Softest/Lower Preference Signals (Generally ignorable or default to terminate the process gently):**
   - `SIGUSR1` - User-defined signal 1
   - `SIGUSR2` - User-defined signal 2
   - `SIGWINCH` - Window size change
   - `SIGURG` - Urgent condition on socket
   - `SIGCONT` - Continue if stopped
   - `SIGCHLD` - Child status has changed
   - `SIGIO` - I/O now possible
   - `SIGALRM` - Timer signal
   - `SIGVTALRM` - Virtual timer expired
   - `SIGPROF` - Profiling timer expired
   - `SIGPWR` - Power failure
   - `SIGSYS` - Bad argument to routine
   - `SIGPIPE` - Broken pipe: write to pipe with no readers
   - `SIGTERM` - Termination signal
   - `SIGHUP` - Hangup detected on controlling terminal or death of controlling process
   - `SIGINT` - This signal is sent to a process by its controlling terminal when a user wishes to interrupt the process.

2. **Higher Preference/More Disruptive Signals (Generally not ignorable and often default to terminate the process abruptly):**
   - `SIGQUIT` - Quit from keyboard and dump core
   - `SIGABRT` - Abort signal from abort()
   - `SIGTSTP` - Stop typed at terminal
   - `SIGTTIN` - Terminal input for background process
   - `SIGTTOU` - Terminal output for background process
   - `SIGSTOP` - Stop process
   - `SIGSEGV` - Invalid memory reference
   - `SIGBUS` - Bus error (bad memory access)
   - `SIGFPE` - Floating-point exception
   - `SIGILL` - Illegal Instruction
   - `SIGTRAP` - Trace/breakpoint trap

3. **Most Severe/Definitive Signals (Cannot be caught or ignored):**
   - `SIGKILL` - Kill signal

## Contributing

We welcome contributions to the killport project! Before you start, please read our [Code of Conduct](CODE_OF_CONDUCT.md) and the [Contributing Guidelines](CONTRIBUTING.md).

To contribute, follow these steps:

1. Fork the repository on GitHub.
2. Clone your fork and create a new branch for your feature or bugfix.
3. Make your changes, following our coding guidelines.
4. Add tests for your changes and ensure all tests pass.
5. Commit your changes, following our commit message guidelines.
6. Push your changes to your fork and create a pull request.

We'll review your pull request and provide feedback or merge your changes.

## License

This project is licensed under the [MIT License](LICENSE). See the LICENSE file for more information.

