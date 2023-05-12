use crate::KillPortSignalOptions;
use log::{debug, info};
use std::{
    alloc::Layout,
    collections::HashSet,
    io::{Error, ErrorKind},
};
use windows_sys::Win32::{
    Foundation::{
        CloseHandle, GetLastError, ERROR_INSUFFICIENT_BUFFER, INVALID_HANDLE_VALUE, NO_ERROR,
    },
    NetworkManagement::IpHelper::{
        GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCP6ROW_OWNER_MODULE,
        MIB_TCP6TABLE_OWNER_MODULE, MIB_TCPROW_OWNER_MODULE, MIB_TCPTABLE_OWNER_MODULE,
        MIB_UDP6ROW_OWNER_MODULE, MIB_UDP6TABLE_OWNER_MODULE, MIB_UDPROW_OWNER_MODULE,
        MIB_UDPTABLE_OWNER_MODULE, TCP_TABLE_OWNER_MODULE_ALL, UDP_TABLE_OWNER_MODULE,
    },
    Networking::WinSock::{ADDRESS_FAMILY, AF_INET, AF_INET6},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32,
            TH32CS_SNAPPROCESS,
        },
        Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE},
    },
};

/// Attempts to kill processes listening on the specified `port`.
///
/// Returns a `Result` with `true` if any processes were killed, `false` if no
/// processes were found listening on the port, and an `Error` if the operation
/// failed or the platform is unsupported.
///
/// # Arguments
///
/// * `port` - A u16 value representing the port number.
pub fn kill_processes_by_port(port: u16, _: KillPortSignalOptions) -> Result<bool, Error> {
    let mut pids = HashSet::new();
    unsafe {
        // Collect the PIDs
        get_process_tcp_v4(port, &mut pids)?;
        get_process_tcp_v6(port, &mut pids)?;
        get_process_udp_v4(port, &mut pids)?;
        get_process_udp_v6(port, &mut pids)?;

        // Nothing was found
        if pids.is_empty() {
            return Ok(false);
        }

        // Collect parents of the PIDs
        collect_parents(&mut pids)?;

        for pid in pids {
            debug!("Found process with PID {}", pid);
            kill_process(pid)?;
        }

        // Something had to have been killed to reach here
        Ok(true)
    }
}

/// Collects all the parent processes for the PIDs in
/// the provided set
///
/// # Arguments
///
/// * `pids` - The set to match PIDs from and insert PIDs into
unsafe fn collect_parents(pids: &mut HashSet<u32>) -> Result<(), Error> {
    // Request a snapshot handle
    let handle = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);

    // Ensure we got a valid handle
    if handle == INVALID_HANDLE_VALUE {
        let error = GetLastError();
        return Err(std::io::Error::new(
            ErrorKind::Other,
            format!("Failed to get handle to processes: {:#x}", error),
        ));
    }

    // Allocate the memory to use for the entries
    let layout = Layout::new::<PROCESSENTRY32>();
    let buffer = std::alloc::alloc_zeroed(layout);

    let entry_ptr: *mut PROCESSENTRY32 = buffer.cast();

    // Set the size of the structure to the correct value
    let dw_size = std::ptr::addr_of_mut!((*entry_ptr).dwSize);
    *dw_size = layout.size() as u32;

    // Process the first item
    if Process32First(handle, entry_ptr) != 0 {
        let mut count = 0;

        loop {
            let entry: PROCESSENTRY32 = entry_ptr.read();

            // Add matching processes to the output
            if pids.contains(&entry.th32ProcessID) {
                pids.insert(entry.th32ParentProcessID);
                count += 1;
            }

            // Process the next entry
            if Process32Next(handle, entry_ptr) == 0 {
                break;
            }
        }

        info!("Collected {} parent processes", count);
    }

    // Deallocate the memory used
    std::alloc::dealloc(buffer, layout);

    // Close the handle we obtained
    CloseHandle(handle);

    Ok(())
}

/// Kills a process with the provided process ID
///
/// # Arguments
///
/// * `pid` - The process ID
unsafe fn kill_process(pid: u32) -> Result<(), Error> {
    info!("Killing process with PID {}", pid);

    // Open the process handle with intent to terminate
    let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
    if handle == 0 {
        let error = GetLastError();
        return Err(std::io::Error::new(
            ErrorKind::Other,
            format!("Failed to obtain handle to process {}: {:#x}", pid, error),
        ));
    }

    let result = TerminateProcess(handle, 0);
    if result == 0 {
        let error = GetLastError();
        return Err(std::io::Error::new(
            ErrorKind::Other,
            format!("Failed to terminate process {}: {:#x}", pid, error),
        ));
    }

    Ok(())
}

