use crate::cli::Mode;
use crate::docker::DockerContainer;
#[cfg(target_os = "linux")]
use crate::linux::find_target_processes;
#[cfg(target_os = "macos")]
use crate::macos::find_target_processes;
use log::info;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::io::Error;

#[derive(Debug)]
pub struct NativeProcess {
    /// System native process ID.
    pub pid: Pid,
    pub name: String,
}

/// Interface for killable targets such as native process and docker container.
pub trait Killable {
    fn kill(&self, signal: Signal) -> Result<bool, Error>;
    fn get_type(&self) -> String;
    fn get_name(&self) -> String;
}

impl Killable for NativeProcess {
    /// Entry point to kill the linux native process.
    ///
    /// # Arguments
    ///
    /// * `signal` - A enum value representing the signal type.
    fn kill(&self, signal: Signal) -> Result<bool, Error> {
        info!("Killing process '{}' with PID {}", self.name, self.pid);

        kill(self.pid, signal).map(|_| true).map_err(|e| {
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
    /// * `String` - A string that describes the type of the killable target. For a `NativeProcess` it will return "process",
    /// and for a `DockerContainer` it will return "container".
    fn get_type(&self) -> String {
        "process".to_string()
    }

    fn get_name(&self) -> String {
        self.name.to_string()
    }
}

impl Killable for DockerContainer {
    /// Entry point to kill the docker containers.
    ///
    /// # Arguments
    ///
    /// * `signal` - A enum value representing the signal type.
    fn kill(&self, signal: Signal) -> Result<bool, Error> {
        Self::kill_container(&self.name, signal)?;
        Ok(true)
    }

    /// Returns the type of the killable target.
    ///
    /// This method is used to identify the type of the target (either a native process or a Docker container)
    /// that is being handled. This information can be useful for logging, error handling, or other needs
    /// where type of the target is relevant.
    ///
    /// # Returns
    ///
    /// * `String` - A string that describes the type of the killable target. For a `NativeProcess` it will return "process",
    /// and for a `DockerContainer` it will return "container".
    fn get_type(&self) -> String {
        "container".to_string()
    }

    fn get_name(&self) -> String {
        self.name.to_string()
    }
}

pub trait KillportOperations {
    /// Finds the killables (native processes and docker containers) associated with the specified `port`.
    fn find_target_killables(&self, port: u16, mode: Mode)
        -> Result<Vec<Box<dyn Killable>>, Error>;

    /// Manages the action of killing or simulating the killing of services by port.
    fn kill_service_by_port(
        &self,
        port: u16,
        signal: Signal,
        mode: Mode,
        dry_run: bool,
    ) -> Result<Vec<(String, String)>, Error>;
}

pub struct Killport;

impl KillportOperations for Killport {
    /// Finds the killables (native processes and docker containers) associated with the specified `port`.
    ///
    /// Returns a `Vec` of killables.
    ///
    /// # Arguments
    ///
    /// * `port` - A u16 value representing the port number.
    fn find_target_killables(
        &self,
        port: u16,
        mode: Mode,
    ) -> Result<Vec<Box<dyn Killable>>, Error> {
        let mut target_killables: Vec<Box<dyn Killable>> = vec![];
        let docker_present = mode != Mode::Process && DockerContainer::is_docker_present()?;

        if mode != Mode::Container {
            let target_processes = find_target_processes(port)?;

            for process in target_processes {
                // Check if the process name contains 'docker' and skip if in docker mode
                if docker_present && process.name.to_lowercase().contains("docker") {
                    continue;
                }
                target_killables.push(Box::new(process));
            }
        }

        // Add containers if Docker is present and mode is not set to only process
        if docker_present && mode != Mode::Process {
            let target_containers = DockerContainer::find_target_containers(port)?; // Assume this function returns Result<Vec<DockerContainer>, Error>

            for container in target_containers {
                target_killables.push(Box::new(container));
            }
        }

        Ok(target_killables)
    }

    /// Manages the action of killing or simulating the killing of services by port.
    /// This function can either actually kill processes or containers, or simulate the action based on the `dry_run` flag.
    ///
    /// # Arguments
    /// * `port` - The port number to check for killable entities.
    /// * `signal` - The signal to send if not simulating.
    /// * `mode` - The mode of operation, determining if processes, containers, or both should be targeted.
    /// * `dry_run` - If true, simulates the actions without actually killing any entities.
    ///
    /// # Returns
    /// * `Result<Vec<(String, String)>, Error>` - A list of killable entities or an error.
    fn kill_service_by_port(
        &self,
        port: u16,
        signal: Signal,
        mode: Mode,
        dry_run: bool,
    ) -> Result<Vec<(String, String)>, Error> {
        let mut results = Vec::new();
        let target_killables = self.find_target_killables(port, mode)?; // Use the existing function to find targets

        for killable in target_killables {
            if dry_run {
                // In dry-run mode, collect information about the entity without killing
                results.push((killable.get_type(), killable.get_name()));
            } else {
                // In actual mode, attempt to kill the entity and collect its information if successful
                if killable.kill(signal)? {
                    results.push((killable.get_type(), killable.get_name()));
                }
            }
        }

        Ok(results)
    }
}
