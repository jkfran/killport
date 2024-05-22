use crate::killport::{Killable, KillableType};
use log::info;
use std::{
    alloc::{alloc, dealloc, Layout},
    collections::HashSet,
    ffi::c_void,
    io::{Error, ErrorKind, Result},
    ptr::addr_of,
    slice,
};
use windows_sys::Win32::{
    Foundation::{
        CloseHandle, GetLastError, BOOL, ERROR_INSUFFICIENT_BUFFER, FALSE, HANDLE,
        INVALID_HANDLE_VALUE, NO_ERROR, WIN32_ERROR,
    },
    NetworkManagement::IpHelper::{
        GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCP6ROW_OWNER_MODULE,
        MIB_TCP6TABLE_OWNER_MODULE, MIB_TCPROW_OWNER_MODULE, MIB_TCPTABLE_OWNER_MODULE,
        MIB_UDP6ROW_OWNER_MODULE, MIB_UDP6TABLE_OWNER_MODULE, MIB_UDPROW_OWNER_MODULE,
        MIB_UDPTABLE_OWNER_MODULE, TCP_TABLE_OWNER_MODULE_ALL, UDP_TABLE_OWNER_MODULE,
    },
    Networking::WinSock::{AF_INET, AF_INET6},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32,
            TH32CS_SNAPPROCESS,
        },
        Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE},
    },
};

/// Represents a windows native process
#[derive(Debug)]
pub struct WindowsProcess {
    pid: u32,
    name: Option<String>,
}

impl WindowsProcess {
    pub fn new(pid: u32, name: Option<String>) -> Self {
        Self { pid, name }
    }
}

/// Finds the processes associated with the specified `port`.
///
/// Returns a `Vec` of native processes.
///
/// # Arguments
///
/// * `port` - Target port number
pub fn find_target_processes(port: u16) -> Result<Vec<WindowsProcess>> {
    let mut pids: HashSet<u32> = HashSet::new();

    let processes = unsafe {
        // Find processes in the TCP IPv4 table
        use_extended_table::<MIB_TCPTABLE_OWNER_MODULE>(port, &mut pids)?;

        // Find processes in the TCP IPv6 table
        use_extended_table::<MIB_TCP6TABLE_OWNER_MODULE>(port, &mut pids)?;

        // Find processes in the UDP IPv4 table
        use_extended_table::<MIB_UDPTABLE_OWNER_MODULE>(port, &mut pids)?;

        // Find processes in the UDP IPv6 table
        use_extended_table::<MIB_UDP6TABLE_OWNER_MODULE>(port, &mut pids)?;

        // Collect parents of the PIDs
        collect_parents(&mut pids)?;

        // Collect the processes
        let mut processes: Vec<WindowsProcess> = pids
            .into_iter()
            .map(|pid| WindowsProcess::new(pid, None))
            .collect();

        lookup_proccess_names(&mut processes)?;

        processes
    };

    Ok(processes)
}

impl Killable for WindowsProcess {
    fn kill(&self, _signal: crate::signal::KillportSignal) -> Result<bool> {
        let mut pids: HashSet<u32> = HashSet::new();
        pids.insert(self.pid);

        if pids.is_empty() {
            return Ok(false);
        }

        unsafe {
            collect_parents(&mut pids)?;

            for pid in pids {
                kill_process(pid)?;
            }
        };

        Ok(true)
    }

    fn get_type(&self) -> KillableType {
        KillableType::Process
    }

    fn get_name(&self) -> String {
        match self.name.as_ref() {
            Some(value) => value.to_string(),
            None => "Unknown".to_string(),
        }
    }
}

