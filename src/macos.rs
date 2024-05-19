use crate::killport::NativeProcess;

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
pub fn find_target_processes(port: u16) -> Result<Vec<NativeProcess>, io::Error> {
    let mut target_pids: Vec<NativeProcess> = vec![];

    if let Ok(procs) = pids_by_type(ProcFilter::All) {
        for p in procs {
            let pid = p as i32;
            let fds = listpidinfo::<ListFDs>(pid, 1024); // Large enough to cover typical number of open files
            if let Ok(fds) = fds {
                for fd in fds {
                    if let ProcFDType::Socket = fd.proc_fdtype.into() {
                        if let Ok(socket) = pidfdinfo::<SocketFDInfo>(pid, fd.proc_fd) {
                            // Correctly cast soi_kind to SocketInfoKind
                            if let Ok(socket_kind) = SocketInfoKind::try_from(socket.psi.soi_kind) {
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
                                            let process_name = name(pid).map_err(|e| {
                                                io::Error::new(io::ErrorKind::Other, e)
                                            })?;
                                            debug!(
                                                "Found process '{}' with PID {} listening on port {}",
                                                process_name, pid, port
                                            );
                                            target_pids.push(NativeProcess {
                                                pid: Pid::from_raw(pid),
                                                name: process_name,
                                            });
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
