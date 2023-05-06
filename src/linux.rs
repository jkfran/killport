use crate::KillPortSignalOptions;

use anyhow::{anyhow, Result};
use bollard::container::{KillContainerOptions, ListContainersOptions};
use bollard::Docker;
use log::{debug, info};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use procfs::process::FDTarget;
use std::collections::HashMap;
use std::io;
use tokio::runtime::Runtime;

/// Interface for killable targets such as native process and docker container.
trait Killable {
    fn kill(&self, signal: KillPortSignalOptions) -> Result<bool>;
}

#[derive(Debug)]
struct NativeProcess {
    /// System native process ID.
    pid: Pid,
}

impl NativeProcess {
    /// Kills the process with the specified `pid`.
    ///
    /// # Arguments
    ///
    /// * `pid` - An i32 value representing the process ID.
    /// * `signal` - A enum value representing the signal type.
    fn kill_process(pid: Pid, signal: KillPortSignalOptions) -> Result<()> {
        info!("Killing process with PID {}", pid);

        let system_signal = match signal {
            KillPortSignalOptions::SIGKILL => Signal::SIGKILL,
            KillPortSignalOptions::SIGTERM => Signal::SIGTERM,
        };
        kill(pid, system_signal)
            .map_err(|e| anyhow!(std::io::Error::new(std::io::ErrorKind::Other, e)))
    }

    /// Recursively kills the process with the specified `pid` and its children.
    ///
    /// # Arguments
    ///
    /// * `pid` - An i32 value representing the process ID.
    /// * `signal` - A enum value representing the signal type.
    fn kill_process_and_children(pid: Pid, signal: KillPortSignalOptions) -> Result<()> {
        let mut children_pids = Vec::new();
        Self::collect_child_pids(pid, &mut children_pids)?;

        for child_pid in children_pids {
            Self::kill_process(child_pid, signal)?;
        }

        Self::kill_process(pid, signal)?;

        Ok(())
    }

    /// Collects the child process IDs for the specified `pid` and stores them in
    /// `child_pids`.
    ///
    /// # Arguments
    ///
    /// * `pid` - An i32 value representing the process ID.
    /// * `child_pids` - A mutable reference to a `Vec<i32>` where the child PIDs will be stored.
    fn collect_child_pids(pid: Pid, child_pids: &mut Vec<Pid>) -> Result<()> {
        let processes = procfs::process::all_processes().unwrap();

        for p in processes {
            let process = p.unwrap();

            if process.stat().unwrap().ppid == pid.as_raw() {
                let pid = Pid::from_raw(process.pid);
                child_pids.push(pid);
                Self::collect_child_pids(pid, child_pids)?;
            }
        }

        Ok(())
    }
}

impl Killable for NativeProcess {
    /// Kill the linux native process.
    ///
    /// # Arguments
    ///
    /// * `signal` - A enum value representing the signal type.
    fn kill(&self, signal: KillPortSignalOptions) -> Result<bool> {
        let mut killed_any = false;

        match Self::kill_process_and_children(self.pid, signal) {
            Ok(_) => killed_any = true,
            Err(err) => return Err(err),
        };

        Ok(killed_any)
    }
}

#[derive(Debug)]
struct DockerContainer {
    /// Container name.
    name: String,
}

impl DockerContainer {
    fn kill_container(name: &String, signal: KillPortSignalOptions) -> Result<()> {
        info!("Killing container with name {}", name);

        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let docker = Docker::connect_with_socket_defaults().unwrap();
            let options = KillContainerOptions {
                signal: match signal {
                    KillPortSignalOptions::SIGKILL => "SIGKILL",
                    KillPortSignalOptions::SIGTERM => "SIGTERM",
                },
            };

            docker
                .kill_container(name.replace("/", "").as_str(), Some(options))
                .await
        })
        .map_err(|e| anyhow!(e))
    }
}

impl Killable for DockerContainer {
    fn kill(&self, signal: KillPortSignalOptions) -> Result<bool> {
        let mut killed_any = false;

        match Self::kill_container(&self.name, signal) {
            Ok(_) => killed_any = true,
            Err(err) => return Err(err),
        };

        Ok(killed_any)
    }
}

/// Attempts to kill processes listening on the specified `port`.
///
/// Returns a `Result` with `true` if any processes were killed, `false` if no
/// processes were found listening on the port, and an `Error` if the operation
/// failed or the platform is unsupported.
///
/// # Arguments
///
/// * `port` - A u16 value representing the port number.
/// * `signal` - A enum value representing the signal type.
pub fn kill_processes_by_port(port: u16, signal: KillPortSignalOptions) -> Result<bool> {
    let mut killed_any = false;
    let target_killables = find_target_killables(port);

    for killable in target_killables {
        killed_any |= killable.kill(signal).unwrap();
    }

    if !killed_any {
        return Err(anyhow!(
            std::io::Error::new(
                io::ErrorKind::Other,
                "Unable to kill the process. The process might be running as another user or root. Try again with sudo",
            )
        ));
    }

    Ok(killed_any)
}

