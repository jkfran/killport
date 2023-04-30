//! The `killport` command-line utility is designed to kill processes
//! listening on specified ports.
//!
//! The utility accepts a list of port numbers as input and attempts to
//! terminate any processes listening on those ports.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
use linux::kill_processes_by_port;
#[cfg(target_os = "macos")]
use macos::kill_processes_by_port;

use clap::{Parser, ValueEnum};
use clap_verbosity_flag::{Verbosity, WarnLevel};
use log::error;
use std::process::exit;

/// The `KillPortArgs` struct is used to parse command-line arguments for the
/// `killport` utility.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum KillPortSigSpecOptions {
    SIGKILL,
    SIGTERM,
}

/// The `KillPortArgs` struct is used to parse command-line arguments for the
/// `killport` utility.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct KillPortArgs {
    /// A list of port numbers to kill processes on.
    #[arg(
        name = "ports",
        help = "The list of port numbers to kill processes on",
        required = true
    )]
    ports: Vec<u16>,

    /// A verbosity flag to control the level of logging output.
    #[command(flatten)]
    verbose: Verbosity<WarnLevel>,
}

/// The `main` function is the entry point of the `killport` utility.
///
/// It parses command-line arguments, sets up the logging environment, and
/// attempts to kill processes listening on the specified ports.
fn main() {
    // Parse command-line arguments
    let args = KillPortArgs::parse();

    // Set up logging environment
    let log_level = args
        .verbose
        .log_level()
        .map(|level| level.to_level_filter())
        .unwrap();

    env_logger::Builder::new()
        .format_module_path(log_level == log::LevelFilter::Trace)
        .format_target(log_level == log::LevelFilter::Trace)
        .format_timestamp(Option::None)
        .filter_level(log_level)
        .init();

    // Attempt to kill processes listening on specified ports
    for port in args.ports {
        match kill_processes_by_port(port) {
            Ok(killed) => {
                if killed {
                    println!("Successfully killed process listening on port {}", port);
                } else {
                    println!("No processes found using port {}", port);
                }
            }
            Err(err) => {
                error!("{}", err);
                exit(1);
            }
        }
    }
}
