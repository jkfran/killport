# killport

`killport` is a command-line utility for killing processes listening on specific ports. It's designed to be simple, fast, and effective. The tool is built with Rust and works on Linux and macOS.

## Features

- Kill processes by port number
- Supports multiple port numbers
- Verbosity control
- Works on Linux and macOS

## Installation

### Using Homebrew

Run the following command to install killport using Homebrew.

```sh
brew tap jkfran/killport
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
killport [FLAGS] <ports>...
```

### Examples

Kill a single process listening on port 8080:

```sh
killport 8080
```

Kill multiple processes listening on ports 8045, 8046, and 8080:

```sh
killport 8045 8046 8080
```

Kill processes with specified signal:

```sh
killport -s sigterm 8080
```

### Flags

-s, --sigspec
    Specify a signal name to be sent. (e.g. sigterm)

-v, --verbose
    Increase the verbosity level. Use multiple times for more detailed output.

-h, --help
    Display the help message and exit.

-V, --version
    Display the version information and exit.

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

