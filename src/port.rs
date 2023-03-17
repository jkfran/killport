use crate::process::kill_processes_by_inode;
use std::io::Error;

pub fn kill_port(port: u16) -> Result<bool, Error> {
    if !cfg!(target_family = "unix") {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Unsupported platform",
        ));
    }

    let mut killed_any = false;

    let target_inodes = find_target_inodes(port);

    if !target_inodes.is_empty() {
        for target_inode in target_inodes {
            killed_any |= kill_processes_by_inode(target_inode)?;
        }
    }

    Ok(killed_any)
}

fn find_target_inodes(port: u16) -> Vec<u64> {
    let tcp = procfs::net::tcp().unwrap();
    let tcp6 = procfs::net::tcp6().unwrap();
    let mut target_inodes = Vec::new();

    target_inodes.extend(
        tcp.into_iter()
            .filter(|tcp_entry| tcp_entry.local_address.port() == port)
            .map(|tcp_entry| tcp_entry.inode),
    );
    target_inodes.extend(
        tcp6.into_iter()
            .filter(|tcp_entry| tcp_entry.local_address.port() == port)
            .map(|tcp_entry| tcp_entry.inode),
    );

    target_inodes
}
