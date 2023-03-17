//! This module provides functions for working with processes and inodes.
//!
//! It exposes a single public function `kill_processes_by_inode` that attempts to
//! kill processes associated with a specified inode.

use log::{debug, info, warn};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use procfs::process::FDTarget;
use std::io;
use std::io::Error;
use std::path::Path;

/// Attempts to kill processes associated with the specified `target_inode`.
///
/// Returns a `Result` with `true` if any processes were killed, and an `Error`
/// if the operation failed or if no processes were found associated with the inode.
///
/// # Arguments
///
/// * `target_inode` - A u64 value representing the target inode.
pub fn kill_processes_by_inode(target_inode: u64) -> Result<bool, Error> {
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

                        match kill_process_and_children(process.pid) {
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
fn kill_process_and_children(pid: i32) -> Result<(), std::io::Error> {
    let mut children_pids = Vec::new();
    collect_child_pids(pid, &mut children_pids)?;

    for child_pid in children_pids {
        kill_process(child_pid)?;
    }

    kill_process(pid)?;

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
fn kill_process(pid: i32) -> Result<(), std::io::Error> {
    info!("Killing process with PID {}", pid);
    let pid = Pid::from_raw(pid);
    kill(pid, Signal::SIGKILL).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}