/// Collects the names for the processes in the provided collection of
/// processes. If name resolving fails that process is just "Unknown"
///
/// # Arguments
///
/// * `processes` - The set of processes to resolve the names of
unsafe fn lookup_proccess_names(processes: &mut [WindowsProcess]) -> Result<()> {
    // Request a snapshot handle
    let handle: HANDLE = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);

    // Ensure we got a valid handle
    if handle == INVALID_HANDLE_VALUE {
        let error: WIN32_ERROR = GetLastError();
        return Err(Error::new(
            ErrorKind::Other,
            format!("Failed to get handle to processes: {:#x}", error),
        ));
    }

    // Allocate the memory to use for the entries
    let mut entry: PROCESSENTRY32 = std::mem::zeroed();
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;

    // Process the first item
    if Process32First(handle, &mut entry) != FALSE {
        loop {
            let target_process = processes
                .iter_mut()
                .find(|proc| proc.pid == entry.th32ProcessID);
            if let Some(target_process) = target_process {
                let name_chars = entry
                    .szExeFile
                    .iter()
                    .copied()
                    .take_while(|value| *value != 0)
                    .collect();

                let name = String::from_utf8(name_chars);
                if let Ok(name) = name {
                    target_process.name = Some(name)
                }
            }

            // Process the next entry
            if Process32Next(handle, &mut entry) == FALSE {
                break;
            }
        }
    }

    // Close the handle now that its no longer needed
    CloseHandle(handle);

    Ok(())
}

/// Collects all the parent processes for the PIDs in
/// the provided set
///
/// # Arguments
///
/// * `pids` - The set to match PIDs from and insert PIDs into
unsafe fn collect_parents(pids: &mut HashSet<u32>) -> Result<()> {
    // Request a snapshot handle
    let handle: HANDLE = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);

    // Ensure we got a valid handle
    if handle == INVALID_HANDLE_VALUE {
        let error: WIN32_ERROR = GetLastError();
        return Err(Error::new(
            ErrorKind::Other,
            format!("Failed to get handle to processes: {:#x}", error),
        ));
    }

    // Allocate the memory to use for the entries
    let mut entry: PROCESSENTRY32 = std::mem::zeroed();
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;

    // Process the first item
    if Process32First(handle, &mut entry) != FALSE {
        let mut count = 0;

        loop {
            // Add matching processes to the output
            if pids.contains(&entry.th32ProcessID) {
                pids.insert(entry.th32ParentProcessID);
                count += 1;
            }

            // Process the next entry
            if Process32Next(handle, &mut entry) == FALSE {
                break;
            }
        }

        info!("Collected {} parent processes", count);
    }

    // Close the handle now that its no longer needed
    CloseHandle(handle);

    Ok(())
}

