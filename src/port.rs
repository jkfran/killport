//! This module provides functions for working with network ports.
//!
//! It exposes a single public function `kill_port` that attempts to kill
//! processes listening on a specified port.

use crate::process::kill_processes_by_inode;
use std::io::Error;

/// Attempts to kill processes listening on the specified `port`.
///
/// Returns a `Result` with `true` if any processes were killed, `false` if no
/// processes were found listening on the port, and an `Error` if the operation
/// failed or the platform is unsupported.
///
/// # Arguments
///
/// * `port` - A u16 value representing the port number.
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

/// Finds the inodes associated with the specified `port`.
///
/// Returns a `Vec` of inodes for both IPv4 and IPv6 connections.
///
/// # Arguments
///
/// * `port` - A u16 value representing the port number.
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
