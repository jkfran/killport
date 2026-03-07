use crate::unix::UnixProcess;

use libproc::libproc::file_info::pidfdinfo;
use libproc::libproc::file_info::{ListFDs, ProcFDType};
use libproc::libproc::net_info::{SocketFDInfo, SocketInfoKind};
use libproc::libproc::proc_pid::{listpidinfo, name};
use libproc::processes::{pids_by_type, ProcFilter};
use log::debug;
use nix::unistd::Pid;
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

    if let Ok(procs) = pids_by_type(ProcFilter::All) {
        for p in procs {
            let pid = p as i32;
            let fds = listpidinfo::<ListFDs>(pid, 1024); // Large enough to cover typical number of open files
            if let Ok(fds) = fds {
                for fd in fds {
                    if let ProcFDType::Socket = fd.proc_fdtype.into() {
                        if let Ok(socket) = pidfdinfo::<SocketFDInfo>(pid, fd.proc_fd) {
                            // Correctly cast soi_kind to SocketInfoKind
                            let socket_kind = SocketInfoKind::from(socket.psi.soi_kind);
                            {
                                match socket_kind {
                                    SocketInfoKind::In | SocketInfoKind::Tcp => {
                                        let local_port = unsafe {
                                            match socket_kind {
                                                SocketInfoKind::In => {
                                                    socket.psi.soi_proto.pri_in.insi_lport as u16
                                                }
                                                SocketInfoKind::Tcp => {
                                                    socket
                                                        .psi
                                                        .soi_proto
                                                        .pri_tcp
                                                        .tcpsi_ini
                                                        .insi_lport
                                                        as u16
                                                }
                                                _ => continue,
                                            }
                                        };
                                        if u16::from_be(local_port) == port {
                                            let process_name =
                                                name(pid).map_err(io::Error::other)?;
                                            debug!(
                                                "Found process '{}' with PID {} listening on port {}",
                                                process_name, pid, port
                                            );
                                            target_pids.push(UnixProcess::new(
                                                Pid::from_raw(pid),
                                                process_name,
                                            ));
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
    }

    Ok(target_pids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::killport::Killable;
    use std::net::TcpListener;

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
            assert_eq!(process.get_type(), crate::killport::KillableType::Process);
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
}
