use crate::signal::KillportSignal;
use bollard::query_parameters::{KillContainerOptions, ListContainersOptions};
use bollard::Docker;
use log::debug;
use std::collections::HashMap;
use std::io::Error;
use tokio::runtime::Runtime;

pub struct Container {
    pub name: String,
}

impl Container {
    /// Kill the container by name.
    ///
    /// # Arguments
    ///
    /// * `rt` - A reference to a Tokio runtime.
    /// * `name` - A container name.
    /// * `signal` - A enum value representing the signal type.
    pub fn kill(rt: &Runtime, name: &str, signal: KillportSignal) -> Result<(), Error> {
        rt.block_on(async {
            let docker =
                Docker::connect_with_socket_defaults().map_err(|e| Error::other(e.to_string()))?;

            let options = KillContainerOptions {
                signal: signal.to_string(),
            };

            docker
                .kill_container(name, Some(options))
                .await
                .map_err(|e| Error::other(e.to_string()))
        })
    }

    /// Finds the containers associated with the specified `port`.
    pub fn find_target_containers(rt: &Runtime, port: u16) -> Result<Vec<Self>, Error> {
        rt.block_on(async {
            let docker = match Docker::connect_with_socket_defaults() {
                Ok(d) => d,
                Err(e) => {
                    debug!("Container runtime socket not available: {}", e);
                    return Ok(vec![]);
                }
            };

            let mut filters = HashMap::new();
            filters.insert("publish".to_string(), vec![port.to_string()]);
            filters.insert("status".to_string(), vec!["running".to_string()]);

            let options = ListContainersOptions {
                filters: Some(filters),
                ..Default::default()
            };

            let containers = match docker.list_containers(Some(options)).await {
                Ok(c) => c,
                Err(e) => {
                    debug!("Failed to list containers: {}", e);
                    return Ok(vec![]);
                }
            };

            Ok(containers
                .iter()
                .filter_map(|container| {
                    container.names.as_ref()?.first().map(|name| Container {
                        name: name.strip_prefix('/').unwrap_or(name).to_string(),
                    })
                })
                .collect())
        })
    }

    pub fn is_available(rt: &Runtime) -> Result<bool, Error> {
        rt.block_on(async {
            let docker = match Docker::connect_with_socket_defaults() {
                Ok(d) => d,
                Err(e) => {
                    debug!("Container runtime socket not available: {}", e);
                    return Ok(false);
                }
            };

            match docker.version().await {
                Ok(version) => {
                    debug!("Connected to container runtime: {:?}", version);
                    Ok(true)
                }
                Err(e) => {
                    debug!("Failed to connect to container runtime: {}", e);
                    Ok(false)
                }
            }
        })
    }
}
