use bollard::container::{KillContainerOptions, ListContainersOptions};
use bollard::Docker;
use log::debug;
use nix::sys::signal::Signal;
use std::collections::HashMap;
use std::io::Error;
use tokio::runtime::Runtime;

pub struct DockerContainer {
    pub name: String,
}

impl DockerContainer {
    /// Kill the docker container.
    ///
    /// # Arguments
    ///
    /// * `name` - A container name.
    /// * `signal` - A enum value representing the signal type.
    pub fn kill_container(name: &str, signal: Signal) -> Result<(), Error> {
        let rt = Runtime::new()?;
        rt.block_on(async {
            let docker = Docker::connect_with_socket_defaults()
                .map_err(|e| Error::new(std::io::ErrorKind::Other, e.to_string()))?;

            let options = KillContainerOptions {
                signal: signal.to_string(),
            };

            docker
                .kill_container(name, Some(options))
                .await
                .map_err(|e| Error::new(std::io::ErrorKind::Other, e.to_string()))
        })
    }

    /// Finds the Docker containers associated with the specified `port`.
    pub fn find_target_containers(port: u16) -> Result<Vec<Self>, Error> {
        let rt = Runtime::new()?;
        rt.block_on(async {
            let docker = Docker::connect_with_socket_defaults()
                .map_err(|e| Error::new(std::io::ErrorKind::Other, e.to_string()))?;

            let mut filters = HashMap::new();
            filters.insert("publish".to_string(), vec![port.to_string()]);
            filters.insert("status".to_string(), vec!["running".to_string()]);

            let options = ListContainersOptions {
                filters,
                ..Default::default()
            };

            let containers = docker
                .list_containers::<String>(Some(options))
                .await
                .map_err(|e| Error::new(std::io::ErrorKind::Other, e.to_string()))?;

            Ok(containers
                .iter()
                .filter_map(|container| {
                    container
                        .names
                        .as_ref()?
                        .first()
                        .map(|name| DockerContainer {
                            name: if let Some(stripped) = name.strip_prefix('/') {
                                stripped.to_string()
                            } else {
                                name.clone()
                            },
                        })
                })
                .collect())
        })
    }

    pub fn is_docker_present() -> Result<bool, Error> {
        let rt = Runtime::new()?;
        rt.block_on(async {
            let docker = Docker::connect_with_socket_defaults()
                .map_err(|e| Error::new(std::io::ErrorKind::Other, e.to_string()))?;

            // Attempt to get the Docker version as a test of connectivity.
            match docker.version().await {
                Ok(version) => {
                    debug!("Connected to Docker version: {:?}", version);
                    Ok(true)
                }
                Err(e) => {
                    debug!("Failed to connect to Docker: {}", e);
                    Ok(false)
                }
            }
        })
    }
}
