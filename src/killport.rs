use crate::docker::DockerContainer;
#[cfg(target_os = "linux")]
use crate::linux::find_target_processes;
#[cfg(target_os = "macos")]
use crate::macos::find_target_processes;
#[cfg(target_os = "windows")]
use crate::windows::find_target_processes;
use crate::{cli::Mode, signal::KillportSignal};
use std::{fmt::Display, io::Error};

/// Interface for killable targets such as native process and docker container.
pub trait Killable {
    fn kill(&self, signal: KillportSignal) -> Result<bool, Error>;
    fn get_type(&self) -> KillableType;
    fn get_name(&self) -> String;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KillableType {
    Process,
    Container,
}

impl Display for KillableType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            KillableType::Process => "process",
            KillableType::Container => "container",
        })
    }
}

impl Killable for DockerContainer {
    /// Entry point to kill the docker containers.
    ///
    /// # Arguments
    ///
    /// * `signal` - A enum value representing the signal type.
    fn kill(&self, signal: KillportSignal) -> Result<bool, Error> {
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
    /// * `String` - A string that describes the type of the killable target. For a `UnixProcess` it will return "process",
    /// and for a `DockerContainer` it will return "container".
    fn get_type(&self) -> KillableType {
        KillableType::Container
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
        signal: KillportSignal,
        mode: Mode,
        dry_run: bool,
    ) -> Result<Vec<(KillableType, String)>, Error>;
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
                if docker_present && process.get_name().to_lowercase().contains("docker") {
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
        signal: KillportSignal,
        mode: Mode,
        dry_run: bool,
    ) -> Result<Vec<(KillableType, String)>, Error> {
        let mut results = Vec::new();
        let target_killables = self.find_target_killables(port, mode)?; // Use the existing function to find targets

        for killable in target_killables {
            if dry_run {
                // In dry-run mode, collect information about the entity without killing
                results.push((killable.get_type(), killable.get_name()));
            } else {
                // In actual mode, attempt to kill the entity and collect its information if successful
                if killable.kill(signal.clone())? {
                    results.push((killable.get_type(), killable.get_name()));
                }
            }
        }

        Ok(results)
    }
}