/// Kills a process with the provided process ID
///
/// # Arguments
///
/// * `pid` - The process ID
unsafe fn kill_process(pid: u32) -> Result<()> {
    info!("Killing process with PID {}", pid);

    // Open the process handle with intent to terminate
    let handle: HANDLE = OpenProcess(PROCESS_TERMINATE, FALSE, pid);
    if handle == 0 {
        let error: WIN32_ERROR = GetLastError();
        return Err(Error::new(
            ErrorKind::Other,
            format!("Failed to obtain handle to process {}: {:#x}", pid, error),
        ));
    }

    // Terminate the process
    let result: BOOL = TerminateProcess(handle, 0);

    // Close the handle now that its no longer needed
    CloseHandle(handle);

    if result == FALSE {
        let error: WIN32_ERROR = GetLastError();
        return Err(Error::new(
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
unsafe fn use_extended_table<T>(port: u16, pids: &mut HashSet<u32>) -> Result<()>
where
    T: TableClass,
{
    // Allocation of initial memory
    let mut layout: Layout = Layout::new::<T>();
    let mut buffer: *mut u8 = alloc(layout);

    // Current buffer size later changed by the fn call to be the estimated size
    // for resizing the buffer
    let mut size: u32 = layout.size() as u32;

    // Result of asking for the table
    let mut result: WIN32_ERROR;

    loop {
        // Ask windows for the extended table
        result = (T::TABLE_FN)(
            buffer.cast(),
            &mut size,
            FALSE,
            T::FAMILY,
            T::TABLE_CLASS,
            0,
        );

        // No error occurred
        if result == NO_ERROR {
            break;
        }

        // Always deallocate the memory regardless of the error
        // (Resizing needs to reallocate the memory anyway)
        dealloc(buffer, layout);

        // Handle buffer too small
        if result == ERROR_INSUFFICIENT_BUFFER {
            // Create the new memory layout from the new size and previous alignment
            layout = Layout::from_size_align_unchecked(size as usize, layout.align());
            // Allocate the new chunk of memory
            buffer = alloc(layout);
            continue;
        }

        // Handle unknown failures
        return Err(Error::new(
            ErrorKind::Other,
            format!(
                "Failed to get size estimate for extended table: {:#x}",
                result
            ),
        ));
    }

    let table: *const T = buffer.cast();

    // Obtain the processes from the table
    T::get_processes(table, port, pids);

    // Deallocate the buffer memory
    dealloc(buffer, layout);

    Ok(())
}

/// Type of the GetExtended[UDP/TCP]Table Windows API function
type GetExtendedTable =
    unsafe extern "system" fn(*mut c_void, *mut u32, i32, AddressFamily, i32, u32) -> WIN32_ERROR;

/// For some reason the actual INET types are u16 so this
/// is just a casted version to u32
type AddressFamily = u32;

/// IPv4 Address family
const INET: AddressFamily = AF_INET as u32;
/// IPv6 Address family
const INET6: AddressFamily = AF_INET6 as u32;

/// Table class type (either TCP_TABLE_CLASS for TCP or UDP_TABLE_CLASS for UDP)
type TableClassType = i32;

/// TCP class type for the owner to module mappings
const TCP_TYPE: TableClassType = TCP_TABLE_OWNER_MODULE_ALL;
/// UDP class type for the owner to module mappings
const UDP_TYPE: TableClassType = UDP_TABLE_OWNER_MODULE;

/// Trait implemented by extended tables that can
/// be enumerated for processes that match a
/// specific PID
trait TableClass {
    /// Windows function for loading this table class
    const TABLE_FN: GetExtendedTable;

    /// Address family type
    const FAMILY: AddressFamily;

    /// Windows table class type
    const TABLE_CLASS: TableClassType;

    /// Iterates the contents of the extended table inserting any
    /// process entires that match the provided `port` into the
    /// `pids` set
    ///
    /// # Arguments
    ///
    /// * `table` - The pointer to the table class
    /// * `port` - The port to search for
    /// * `pids` - The process IDs to insert into
    unsafe fn get_processes(table: *const Self, port: u16, pids: &mut HashSet<u32>);
}

/// Implementation for get_processes is identical for all of the
/// implementations only difference is the type of row pointer
/// other than that all the fields accessed are the same to in
/// order to prevent repeating this its a macro now
macro_rules! impl_get_processes {
    ($ty:ty) => {
        unsafe fn get_processes(table: *const Self, port: u16, pids: &mut HashSet<u32>) {
            let row_ptr: *const $ty = addr_of!((*table).table).cast();
            let length: usize = addr_of!((*table).dwNumEntries).read_unaligned() as usize;

            slice::from_raw_parts(row_ptr, length)
                .iter()
                .for_each(|element| {
                    // Convert the port value
                    let local_port: u16 = (element.dwLocalPort as u16).to_be();
                    if local_port == port {
                        pids.insert(element.dwOwningPid);
                    }
                });
        }
    };
}

/// TCP IPv4 table class
impl TableClass for MIB_TCPTABLE_OWNER_MODULE {
    const TABLE_FN: GetExtendedTable = GetExtendedTcpTable;
    const FAMILY: AddressFamily = INET;
    const TABLE_CLASS: TableClassType = TCP_TYPE;

    impl_get_processes!(MIB_TCPROW_OWNER_MODULE);
}

/// TCP IPv6 table class
impl TableClass for MIB_TCP6TABLE_OWNER_MODULE {
    const TABLE_FN: GetExtendedTable = GetExtendedTcpTable;
    const FAMILY: AddressFamily = INET6;
    const TABLE_CLASS: TableClassType = TCP_TYPE;

    impl_get_processes!(MIB_TCP6ROW_OWNER_MODULE);
}

/// UDP IPv4 table class
impl TableClass for MIB_UDPTABLE_OWNER_MODULE {
    const TABLE_FN: GetExtendedTable = GetExtendedUdpTable;
    const FAMILY: AddressFamily = INET;
    const TABLE_CLASS: TableClassType = UDP_TYPE;

    impl_get_processes!(MIB_UDPROW_OWNER_MODULE);
}

/// UDP IPv6 table class
impl TableClass for MIB_UDP6TABLE_OWNER_MODULE {
    const TABLE_FN: GetExtendedTable = GetExtendedUdpTable;
    const FAMILY: AddressFamily = INET6;
    const TABLE_CLASS: TableClassType = UDP_TYPE;

    impl_get_processes!(MIB_UDP6ROW_OWNER_MODULE);
}
