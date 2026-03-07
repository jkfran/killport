//! Wrapper around signals for platforms that they are not supported on

use std::{fmt::Display, str::FromStr};

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KillportSignal(pub nix::sys::signal::Signal);

/// On a platform where we don't have the proper signals enum
#[cfg(not(unix))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KillportSignal(pub String);

impl Display for KillportSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl FromStr for KillportSignal {
    type Err = std::io::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        #[cfg(unix)]
        {
            let signal = nix::sys::signal::Signal::from_str(value)?;
            Ok(KillportSignal(signal))
        }

        #[cfg(not(unix))]
        {
            Ok(KillportSignal(value.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    mod unix_tests {
        use super::super::*;
        use nix::sys::signal::Signal;

        #[test]
        fn test_signal_from_str_sigkill() {
            let signal: KillportSignal = "SIGKILL".parse().unwrap();
            assert_eq!(signal.0, Signal::SIGKILL);
        }

        #[test]
        fn test_signal_from_str_sigterm() {
            let signal: KillportSignal = "SIGTERM".parse().unwrap();
            assert_eq!(signal.0, Signal::SIGTERM);
        }

        #[test]
        fn test_signal_from_str_sigint() {
            let signal: KillportSignal = "SIGINT".parse().unwrap();
            assert_eq!(signal.0, Signal::SIGINT);
        }

        #[test]
        fn test_signal_from_str_sighup() {
            let signal: KillportSignal = "SIGHUP".parse().unwrap();
            assert_eq!(signal.0, Signal::SIGHUP);
        }

        #[test]
        fn test_signal_from_str_sigusr1() {
            let signal: KillportSignal = "SIGUSR1".parse().unwrap();
            assert_eq!(signal.0, Signal::SIGUSR1);
        }

        #[test]
        fn test_signal_from_str_sigusr2() {
            let signal: KillportSignal = "SIGUSR2".parse().unwrap();
            assert_eq!(signal.0, Signal::SIGUSR2);
        }

        #[test]
        fn test_signal_from_str_sigquit() {
            let signal: KillportSignal = "SIGQUIT".parse().unwrap();
            assert_eq!(signal.0, Signal::SIGQUIT);
        }

        #[test]
        fn test_signal_from_str_sigabrt() {
            let signal: KillportSignal = "SIGABRT".parse().unwrap();
            assert_eq!(signal.0, Signal::SIGABRT);
        }

        #[test]
        fn test_signal_from_str_sigalrm() {
            let signal: KillportSignal = "SIGALRM".parse().unwrap();
            assert_eq!(signal.0, Signal::SIGALRM);
        }

        #[test]
        fn test_signal_from_str_sigpipe() {
            let signal: KillportSignal = "SIGPIPE".parse().unwrap();
            assert_eq!(signal.0, Signal::SIGPIPE);
        }

        #[test]
        fn test_signal_from_str_invalid() {
            assert!("INVALID".parse::<KillportSignal>().is_err());
        }

        #[test]
        fn test_signal_from_str_empty() {
            assert!("".parse::<KillportSignal>().is_err());
        }

        #[test]
        fn test_signal_from_str_lowercase_fails() {
            // nix requires uppercase signal names
            assert!("sigkill".parse::<KillportSignal>().is_err());
        }

        #[test]
        fn test_signal_display() {
            let signal = KillportSignal(Signal::SIGKILL);
            assert_eq!(signal.to_string(), "SIGKILL");
        }

        #[test]
        fn test_signal_display_sigterm() {
            let signal = KillportSignal(Signal::SIGTERM);
            assert_eq!(signal.to_string(), "SIGTERM");
        }

        #[test]
        fn test_signal_clone() {
            let signal = KillportSignal(Signal::SIGKILL);
            let cloned = signal.clone();
            assert_eq!(signal, cloned);
        }

        #[test]
        fn test_signal_eq() {
            let a = KillportSignal(Signal::SIGKILL);
            let b = KillportSignal(Signal::SIGKILL);
            assert_eq!(a, b);
        }

        #[test]
        fn test_signal_ne() {
            let a = KillportSignal(Signal::SIGKILL);
            let b = KillportSignal(Signal::SIGTERM);
            assert_ne!(a, b);
        }

        #[test]
        fn test_signal_debug() {
            let signal = KillportSignal(Signal::SIGKILL);
            let debug_str = format!("{:?}", signal);
            assert!(debug_str.contains("SIGKILL"));
        }
    }
}