/// Finds the killables (native processes and docker containers) associated with the specified `port`.
///
/// Returns a `Vec` of killables.
///
/// # Arguments
///
/// * `port` - A u16 value representing the port number.
#[cfg(target_os = "linux")]
fn find_target_killables(port: u16) -> Vec<Box<dyn Killable>> {
    let mut target_killables: Vec<Box<dyn Killable>> = vec![];

    let target_inodes = find_target_inodes(port);
    let target_processes = find_target_processes(target_inodes);
    for process in target_processes {
        target_killables.push(Box::new(process));
    }

    let target_containers = find_target_containers(port);
    for container in target_containers {
        target_killables.push(Box::new(container));
    }

    target_killables
}

/// Finds the inodes associated with the specified `port`.
///
/// Returns a `Vec` of inodes for both IPv4 and IPv6 connections.
///
/// # Arguments
///
/// * `port` - A u16 value representing the port number.
#[cfg(target_os = "linux")]
fn find_target_inodes(port: u16) -> Vec<u64> {
    let tcp = procfs::net::tcp();
    let tcp6 = procfs::net::tcp6();
    let udp = procfs::net::udp();
    let udp6 = procfs::net::udp6();
    let mut target_inodes = Vec::new();

    trait NetEntry {
        fn local_address(&self) -> std::net::SocketAddr;

        fn inode(&self) -> u64;
    }

    impl NetEntry for procfs::net::TcpNetEntry {
        fn local_address(&self) -> std::net::SocketAddr {
            self.local_address
        }

        fn inode(&self) -> u64 {
            self.inode
        }
    }

    impl NetEntry for procfs::net::UdpNetEntry {
        fn local_address(&self) -> std::net::SocketAddr {
            self.local_address
        }

        fn inode(&self) -> u64 {
            self.inode
        }
    }

    fn add_matching_inodes<T: NetEntry>(
        target_inodes: &mut Vec<u64>,
        net_entries: procfs::ProcResult<Vec<T>>,
        port: u16,
    ) {
        if let Ok(net_entries) = net_entries {
            target_inodes.extend(
                net_entries
                    .into_iter()
                    .filter(move |net_entry| net_entry.local_address().port() == port)
                    .map(|net_entry| net_entry.inode()),
            );
        }
    }

    add_matching_inodes(&mut target_inodes, tcp, port);
    add_matching_inodes(&mut target_inodes, tcp6, port);
    add_matching_inodes(&mut target_inodes, udp, port);
    add_matching_inodes(&mut target_inodes, udp6, port);

    target_inodes
}

/// Finds the processes associated with the specified `port`.
///
/// Returns a `Vec` of native processes.
///
/// # Arguments
///
/// * `inodes` - Target inodes
fn find_target_processes(inodes: Vec<u64>) -> Vec<NativeProcess> {
    let mut target_pids: Vec<NativeProcess> = vec![];

    for inode in inodes {
        let processes = procfs::process::all_processes().unwrap();
        for p in processes {
            let process = p.unwrap();

            if let Ok(fds) = process.fd() {
                for fd in fds {
                    if let FDTarget::Socket(sock_inode) = fd.unwrap().target {
                        if inode == sock_inode {
                            debug!("Found process with PID {}", process.pid);
                            target_pids.push(NativeProcess {
                                pid: Pid::from_raw(process.pid),
                            });
                        }
                    }
                }
            }
        }
    }

    target_pids
}

/// Finds the inodes associated with the specified `port`.
///
/// Returns a `Vec` of inodes for both IPv4 and IPv6 connections.
///
/// # Arguments
///
/// * `port` - A u16 value representing the port number.
fn find_target_containers(port: u16) -> Vec<DockerContainer> {
    let mut target_containers: Vec<DockerContainer> = vec![];

    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let docker = Docker::connect_with_socket_defaults().unwrap();

        let mut filters = HashMap::new();
        filters.insert("publish".to_string(), vec![port.to_string()]);
        let options = ListContainersOptions {
            all: true,
            filters,
            ..Default::default()
        };

        let containers = docker
            .list_containers::<String>(Some(options))
            .await
            .unwrap();
        for container in containers {
            let ports = container.ports.clone().unwrap_or_else(|| vec![]);

            for p in ports {
                if p.public_port.is_none() {
                    continue;
                };

                let container_name = container.names.clone().unwrap().pop().unwrap();
                target_containers.push(DockerContainer {
                    name: container_name.to_string(),
                });
            }
        }
    });

    target_containers
}