/// Reads the extended TCP table into memory using the provided address `family`
/// to determine the output type. Returns the memory pointer to the loaded struct
///
/// # Arguments
///
/// * `layout` - The layout of the memory
/// * `family` - The address family type
unsafe fn get_extended_tcp_table(layout: Layout, family: ADDRESS_FAMILY) -> Result<*mut u8, Error> {
    let mut buffer = std::alloc::alloc(layout);

    // Size estimate for resizing the buffer
    let mut size = 0;

    // Result of asking for the TCP table
    let mut result: u32;

    loop {
        // Ask windows for the extended TCP table mapping between TCP ports and PIDs
        result = GetExtendedTcpTable(
            buffer.cast(),
            &mut size,
            1,
            family as u32,
            TCP_TABLE_OWNER_MODULE_ALL,
            0,
        );

        // No error occurred
        if result == NO_ERROR {
            break;
        }

        // Handle buffer too small
        if result == ERROR_INSUFFICIENT_BUFFER {
            // Resize the buffer to the new size
            buffer = std::alloc::realloc(buffer, layout, size as usize);
            continue;
        }

        // Deallocate the buffer memory
        std::alloc::dealloc(buffer, layout);

        // Handle unknown failures
        return Err(std::io::Error::new(
            ErrorKind::Other,
            "Failed to get size estimate for TCP table",
        ));
    }

    Ok(buffer)
}

/// Reads the extended UDP table into memory using the provided address `family`
/// to determine the output type. Returns the memory pointer to the loaded struct
///
/// # Arguments
///
/// * `layout` - The layout of the memory
/// * `family` - The address family type
unsafe fn get_extended_udp_table(layout: Layout, family: ADDRESS_FAMILY) -> Result<*mut u8, Error> {
    let mut buffer = std::alloc::alloc(layout);

    // Size estimate for resizing the buffer
    let mut size = 0;

    // Result of asking for the TCP table
    let mut result: u32;

    loop {
        // Ask windows for the extended UDP table mapping between UDP ports and PIDs
        result = GetExtendedUdpTable(
            buffer.cast(),
            &mut size,
            1,
            family as u32,
            UDP_TABLE_OWNER_MODULE,
            0,
        );

        // No error occurred
        if result == NO_ERROR {
            break;
        }

        // Handle buffer too small
        if result == ERROR_INSUFFICIENT_BUFFER {
            // Resize the buffer to the new size
            buffer = std::alloc::realloc(buffer, layout, size as usize);
            continue;
        }

        // Deallocate the buffer memory
        std::alloc::dealloc(buffer, layout);

        // Handle unknown failures
        return Err(std::io::Error::new(
            ErrorKind::Other,
            "Failed to get size estimate for UDP table",
        ));
    }

    Ok(buffer)
}

/// Searches through the IPv4 extended TCP table for any processes
/// that are listening on the provided `port`. Will append any processes
/// found onto the provided `pids` set
///
/// # Arguments
///
/// * `port` The port to search for
/// * `pids` The set of process IDs to append to
unsafe fn get_process_tcp_v4(port: u16, pids: &mut HashSet<u32>) -> Result<(), Error> {
    // Create the memory layout for the table
    let layout = Layout::new::<MIB_TCPTABLE_OWNER_MODULE>();
    let buffer = get_extended_tcp_table(layout, AF_INET)?;

    let tcp_table: *const MIB_TCPTABLE_OWNER_MODULE = buffer.cast();

    // Read the length of the table
    let length = std::ptr::addr_of!((*tcp_table).dwNumEntries).read_unaligned() as usize;

    // Get a pointer to the start of the table
    let table_ptr: *const MIB_TCPROW_OWNER_MODULE = std::ptr::addr_of!((*tcp_table).table).cast();

    // Find the process IDs
    std::slice::from_raw_parts(table_ptr, length)
        .iter()
        .for_each(|element| {
            // Convert the port value
            let local_port: u16 = (element.dwLocalPort as u16).to_be();
            if local_port == port {
                pids.insert(element.dwOwningPid);
            }
        });

    // Deallocate the buffer memory
    std::alloc::dealloc(buffer, layout);

    Ok(())
}

