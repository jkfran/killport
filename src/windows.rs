use crate::KillPortSignalOptions;
use std::alloc::Layout;
use std::io::{Error, ErrorKind};
use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
use windows_sys::Win32::{
    Foundation::{ERROR_INSUFFICIENT_BUFFER, NO_ERROR},
    NetworkManagement::IpHelper::{
        GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCP6ROW_OWNER_MODULE,
        MIB_TCP6TABLE_OWNER_MODULE, MIB_TCPROW_OWNER_MODULE, MIB_TCPTABLE_OWNER_MODULE,
        MIB_UDP6ROW_OWNER_MODULE, MIB_UDP6TABLE_OWNER_MODULE, MIB_UDPROW_OWNER_MODULE,
        MIB_UDPTABLE_OWNER_MODULE, TCP_TABLE_OWNER_MODULE_ALL, UDP_TABLE_OWNER_MODULE,
    },
    Networking::WinSock::{ADDRESS_FAMILY, AF_INET, AF_INET6},
};

pub fn kill_processes_by_port(port: u16, _: KillPortSignalOptions) -> Result<bool, Error> {
    let mut pids = Vec::new();

    unsafe { get_process_tcp_v4(port, &mut pids)? }
    unsafe { get_process_tcp_v6(port, &mut pids)? }
    unsafe { get_process_udp_v4(port, &mut pids)? }
    unsafe { get_process_udp_v6(port, &mut pids)? }

    let mut killed = false;

    for pid in pids {
        unsafe { kill_process(pid)? }
        killed = true;
    }

    Ok(killed)
}

unsafe fn kill_process(pid: u32) -> Result<(), Error> {
    // Open the process handle with intent to terminate
    let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
    if (&handle as *const isize).is_null() {
        return Err(std::io::Error::new(
            ErrorKind::Other,
            "Failed to obtain handle to process",
        ));
    }

    let result = TerminateProcess(handle, 0);
    if result == 0 {
        return Err(std::io::Error::new(
            ErrorKind::Other,
            "Failed to terminate process",
        ));
    }

    Ok(())
}

/// Reads the extended TCP table into memory using the provided address `family`
/// to determine the output type. Returns the memory pointer to the loaded struct
///
/// `layout` The layout of the memory
/// `family` The address family type
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
/// `layout` The layout of the memory
/// `family` The address family type
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
/// found onto the provided `pids` list
unsafe fn get_process_tcp_v4(port: u16, pids: &mut Vec<u32>) -> Result<(), Error> {
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
                pids.push(element.dwOwningPid)
            }
        });

    // Deallocate the buffer memory
    std::alloc::dealloc(buffer, layout);

    Ok(())
}

/// Searches through the IPv6 extended TCP table for any processes
/// that are listening on the provided `port`. Will append any processes
/// found onto the provided `pids` list
unsafe fn get_process_tcp_v6(port: u16, pids: &mut Vec<u32>) -> Result<(), Error> {
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
                pids.push(element.dwOwningPid)
            }
        });

    // Deallocate the buffer memory
    std::alloc::dealloc(buffer, layout);

    Ok(())
}

/// Searches through the IPv4 extended UDP table for any processes
/// that are listening on the provided `port`. Will append any processes
/// found onto the provided `pids` list
unsafe fn get_process_udp_v4(port: u16, pids: &mut Vec<u32>) -> Result<(), Error> {
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
                pids.push(element.dwOwningPid)
            }
        });

    // Deallocate the buffer memory
    std::alloc::dealloc(buffer, layout);
    Ok(())
}

/// Searches through the IPv6 extended UDP table for any processes
/// that are listening on the provided `port`. Will append any processes
/// found onto the provided `pids` list
unsafe fn get_process_udp_v6(port: u16, pids: &mut Vec<u32>) -> Result<(), Error> {
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
                pids.push(element.dwOwningPid)
            }
        });

    // Deallocate the buffer memory
    std::alloc::dealloc(buffer, layout);
    Ok(())
}
