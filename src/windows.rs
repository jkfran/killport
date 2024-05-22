use crate::killport::{Killable, KillableType};
use log::info;
use std::{
    alloc::{alloc, dealloc, Layout},
    collections::{HashMap, HashSet},
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
    name: String,
    parent: Option<Box<WindowsProcess>>,
}

impl WindowsProcess {
    pub fn new(pid: u32, name: String) -> Self {
        Self {
            pid,
            name,
            parent: None,
        }
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
    let lookup_table: ProcessLookupTable = ProcessLookupTable::create()?;
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

        let mut processes: Vec<WindowsProcess> = Vec::with_capacity(pids.len());

        for pid in pids {
            let process_name = lookup_table
                .process_names
                .get(&pid)
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string());

            let mut process = WindowsProcess::new(pid, process_name);

            // Resolve the process parents
            lookup_process_parents(&lookup_table, &mut process)?;

            processes.push(process);
        }

        processes
    };

    Ok(processes)
}

impl Killable for WindowsProcess {
    fn kill(&self, _signal: crate::signal::KillportSignal) -> Result<bool> {
        let mut killed = false;
        let mut next = Some(self);
        while let Some(current) = next {
            unsafe {
                kill_process(current)?;
            }

            killed = true;
            next = current.parent.as_ref().map(|value| value.as_ref());
        }

        Ok(killed)
    }

    fn get_type(&self) -> KillableType {
        KillableType::Process
    }

    fn get_name(&self) -> String {
        self.name.to_string()
    }
}

/// Checks if there is a running process with the provided pid
///
/// # Arguments
///
/// * `pid` - The process ID to search for
fn is_process_running(pid: u32) -> Result<bool> {
    let mut snapshot = WindowsProcessesSnapshot::create()?;
    let is_running = snapshot.any(|entry| entry.th32ProcessID == pid);
    Ok(is_running)
}

/// Lookup table for finding the names and parents for
/// a process using its pid
pub struct ProcessLookupTable {
    /// Mapping from pid to name
    process_names: HashMap<u32, String>,
    /// Mapping from pid to parent pid
    process_parents: HashMap<u32, u32>,
}

impl ProcessLookupTable {
    pub fn create() -> Result<Self> {
        let mut process_names: HashMap<u32, String> = HashMap::new();
        let mut process_parents: HashMap<u32, u32> = HashMap::new();

        WindowsProcessesSnapshot::create()?.for_each(|entry| {
            process_names.insert(entry.th32ProcessID, get_process_entry_name(&entry));
            process_parents.insert(entry.th32ProcessID, entry.th32ParentProcessID);
        });

        Ok(Self {
            process_names,
            process_parents,
        })
    }
}

/// Finds any parent processes of the provided process, adding
/// the process to the list of parents
///
/// WARNING - This worked in the previous versions because the implementation
/// was flawwed and didn't properly look up the tree of parents, trying to kill
/// all of the parents causes problems since you'll end up killing explorer.exe
/// or some other windows sys process. So I've limited the depth to a single process deep
///
/// # Arguments
///
/// * `process` - The process to collect parents for
fn lookup_process_parents(
    lookup_table: &ProcessLookupTable,
    process: &mut WindowsProcess,
) -> Result<()> {
    const MAX_PARENT_DEPTH: u8 = 1;

    let mut current_procces = process;
    let mut depth = 0;

    while let Some(&parent_pid) = lookup_table.process_parents.get(&current_procces.pid) {
        if depth == MAX_PARENT_DEPTH {
            break;
        }

        let process_name = lookup_table
            .process_names
            .get(&parent_pid)
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string());

        // Add the new parent process
        let parent = current_procces
            .parent
            .insert(Box::new(WindowsProcess::new(parent_pid, process_name)));

        current_procces = parent;
        depth += 1
    }

    Ok(())
}

/// Parses the name from a process entry, falls back to "Unknown"
/// for invalid names
///
/// # Arguments
///
/// * `entry` - The process entry
fn get_process_entry_name(entry: &PROCESSENTRY32) -> String {
    let name_chars = entry
        .szExeFile
        .iter()
        .copied()
        .take_while(|value| *value != 0)
        .collect();

    let name = String::from_utf8(name_chars);
    name.unwrap_or_else(|_| "Unknown".to_string())
}

/// Snapshot of the running windows processes that can be iterated to find
/// information about various processes such as parent processes and
/// process names
///
/// This is a safe abstraction
pub struct WindowsProcessesSnapshot {
    /// Handle to the snapshot
    handle: HANDLE,
    /// The memory for reading process entries
    entry: PROCESSENTRY32,
    /// State of reading
    state: SnapshotState,
}

/// State for the snapshot iterator
pub enum SnapshotState {
    /// Can read the first entry
    First,
    /// Can read the next entry
    Next,
    /// Reached the end, cannot iterate further always give [None]
    End,
}

impl WindowsProcessesSnapshot {
    /// Creates a new process snapshot to iterate
    pub fn create() -> Result<Self> {
        // Request a snapshot handle
        let handle: HANDLE = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };

        // Ensure we got a valid handle
        if handle == INVALID_HANDLE_VALUE {
            let error: WIN32_ERROR = unsafe { GetLastError() };
            return Err(Error::new(
                ErrorKind::Other,
                format!("Failed to get handle to processes: {:#x}", error),
            ));
        }

        // Allocate the memory to use for the entries
        let mut entry: PROCESSENTRY32 = unsafe { std::mem::zeroed() };
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;

        Ok(Self {
            handle,
            entry,
            state: SnapshotState::First,
        })
    }
}

impl Iterator for WindowsProcessesSnapshot {
    type Item = PROCESSENTRY32;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            SnapshotState::First => {
                // Process the first entry
                if unsafe { Process32First(self.handle, &mut self.entry) } == FALSE {
                    self.state = SnapshotState::End;
                    return None;
                }
                self.state = SnapshotState::Next;

                Some(self.entry)
            }
            SnapshotState::Next => {
                // Process the next entry
                if unsafe { Process32Next(self.handle, &mut self.entry) } == FALSE {
                    self.state = SnapshotState::End;
                    return None;
                }

                Some(self.entry)
            }
            SnapshotState::End => None,
        }
    }
}

impl Drop for WindowsProcessesSnapshot {
    fn drop(&mut self) {
        unsafe {
            // Close the handle now that its no longer needed
            CloseHandle(self.handle);
        }
    }
}

/// Kills a process with the provided process ID
///
/// # Arguments
///
/// * `process` - The process
unsafe fn kill_process(process: &WindowsProcess) -> Result<()> {
    info!("Killing process {}:{}", process.get_name(), process.pid);

    // Open the process handle with intent to terminate
    let handle: HANDLE = OpenProcess(PROCESS_TERMINATE, FALSE, process.pid);
    if handle == 0 {
        // If the process just isn't running we can ignore the error
        if !is_process_running(process.pid)? {
            return Ok(());
        }

        let error: WIN32_ERROR = GetLastError();
        return Err(Error::new(
            ErrorKind::Other,
            format!(
                "Failed to obtain handle to process {}:{}: {:#x}",
                process.get_name(),
                process.pid,
                error
            ),
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
            format!(
                "Failed to terminate process {}:{}: {:#x}",
                process.get_name(),
                process.pid,
                error
            ),
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
