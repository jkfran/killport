use crate::unix::UnixProcess;

use libproc::libproc::bsd_info::BSDInfo;
use libproc::libproc::file_info::pidfdinfo;
use libproc::libproc::file_info::{ListFDs, ProcFDType};
use libproc::libproc::net_info::{SocketFDInfo, SocketInfoKind};
use libproc::libproc::proc_pid::{listpidinfo, name, pidinfo};
use libproc::processes::{pids_by_type, ProcFilter};
use log::debug;
use nix::unistd::Pid;
use std::collections::HashSet;
use std::io;

/// Finds the processes associated with the specified `port`.
///
/// Returns a `Vec` of native processes.
///
/// # Arguments
///
/// * `port` - Target port number
pub fn find_target_processes(port: u16) -> Result<Vec<UnixProcess>, io::Error> {
    let mut target_pids: Vec<UnixProcess> = vec![];
    let mut seen_pids: HashSet<i32> = HashSet::new();

    if let Ok(procs) = pids_by_type(ProcFilter::All) {
        'next_process: for p in procs {
            let pid = p as i32;
            if seen_pids.contains(&pid) {
                continue;
            }
            // Size the fd list to the process's actual fd count — a fixed cap
            // silently misses sockets in fd-heavy processes. Small headroom
            // covers fds opened between the two calls; fall back to a generous
            // default when the count is unavailable.
            let max_fds = pidinfo::<BSDInfo>(pid, 0)
                .map(|info| info.pbi_nfiles as usize + 32)
                .unwrap_or(4096);
            let fds = listpidinfo::<ListFDs>(pid, max_fds);
            if let Ok(fds) = fds {
                for fd in fds {
                    if let ProcFDType::Socket = fd.proc_fdtype.into() {
                        if let Ok(socket) = pidfdinfo::<SocketFDInfo>(pid, fd.proc_fd) {
                            let socket_kind = SocketInfoKind::from(socket.psi.soi_kind);
                            match socket_kind {
                                SocketInfoKind::In | SocketInfoKind::Tcp => {
                                    let local_port = unsafe {
                                        match socket_kind {
                                            SocketInfoKind::In => {
                                                socket.psi.soi_proto.pri_in.insi_lport as u16
                                            }
                                            SocketInfoKind::Tcp => {
                                                socket.psi.soi_proto.pri_tcp.tcpsi_ini.insi_lport
                                                    as u16
                                            }
                                            _ => continue,
                                        }
                                    };
                                    if u16::from_be(local_port) == port {
                                        // The process can exit between socket
                                        // enumeration and the name lookup; a
                                        // vanished process must not fail the scan.
                                        let process_name = match name(pid) {
                                            Ok(process_name) => process_name,
                                            Err(e) => {
                                                debug!(
                                                    "Skipping PID {}: name lookup failed ({})",
                                                    pid, e
                                                );
                                                continue 'next_process;
                                            }
                                        };
                                        debug!(
                                            "Found process '{}' with PID {} listening on port {}",
                                            process_name, pid, port
                                        );
                                        seen_pids.insert(pid);
                                        target_pids.push(UnixProcess::new(
                                            Pid::from_raw(pid),
                                            process_name,
                                        ));
                                        continue 'next_process;
                                    }
                                }
                                _ => (),
                            }
                        }
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
        // Bind a TCP listener in the current process
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let result = find_target_processes(port);
        assert!(result.is_ok());

        let processes = result.unwrap();
        assert!(
            !processes.is_empty(),
            "Expected to find at least one process on port {}",
            port
        );

        // The found process should have a name
        for process in &processes {
            assert!(!process.get_name().is_empty());
        }

        drop(listener);
    }

    #[test]
    fn test_find_target_processes_correct_type() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let processes = find_target_processes(port).unwrap();
        for process in &processes {
            assert_eq!(process.get_type(), crate::killable::KillableType::Process);
        }

        drop(listener);
    }

    #[test]
    fn test_find_target_processes_ipv4() {
        let listener = TcpListener::bind("0.0.0.0:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let processes = find_target_processes(port).unwrap();
        assert!(
            !processes.is_empty(),
            "Expected to find process on 0.0.0.0:{}",
            port
        );

        drop(listener);
    }

    #[test]
    fn test_find_target_processes_after_close() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        // After closing the listener, the process should no longer be found on that port
        // (though there might be a TIME_WAIT state)
        let processes = find_target_processes(port).unwrap();
        // We just verify it doesn't crash; the result depends on OS timing
        let _ = processes;
    }

    #[test]
    fn test_find_target_processes_multiple_on_different_ports() {
        let listener1 = TcpListener::bind("127.0.0.1:0").unwrap();
        let port1 = listener1.local_addr().unwrap().port();

        let listener2 = TcpListener::bind("127.0.0.1:0").unwrap();
        let port2 = listener2.local_addr().unwrap().port();

        let processes1 = find_target_processes(port1).unwrap();
        let processes2 = find_target_processes(port2).unwrap();

        assert!(!processes1.is_empty());
        assert!(!processes2.is_empty());

        drop(listener1);
        drop(listener2);
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
    fn test_find_target_processes_ipv6_tcp_listener() {
        let listener = TcpListener::bind("[::1]:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let processes = find_target_processes(port).unwrap();
        assert!(
            !processes.is_empty(),
            "Expected to find process with IPv6 TCP on port {}",
            port
        );

        drop(listener);
    }

    #[test]
    fn test_find_target_processes_dedup() {
        // Bind both IPv4 and IPv6 on the same port — should return only one process entry
        let listener4 = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener4.local_addr().unwrap().port();
        // Bind IPv6 on the same port
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

    #[test]
    fn test_find_target_processes_udp6_listener() {
        let socket = UdpSocket::bind("[::1]:0").unwrap();
        let port = socket.local_addr().unwrap().port();

        let processes = find_target_processes(port).unwrap();
        assert!(
            !processes.is_empty(),
            "Expected to find process with IPv6 UDP on port {}",
            port
        );

        drop(socket);
    }

    #[test]
    fn test_find_target_processes_beyond_1024_fds() {
        // Regression test: the fd list used to be capped at 1024 entries, so a
        // socket whose fd index landed past that was silently missed.
        const TARGET_FDS: libc::rlim_t = 2200;

        unsafe {
            let mut lim = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            if libc::getrlimit(libc::RLIMIT_NOFILE, &mut lim) != 0 {
                eprintln!("skipping: getrlimit failed");
                return;
            }
            if lim.rlim_cur < TARGET_FDS {
                let raised = libc::rlimit {
                    rlim_cur: TARGET_FDS.min(lim.rlim_max),
                    rlim_max: lim.rlim_max,
                };
                if libc::setrlimit(libc::RLIMIT_NOFILE, &raised) != 0
                    || raised.rlim_cur < TARGET_FDS
                {
                    eprintln!("skipping: cannot raise RLIMIT_NOFILE to {}", TARGET_FDS);
                    return;
                }
            }
        }

        // Hold enough fds open that the listener bound afterwards gets an fd
        // index well past the old 1024 cap.
        let mut hoard = Vec::new();
        for _ in 0..1500 {
            match std::fs::File::open("/dev/null") {
                Ok(f) => hoard.push(f),
                Err(_) => break,
            }
        }
        if hoard.len() < 1200 {
            eprintln!("skipping: could only open {} fds", hoard.len());
            return;
        }

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let processes = find_target_processes(port).unwrap();
        assert!(
            !processes.is_empty(),
            "Expected to find listener on port {} even with fd index > 1024",
            port
        );

        drop(listener);
        drop(hoard);
    }
}
