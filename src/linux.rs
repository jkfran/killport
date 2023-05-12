use log::{debug, info, warn};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use procfs::process::FDTarget;
use std::io;
use std::io::Error;
use std::path::Path;

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
pub fn kill_processes_by_port(port: u16, signal: Signal) -> Result<bool, Error> {
    let mut killed_any = false;

    let target_inodes = find_target_inodes(port);

    if !target_inodes.is_empty() {
        for target_inode in target_inodes {
            killed_any |= kill_processes_by_inode(target_inode, signal)?;
        }
    }

    Ok(killed_any)
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

/// Attempts to kill processes associated with the specified `target_inode`.
///
/// Returns a `Result` with `true` if any processes were killed, and an `Error`
/// if the operation failed or if no processes were found associated with the inode.
///
/// # Arguments
///
/// * `target_inode` - A u64 value representing the target inode.
/// * `signal` - A enum value representing the signal type.
fn kill_processes_by_inode(
    target_inode: u64,
    signal: Signal,
) -> Result<bool, Error> {
    let processes = procfs::process::all_processes().unwrap();
    let mut killed_any = false;

    for p in processes {
        let process = p.unwrap();
        if let Ok(fds) = process.fd() {
            for fd in fds {
                if let FDTarget::Socket(inode) = fd.unwrap().target {
                    if target_inode == inode {
                        debug!("Found process with PID {}", process.pid);

                        if let Ok(cmdline) = process.cmdline() {
                            if let Some(cmd) = Path::new(&cmdline[0])
                                .file_name()
                                .and_then(|fname| fname.to_str())
                            {
                                if cmd.starts_with("docker") {
                                    warn!("Found Docker. You might need to stop the container manually");
                                }
                            }
                        }

                        match kill_process_and_children(process.pid, signal) {
                            Ok(_) => {
                                killed_any = true;
                            }
                            Err(err) => {
                                return Err(err);
                            }
                        }
                        break;
                    }
                }
            }
        }
    }

    if !killed_any {
        return Err(Error::new(
            io::ErrorKind::Other,
            "Unable to kill the process. The process might be running as another user or root. Try again with sudo",
        ));
    }

    Ok(killed_any)
}

/// Recursively kills the process with the specified `pid` and its children.
///
/// # Arguments
///
/// * `pid` - An i32 value representing the process ID.
/// * `signal` - A enum value representing the signal type.
fn kill_process_and_children(
    pid: i32,
    signal: Signal,
) -> Result<(), std::io::Error> {
    let mut children_pids = Vec::new();
    collect_child_pids(pid, &mut children_pids)?;

    for child_pid in children_pids {
        kill_process(child_pid, signal)?;
    }

    kill_process(pid, signal)?;

    Ok(())
}

/// Collects the child process IDs for the specified `pid` and stores them in
/// `child_pids`.
///
/// # Arguments
///
/// * `pid` - An i32 value representing the process ID.
/// * `child_pids` - A mutable reference to a `Vec<i32>` where the child PIDs will be stored.
fn collect_child_pids(pid: i32, child_pids: &mut Vec<i32>) -> Result<(), std::io::Error> {
    let processes = procfs::process::all_processes().unwrap();

    for p in processes {
        let process = p.unwrap();

        if process.stat().unwrap().ppid == pid {
            child_pids.push(process.pid);
            collect_child_pids(process.pid, child_pids)?;
        }
    }

    Ok(())
}

/// Kills the process with the specified `pid`.
///
/// # Arguments
///
/// * `pid` - An i32 value representing the process ID.
/// * `signal` - A enum value representing the signal type.
fn kill_process(pid: i32, signal: Signal) -> Result<(), std::io::Error> {
    info!("Killing process with PID {}", pid);
    let pid = Pid::from_raw(pid);

    kill(pid, signal).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}
