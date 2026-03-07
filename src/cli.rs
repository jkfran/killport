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

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Mode Display ────────────────────────────────────────────────────

    #[test]
    fn test_mode_display_auto() {
        assert_eq!(Mode::Auto.to_string(), "auto");
    }

    #[test]
    fn test_mode_display_process() {
        assert_eq!(Mode::Process.to_string(), "process");
    }

    #[test]
    fn test_mode_display_container() {
        assert_eq!(Mode::Container.to_string(), "container");
    }

    // ─── Service Descriptors ─────────────────────────────────────────────

    #[test]
    fn test_service_descriptors_auto() {
        assert_eq!(service_descriptors(Mode::Auto), ("service", "services"));
    }

    #[test]
    fn test_service_descriptors_process() {
        assert_eq!(service_descriptors(Mode::Process), ("process", "processes"));
    }

    #[test]
    fn test_service_descriptors_container() {
        assert_eq!(
            service_descriptors(Mode::Container),
            ("container", "containers")
        );
    }

    // ─── Signal Parsing ──────────────────────────────────────────────────

    #[test]
    fn test_parse_signal_sigkill() {
        assert!(parse_signal("sigkill").is_ok());
    }

    #[test]
    fn test_parse_signal_uppercase() {
        assert!(parse_signal("SIGTERM").is_ok());
    }

    #[test]
    fn test_parse_signal_mixed_case() {
        assert!(parse_signal("SigInt").is_ok());
    }

    #[test]
    fn test_parse_signal_invalid() {
        assert!(parse_signal("NOTASIGNAL").is_err());
    }

    #[test]
    fn test_parse_signal_empty() {
        assert!(parse_signal("").is_err());
    }

    // ─── CLI Argument Parsing ────────────────────────────────────────────

    #[test]
    fn test_cli_args_single_port() {
        let args = KillPortArgs::try_parse_from(["killport", "8080"]).unwrap();
        assert_eq!(args.ports, vec![8080]);
    }

    #[test]
    fn test_cli_args_multiple_ports() {
        let args = KillPortArgs::try_parse_from(["killport", "80", "443", "8080"]).unwrap();
        assert_eq!(args.ports, vec![80, 443, 8080]);
    }

    #[test]
    fn test_cli_args_no_ports_fails() {
        assert!(KillPortArgs::try_parse_from(["killport"]).is_err());
    }

    #[test]
    fn test_cli_args_port_zero() {
        let args = KillPortArgs::try_parse_from(["killport", "0"]).unwrap();
        assert_eq!(args.ports, vec![0]);
    }

    #[test]
    fn test_cli_args_port_65535() {
        let args = KillPortArgs::try_parse_from(["killport", "65535"]).unwrap();
        assert_eq!(args.ports, vec![65535]);
    }

    #[test]
    fn test_cli_args_port_overflow() {
        assert!(KillPortArgs::try_parse_from(["killport", "65536"]).is_err());
    }

    #[test]
    fn test_cli_args_port_negative() {
        assert!(KillPortArgs::try_parse_from(["killport", "-1"]).is_err());
    }

    #[test]
    fn test_cli_args_port_string() {
        assert!(KillPortArgs::try_parse_from(["killport", "abc"]).is_err());
    }

    #[test]
    fn test_cli_args_mode_auto() {
        let args = KillPortArgs::try_parse_from(["killport", "8080", "--mode", "auto"]).unwrap();
        assert_eq!(args.mode, Mode::Auto);
    }

    #[test]
    fn test_cli_args_mode_process() {
        let args = KillPortArgs::try_parse_from(["killport", "8080", "--mode", "process"]).unwrap();
        assert_eq!(args.mode, Mode::Process);
    }

    #[test]
    fn test_cli_args_mode_container() {
        let args =
            KillPortArgs::try_parse_from(["killport", "8080", "--mode", "container"]).unwrap();
        assert_eq!(args.mode, Mode::Container);
    }

    #[test]
    fn test_cli_args_mode_invalid() {
        assert!(KillPortArgs::try_parse_from(["killport", "8080", "--mode", "foobar"]).is_err());
    }

    #[test]
    fn test_cli_args_mode_short_flag() {
        let args = KillPortArgs::try_parse_from(["killport", "8080", "-m", "process"]).unwrap();
        assert_eq!(args.mode, Mode::Process);
    }

    #[test]
    fn test_cli_args_mode_default() {
        let args = KillPortArgs::try_parse_from(["killport", "8080"]).unwrap();
        assert_eq!(args.mode, Mode::Auto);
    }

    #[test]
    fn test_cli_args_signal_default() {
        let args = KillPortArgs::try_parse_from(["killport", "8080"]).unwrap();
        assert_eq!(args.signal.to_string().to_uppercase(), "SIGKILL");
    }

    #[test]
    fn test_cli_args_signal_custom() {
        let args = KillPortArgs::try_parse_from(["killport", "8080", "-s", "sigterm"]).unwrap();
        assert_eq!(args.signal.to_string().to_uppercase(), "SIGTERM");
    }

    #[test]
    fn test_cli_args_dry_run_present() {
        let args = KillPortArgs::try_parse_from(["killport", "8080", "--dry-run"]).unwrap();
        assert!(args.dry_run);
    }

    #[test]
    fn test_cli_args_dry_run_absent() {
        let args = KillPortArgs::try_parse_from(["killport", "8080"]).unwrap();
        assert!(!args.dry_run);
    }

    #[test]
    fn test_cli_args_combined_flags() {
        let args = KillPortArgs::try_parse_from([
            "killport",
            "-m",
            "process",
            "-s",
            "sigterm",
            "--dry-run",
            "8080",
            "8081",
        ])
        .unwrap();
        assert_eq!(args.ports, vec![8080, 8081]);
        assert_eq!(args.mode, Mode::Process);
        assert_eq!(args.signal.to_string().to_uppercase(), "SIGTERM");
        assert!(args.dry_run);
    }

    #[test]
    fn test_cli_args_help() {
        let result = KillPortArgs::try_parse_from(["killport", "--help"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
    }

    #[test]
    fn test_cli_args_version() {
        let result = KillPortArgs::try_parse_from(["killport", "--version"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
    }

    #[test]
    fn test_mode_clone() {
        let mode = Mode::Auto;
        let cloned = mode;
        assert_eq!(mode, cloned);
    }

    #[test]
    fn test_mode_debug() {
        let debug_str = format!("{:?}", Mode::Auto);
        assert_eq!(debug_str, "Auto");
    }

    #[test]
    fn test_mode_equality() {
        assert_eq!(Mode::Auto, Mode::Auto);
        assert_ne!(Mode::Auto, Mode::Process);
        assert_ne!(Mode::Process, Mode::Container);
    }
}
