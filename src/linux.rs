use procfs::process::FDTarget;

/// Finds the inodes associated with the specified `port`.
///
/// Returns a `Vec` of inodes for both IPv4 and IPv6 connections.
///
/// # Arguments
///
/// * `port` - A u16 value representing the port number.
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
fn find_target_processes(port: u16) -> Result<Vec<NativeProcess>, Error> {
    let mut target_pids: Vec<NativeProcess> = vec![];
    let inodes = find_target_inodes(port);

    for inode in inodes {
        let processes = procfs::process::all_processes()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        for p in processes {
            let process = p.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

            if let Ok(fds) = process.fd() {
                for fd in fds {
                    let fd = fd.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

                    if let FDTarget::Socket(sock_inode) = fd.target {
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

    Ok(target_pids)
}
