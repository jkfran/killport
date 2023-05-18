use crate::KillPortSignalOptions;
use log::{debug, info};
use std::{
    alloc::Layout,
    collections::HashSet,
    ffi::c_void,
    io::{Error, ErrorKind},
    ptr::addr_of,
};
use windows_sys::Win32::{
    Foundation::{
        CloseHandle, GetLastError, ERROR_INSUFFICIENT_BUFFER, INVALID_HANDLE_VALUE, NO_ERROR,
    },
    NetworkManagement::IpHelper::{
        GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCP6ROW_OWNER_MODULE,
        MIB_TCP6TABLE_OWNER_MODULE, MIB_TCPROW_OWNER_MODULE, MIB_TCPTABLE_OWNER_MODULE,
        MIB_UDP6ROW_OWNER_MODULE, MIB_UDP6TABLE_OWNER_MODULE, MIB_UDPROW_OWNER_MODULE,
        MIB_UDPTABLE_OWNER_MODULE, TCP_TABLE_CLASS, TCP_TABLE_OWNER_MODULE_ALL, UDP_TABLE_CLASS,
        UDP_TABLE_OWNER_MODULE,
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
        // Find processes in the TCP IPv4 table
        use_extended_table::<MIB_TCPTABLE_OWNER_MODULE>(port, &mut pids)?;

        // Find processes in the TCP IPv6 table
        use_extended_table::<MIB_TCP6TABLE_OWNER_MODULE>(port, &mut pids)?;

        // Find processes in the UDP IPv4 table
        use_extended_table::<MIB_UDPTABLE_OWNER_MODULE>(port, &mut pids)?;

        // Find processes in the UDP IPv6 table
        use_extended_table::<MIB_UDP6TABLE_OWNER_MODULE>(port, &mut pids)?;

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
    let mut entry: PROCESSENTRY32 = std::mem::zeroed();
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;

    // Process the first item
    if Process32First(handle, &mut entry) != 0 {
        let mut count = 0;

        loop {
            // Add matching processes to the output
            if pids.contains(&entry.th32ProcessID) {
                pids.insert(entry.th32ParentProcessID);
                count += 1;
            }

            // Process the next entry
            if Process32Next(handle, &mut entry) == 0 {
                break;
            }
        }

        info!("Collected {} parent processes", count);
    }

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

/// Reads the extended table of the specified generic [`TableClass`] iterating
/// the processes in that extended table checking if any bind the provided `port`
/// those that do will have the process ID inserted into `pids`
///
/// # Arguments
///
/// * `port` - The port to check for
/// * `pids` - The output list of process IDs
unsafe fn use_extended_table<T>(port: u16, pids: &mut HashSet<u32>) -> Result<(), Error>
where
    T: TableClass,
{
    let mut layout = Layout::new::<T>();
    let mut buffer = std::alloc::alloc(layout);

    // Size estimate for resizing the buffer
    let mut size = 0;

    // Result of asking for the TCP table
    let mut result: u32;

    loop {
        // Ask windows for the extended table
        result = (T::TABLE_FN)(
            buffer.cast(),
            &mut size,
            1,
            T::FAMILY as u32,
            T::TABLE_CLASS,
            0,
        );

        // No error occurred
        if result == NO_ERROR {
            break;
        }

        // Handle buffer too small
        if result == ERROR_INSUFFICIENT_BUFFER {
            // Deallocate the old memory layout
            std::alloc::dealloc(buffer, layout);

            // Create the new memory layout from the new size and previous alignment
            layout = Layout::from_size_align_unchecked(size as usize, layout.align());
            // Allocate the new chunk of memory
            buffer = std::alloc::alloc(layout);
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

    let table: *const T = buffer.cast();
    // Obtain the processes from the table
    T::get_processes(table, port, pids);

    // Deallocate the buffer memory
    std::alloc::dealloc(buffer, layout);

    Ok(())
}

/// Type of the GetExtended[UDP/TCP]Table Windows API function
type GetExtendedTable = unsafe extern "system" fn(*mut c_void, *mut u32, i32, u32, i32, u32) -> u32;

/// Trait implemented by extended tables that can
/// be enumerated for processes that match a
/// specific PID
trait TableClass {
    /// Windows function for loading this table class
    const TABLE_FN: GetExtendedTable;

    /// Address family type
    const FAMILY: ADDRESS_FAMILY;

    /// Windows table class type
    const TABLE_CLASS: i32;

    /// Iterates the contents of the extended table inserting any
    /// process entires that match the provided `port` into the
    /// `pids` set
    ///
    /// # Arguments
    ///
    /// * `port` - The port to search for
    /// * `pids` - The process IDs to insert into
    unsafe fn get_processes(table: *const Self, port: u16, pids: &mut HashSet<u32>);
}

impl TableClass for MIB_TCPTABLE_OWNER_MODULE {
    const TABLE_FN: GetExtendedTable = GetExtendedTcpTable;
    const FAMILY: ADDRESS_FAMILY = AF_INET;
    const TABLE_CLASS: TCP_TABLE_CLASS = TCP_TABLE_OWNER_MODULE_ALL;

    unsafe fn get_processes(table: *const Self, port: u16, pids: &mut HashSet<u32>) {
        let row_ptr: *const MIB_TCPROW_OWNER_MODULE = addr_of!((*table).table).cast();
        let length: usize = addr_of!((*table).dwNumEntries).read_unaligned() as usize;

        std::slice::from_raw_parts(row_ptr, length)
            .iter()
            .for_each(|element| {
                // Convert the port value
                let local_port: u16 = (element.dwLocalPort as u16).to_be();
                if local_port == port {
                    pids.insert(element.dwOwningPid);
                }
            });
    }
}

impl TableClass for MIB_TCP6TABLE_OWNER_MODULE {
    const TABLE_FN: GetExtendedTable = GetExtendedTcpTable;
    const FAMILY: ADDRESS_FAMILY = AF_INET6;
    const TABLE_CLASS: TCP_TABLE_CLASS = TCP_TABLE_OWNER_MODULE_ALL;

    unsafe fn get_processes(table: *const Self, port: u16, pids: &mut HashSet<u32>) {
        let row_ptr: *const MIB_TCP6ROW_OWNER_MODULE = addr_of!((*table).table).cast();
        let length: usize = addr_of!((*table).dwNumEntries).read_unaligned() as usize;

        std::slice::from_raw_parts(row_ptr, length)
            .iter()
            .for_each(|element| {
                // Convert the port value
                let local_port: u16 = (element.dwLocalPort as u16).to_be();
                if local_port == port {
                    pids.insert(element.dwOwningPid);
                }
            });
    }
}

impl TableClass for MIB_UDPTABLE_OWNER_MODULE {
    const TABLE_FN: GetExtendedTable = GetExtendedUdpTable;
    const FAMILY: ADDRESS_FAMILY = AF_INET;
    const TABLE_CLASS: UDP_TABLE_CLASS = UDP_TABLE_OWNER_MODULE;

    unsafe fn get_processes(table: *const Self, port: u16, pids: &mut HashSet<u32>) {
        let row_ptr: *const MIB_UDPROW_OWNER_MODULE = addr_of!((*table).table).cast();
        let length: usize = addr_of!((*table).dwNumEntries).read_unaligned() as usize;

        std::slice::from_raw_parts(row_ptr, length)
            .iter()
            .for_each(|element| {
                // Convert the port value
                let local_port: u16 = (element.dwLocalPort as u16).to_be();
                if local_port == port {
                    pids.insert(element.dwOwningPid);
                }
            });
    }
}

impl TableClass for MIB_UDP6TABLE_OWNER_MODULE {
    const TABLE_FN: GetExtendedTable = GetExtendedUdpTable;
    const FAMILY: ADDRESS_FAMILY = AF_INET6;
    const TABLE_CLASS: UDP_TABLE_CLASS = UDP_TABLE_OWNER_MODULE;

    unsafe fn get_processes(table: *const Self, port: u16, pids: &mut HashSet<u32>) {
        let row_ptr: *const MIB_UDP6ROW_OWNER_MODULE = addr_of!((*table).table).cast();
        let length: usize = addr_of!((*table).dwNumEntries).read_unaligned() as usize;

        std::slice::from_raw_parts(row_ptr, length)
            .iter()
            .for_each(|element| {
                // Convert the port value
                let local_port: u16 = (element.dwLocalPort as u16).to_be();
                if local_port == port {
                    pids.insert(element.dwOwningPid);
                }
            });
    }
}
