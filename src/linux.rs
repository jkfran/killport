use crate::unix::UnixProcess;

use log::debug;
use nix::unistd::Pid;
use procfs::process::FDTarget;
use std::collections::HashSet;
use std::io::Error;

/// Finds the inodes associated with the specified `port`.
///
/// Returns the set of socket inodes for both IPv4 and IPv6, TCP and UDP.
///
/// # Arguments
///
/// * `port` - A u16 value representing the port number.
fn find_target_inodes(port: u16) -> HashSet<u64> {
    let tcp = procfs::net::tcp();
    let tcp6 = procfs::net::tcp6();
    let udp = procfs::net::udp();
    let udp6 = procfs::net::udp6();
    let mut target_inodes = HashSet::new();

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
        target_inodes: &mut HashSet<u64>,
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

/// Returns a display name for the process: the full command line, falling
/// back to the short `comm` name when the command line is empty or
/// unreadable (kernel threads, exec races). `None` means the process is
/// effectively gone and should be skipped.
fn process_name(process: &procfs::process::Process) -> Option<String> {
    match process.cmdline() {
        Ok(parts) if !parts.is_empty() => Some(parts.join(" ")),
        _ => process.stat().ok().map(|stat| stat.comm),
    }
}

/// Finds the processes associated with the specified `port`.
///
/// Returns a `Vec` of native processes.
///
/// # Arguments
///
/// * `port` - Target port number
pub fn find_target_processes(port: u16) -> Result<Vec<UnixProcess>, Error> {
    let mut target_pids: Vec<UnixProcess> = vec![];
    let target_inodes = find_target_inodes(port);

    if target_inodes.is_empty() {
        return Ok(target_pids);
    }

    // Single pass over /proc: match each process's socket fds against the
    // full inode set at once (a port bound on tcp/tcp6/udp/udp6 yields up
    // to four inodes; scanning /proc once per inode is wasteful).
    let processes = procfs::process::all_processes().map_err(Error::other)?;
    'next_process: for p in processes {
        // Processes can vanish between enumeration and inspection (race condition).
        // Skip any process that disappears mid-scan.
        let process = match p {
            Ok(p) => p,
            Err(_) => continue,
        };

        if let Ok(fds) = process.fd() {
            for fd in fds {
                let fd = match fd {
                    Ok(fd) => fd,
                    Err(_) => continue,
                };

                if let FDTarget::Socket(sock_inode) = fd.target {
                    if target_inodes.contains(&sock_inode) {
                        let name = match process_name(&process) {
                            Some(name) => name,
                            None => continue 'next_process,
                        };
                        debug!("Found process '{}' with PID {}", name, process.pid());
                        target_pids.push(UnixProcess::new(Pid::from_raw(process.pid), name));
                        // One entry per process, even when several fds match
                        // (dup'd sockets, multiple protocols on one port).
                        continue 'next_process;
                    }
                }
            }
        }
    }

    Ok(target_pids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::killable::Killable;
    use std::net::{TcpListener, UdpSocket};

    #[test]
    fn test_find_target_processes_no_listeners() {
        // Use a port that is very unlikely to be in use
        let result = find_target_processes(19876);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_find_target_processes_tcp_listener() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let processes = find_target_processes(port).unwrap();
        assert!(
            !processes.is_empty(),
            "Expected to find at least one process on port {}",
            port
        );
        for process in &processes {
            assert!(!process.get_name().is_empty());
        }

        drop(listener);
    }

    #[test]
    fn test_find_target_processes_udp_listener() {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = socket.local_addr().unwrap().port();

        let processes = find_target_processes(port).unwrap();
        assert!(
            !processes.is_empty(),
            "Expected to find process with UDP socket on port {}",
            port
        );

        drop(socket);
    }

    #[test]
    fn test_find_target_processes_dedup() {
        // Bind IPv4 and IPv6 on the same port — two distinct socket inodes,
        // one owning process. The single-pass scan must return one entry.
        let listener4 = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener4.local_addr().unwrap().port();
        let listener6 = TcpListener::bind(format!("[::1]:{}", port)).unwrap();

        let processes = find_target_processes(port).unwrap();
        assert_eq!(
            processes.len(),
            1,
            "Expected exactly 1 deduplicated process entry, got {}",
            processes.len()
        );

        drop(listener4);
        drop(listener6);
    }
}