/// Searches through the IPv6 extended TCP table for any processes
/// that are listening on the provided `port`. Will append any processes
/// found onto the provided `pids` set
///
/// # Arguments
///
/// * `port` The port to search for
/// * `pids` The set of process IDs to append to
unsafe fn get_process_tcp_v6(port: u16, pids: &mut HashSet<u32>) -> Result<(), Error> {
    // Create the memory layout for the table
    let layout = Layout::new::<MIB_TCP6TABLE_OWNER_MODULE>();
    let buffer = get_extended_tcp_table(layout, AF_INET6)?;

    let tcp_table: *const MIB_TCP6TABLE_OWNER_MODULE = buffer.cast();

    // Read the length of the table
    let length = std::ptr::addr_of!((*tcp_table).dwNumEntries).read_unaligned() as usize;

    // Get a pointer to the start of the table
    let table_ptr: *const MIB_TCP6ROW_OWNER_MODULE = std::ptr::addr_of!((*tcp_table).table).cast();

    // Find the process IDs
    std::slice::from_raw_parts(table_ptr, length)
        .iter()
        .for_each(|element| {
            // Convert the port value
            let local_port: u16 = (element.dwLocalPort as u16).to_be();
            if local_port == port {
                pids.insert(element.dwOwningPid);
            }
        });

    // Deallocate the buffer memory
    std::alloc::dealloc(buffer, layout);

    Ok(())
}

/// Searches through the IPv4 extended UDP table for any processes
/// that are listening on the provided `port`. Will append any processes
/// found onto the provided `pids` set
///
/// # Arguments
///
/// * `port` The port to search for
/// * `pids` The set of process IDs to append to
unsafe fn get_process_udp_v4(port: u16, pids: &mut HashSet<u32>) -> Result<(), Error> {
    // Create the memory layout for the table
    let layout = Layout::new::<MIB_UDPTABLE_OWNER_MODULE>();
    let buffer = get_extended_udp_table(layout, AF_INET)?;

    let udp_table: *const MIB_UDPTABLE_OWNER_MODULE = buffer.cast();

    // Read the length of the table
    let length = std::ptr::addr_of!((*udp_table).dwNumEntries).read_unaligned() as usize;

    // Get a pointer to the start of the table
    let table_ptr: *const MIB_UDPROW_OWNER_MODULE = std::ptr::addr_of!((*udp_table).table).cast();

    // Find the process IDs
    std::slice::from_raw_parts(table_ptr, length)
        .iter()
        .for_each(|element| {
            // Convert the port value
            let local_port: u16 = (element.dwLocalPort as u16).to_be();
            if local_port == port {
                pids.insert(element.dwOwningPid);
            }
        });

    // Deallocate the buffer memory
    std::alloc::dealloc(buffer, layout);
    Ok(())
}

/// Searches through the IPv6 extended UDP table for any processes
/// that are listening on the provided `port`. Will append any processes
/// found onto the provided `pids` set
///
/// # Arguments
///
/// * `port` The port to search for
/// * `pids` The set of process IDs to append to
unsafe fn get_process_udp_v6(port: u16, pids: &mut HashSet<u32>) -> Result<(), Error> {
    // Create the memory layout for the table
    let layout = Layout::new::<MIB_UDP6TABLE_OWNER_MODULE>();
    let buffer = get_extended_udp_table(layout, AF_INET6)?;

    let udp_table: *const MIB_UDP6TABLE_OWNER_MODULE = buffer.cast();

    // Read the length of the table
    let length = std::ptr::addr_of!((*udp_table).dwNumEntries).read_unaligned() as usize;

    // Get a pointer to the start of the table
    let table_ptr: *const MIB_UDP6ROW_OWNER_MODULE = std::ptr::addr_of!((*udp_table).table).cast();

    // Find the process IDs
    std::slice::from_raw_parts(table_ptr, length)
        .iter()
        .for_each(|element| {
            // Convert the port value
            let local_port: u16 = (element.dwLocalPort as u16).to_be();
            if local_port == port {
                pids.insert(element.dwOwningPid);
            }
        });

    // Deallocate the buffer memory
    std::alloc::dealloc(buffer, layout);
    Ok(())
}
