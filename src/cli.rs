use clap::{Parser, ValueEnum};
use clap_verbosity_flag::{Verbosity, WarnLevel};
use core::fmt;

use crate::signal::KillportSignal;

/// Modes of operation for killport.
#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
pub enum Mode {
    Auto,
    Process,
    Container,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let variant = match *self {
            Mode::Auto => "auto",
            Mode::Process => "process",
            Mode::Container => "container",
        };
        write!(f, "{}", variant)
    }
}

/// Returns appropriate service descriptors based on the mode.
///
/// # Arguments
/// * `mode` - The mode of operation.
///
/// # Returns
/// * `(singular, plural)` - Tuple containing singular and plural forms of the service description.
pub fn service_descriptors(mode: Mode) -> (&'static str, &'static str) {
    match mode {
        Mode::Auto => ("service", "services"),
        Mode::Process => ("process", "processes"),
        Mode::Container => ("container", "containers"),
    }
}

/// `killport` utility.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct KillPortArgs {
    /// A list of port numbers to kill processes on.
    #[arg(
        name = "ports",
        help = "The list of port numbers to kill processes or containers on",
        required = true
    )]
    pub ports: Vec<u16>,

    /// Operation mode.
    #[arg(
        long,
        short = 'm',
        help = "Mode of operation: auto (default, kill both), process (only processes), container (only containers)",
        default_value_t = Mode::Auto)]
    pub mode: Mode,

    /// An option to specify the type of signal to be sent.
    #[arg(
        long,
        short = 's',
        name = "SIG",
        help = "SIG is a signal name",
        default_value = "sigkill",
        value_parser = parse_signal
    )]
    pub signal: KillportSignal,

    /// A verbosity flag to control the level of logging output.
    #[command(flatten)]
    pub verbose: Verbosity<WarnLevel>,

    /// Dry-run flag to only display what would be done without taking action.
    #[arg(
        long,
        help = "Perform a dry run without killing any processes or containers"
    )]
    pub dry_run: bool,
}

fn parse_signal(arg: &str) -> Result<KillportSignal, std::io::Error> {
    arg.to_uppercase().parse()
}
