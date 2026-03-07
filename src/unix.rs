use crate::killable::{Killable, KillableType};
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
            Error::other(format!(
                "Failed to kill process '{}' with PID {}: {}",
                self.name, self.pid, e
            ))
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
    ///   and for a `DockerContainer` it will return "container".
    fn get_type(&self) -> KillableType {
        KillableType::Process
    }

    fn get_name(&self) -> String {
        self.name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nix::sys::signal::Signal;

    #[test]
    fn test_unix_process_new() {
        let pid = Pid::from_raw(1234);
        let process = UnixProcess::new(pid, "test_process".to_string());
        assert_eq!(process.get_name(), "test_process");
        assert_eq!(process.get_type(), KillableType::Process);
    }

    #[test]
    fn test_unix_process_get_type() {
        let process = UnixProcess::new(Pid::from_raw(1), "test".to_string());
        assert_eq!(process.get_type(), KillableType::Process);
    }

    #[test]
    fn test_unix_process_get_name() {
        let process = UnixProcess::new(Pid::from_raw(1), "my_process".to_string());
        assert_eq!(process.get_name(), "my_process");
    }

    #[test]
    fn test_unix_process_get_name_empty() {
        let process = UnixProcess::new(Pid::from_raw(1), String::new());
        assert_eq!(process.get_name(), "");
    }

    #[test]
    fn test_unix_process_get_name_special_chars() {
        let process = UnixProcess::new(Pid::from_raw(1), "/usr/bin/my process --flag".to_string());
        assert_eq!(process.get_name(), "/usr/bin/my process --flag");
    }

    #[test]
    fn test_unix_process_kill_success() {
        // Spawn a child process that we can kill
        use std::process::Command;
        let mut child = Command::new("sleep")
            .arg("60")
            .spawn()
            .expect("Failed to spawn sleep process");
        let pid = child.id() as i32;

        let process = UnixProcess::new(Pid::from_raw(pid), "sleep".to_string());
        let result = process.kill(KillportSignal(Signal::SIGTERM));
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Wait for child to fully exit (avoids zombie process)
        let _ = child.wait();
    }

    #[test]
    fn test_unix_process_kill_nonexistent_pid() {
        // Use a very high PID that is unlikely to exist
        let process = UnixProcess::new(Pid::from_raw(4_000_000), "nonexistent".to_string());
        let result = process.kill(KillportSignal(Signal::SIGTERM));
        assert!(result.is_err());
    }

    #[test]
    fn test_unix_process_kill_permission_denied() {
        // PID 1 (init/launchd) should return EPERM for non-root users
        let process = UnixProcess::new(Pid::from_raw(1), "init".to_string());
        let result = process.kill(KillportSignal(Signal::SIGTERM));
        assert!(result.is_err());
    }

    #[test]
    fn test_unix_process_debug() {
        let process = UnixProcess::new(Pid::from_raw(42), "test".to_string());
        let debug_str = format!("{:?}", process);
        assert!(debug_str.contains("42"));
        assert!(debug_str.contains("test"));
    }
}
