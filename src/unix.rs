use crate::killport::{Killable, KillableType};
use crate::signal::KillportSignal;
use log::info;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use std::io::Error;

/// Process type shared amongst unix-like operating systems
#[derive(Debug)]
pub struct UnixProcess {
    /// System native process ID.
    pid: Pid,
    name: String,
}

impl UnixProcess {
    pub fn new(pid: Pid, name: String) -> Self {
        Self { pid, name }
    }
}

impl Killable for UnixProcess {
    /// Entry point to kill the linux native process.
    ///
    /// # Arguments
    ///
    /// * `signal` - A enum value representing the signal type.
    fn kill(&self, signal: KillportSignal) -> Result<bool, Error> {
        info!("Killing process '{}' with PID {}", self.name, self.pid);

        kill(self.pid, signal.0).map(|_| true).map_err(|e| {
            Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "Failed to kill process '{}' with PID {}: {}",
                    self.name, self.pid, e
                ),
            )
        })
    }

    /// Returns the type of the killable target.
    ///
    /// This method is used to identify the type of the target (either a native process or a Docker container)
    /// that is being handled. This information can be useful for logging, error handling, or other needs
    /// where type of the target is relevant.
    ///
    /// # Returns
    ///
    /// * `String` - A string that describes the type of the killable target. For a `UnixProcess` it will return "process",
    /// and for a `DockerContainer` it will return "container".
    fn get_type(&self) -> KillableType {
        KillableType::Process
    }

    fn get_name(&self) -> String {
        self.name.to_string()
    }
}
