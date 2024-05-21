//! Wrapper around signals for platforms that they are not supported on

use std::{fmt::Display, str::FromStr};

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KillportSignal(pub nix::sys::signal::Signal);

/// On a platform where we don't have the proper signals enum
#[cfg(not(unix))]
#[derive(Debug, Clone)]
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
