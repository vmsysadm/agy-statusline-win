//! Parent-process inspection.
//!
//! Several decisions depend on *who invoked the statusline*: YOLO detection reads the
//! agy CLI's flags, and terminal-title emission has to know whether the invoking session
//! actually owns the console it would write to. Both walk the same chain, so the walk
//! lives here once.
//!
//! On Windows the chain is read directly via `NtQueryInformationProcess` +
//! `ReadProcessMemory` rather than a ToolHelp snapshot of every process on the box — the
//! statusline runs on every prompt refresh and a full snapshot costs tens of milliseconds.

/// One process in the ancestry chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProcessEntry {
    pub(crate) pid: u32,
    /// Full command line, when it could be read. A process whose memory is unreadable
    /// still contributes its pid so the walk can continue past it.
    pub(crate) cmdline: Option<String>,
}

/// Walk from this process up through its ancestors, nearest first.
///
/// Index 0 is the statusline process itself. The walk stops at the root, at a cycle, or
/// at `max_depth` entries, whichever comes first. An unreadable link ends the walk.
#[cfg(windows)]
pub(crate) fn ancestry(max_depth: usize) -> Vec<ProcessEntry> {
    let mut chain = Vec::new();
    let mut pid = std::process::id();

    for _ in 0..max_depth {
        let (parent_pid, cmdline) = match unsafe { read_process_info(pid) } {
            Some(info) => info,
            None => break,
        };
        chain.push(ProcessEntry { pid, cmdline });

        if parent_pid == 0 || parent_pid == pid {
            break;
        }
        pid = parent_pid;
    }

    chain
}

#[cfg(not(windows))]
pub(crate) fn ancestry(_max_depth: usize) -> Vec<ProcessEntry> {
    Vec::new()
}

/// Split a Windows command line into arguments.
///
/// Quote-aware but deliberately not a full CommandLineToArgvW: the callers only need the
/// executable path and whole-word flags, neither of which survives backslash escaping in
/// practice.
pub(crate) fn split_args(cmdline: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut started = false;

    for ch in cmdline.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                started = true;
            }
            c if c.is_whitespace() && !in_quotes => {
                if started {
                    args.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            c => {
                current.push(c);
                started = true;
            }
        }
    }
    if started {
        args.push(current);
    }

    args
}

/// Lowercased executable name of a command line, without directory or `.exe`.
/// `"C:\Users\u\AppData\Local\agy\bin\agy.exe" --print hi` -> `agy`.
pub(crate) fn executable_stem(cmdline: &str) -> Option<String> {
    let args = split_args(cmdline);
    let exe = args.first()?;
    let name = exe.rsplit(['\\', '/']).next().unwrap_or(exe).to_ascii_lowercase();
    Some(name.strip_suffix(".exe").unwrap_or(&name).to_string())
}

/// Read a process's parent pid and command line.
///
/// Uses `NtQueryInformationProcess` to reach the PEB, then `ReadProcessMemory` to pull
/// `ProcessParameters->CommandLine` out of it. Struct offsets are the x64 layout.
#[cfg(windows)]
unsafe fn read_process_info(pid: u32) -> Option<(u32, Option<String>)> {
    use std::ffi::c_void;
    use std::mem;
    use windows_sys::Win32::Foundation::*;
    use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
    use windows_sys::Win32::System::Threading::*;

    type NtQueryFn = unsafe extern "system" fn(HANDLE, u32, *mut c_void, u32, *mut u32) -> i32;

    const PROCESS_BASIC_INFORMATION_CLASS: u32 = 0;

    #[repr(C)]
    struct ProcessBasicInformation {
        _exit_status: i32,
        _pad0: i32,
        peb_base_address: *mut c_void,
        _affinity_mask: usize,
        _base_priority: i32,
        _pad1: i32,
        _unique_process_id: usize,
        inherited_from_unique_process_id: usize,
    }

    #[repr(C)]
    struct UnicodeString {
        length: u16,
        _maximum_length: u16,
        buffer: *mut u16,
    }

    let ntdll = GetModuleHandleA(b"ntdll.dll\0".as_ptr());
    if ntdll.is_null() {
        return None;
    }
    let func_ptr = GetProcAddress(ntdll, b"NtQueryInformationProcess\0".as_ptr());
    let nt_query: NtQueryFn = match func_ptr {
        Some(f) => mem::transmute(f),
        None => return None,
    };

    let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid);
    if handle.is_null() {
        return None;
    }

    let mut pbi: ProcessBasicInformation = mem::zeroed();
    let status = nt_query(
        handle,
        PROCESS_BASIC_INFORMATION_CLASS,
        &mut pbi as *mut _ as *mut c_void,
        mem::size_of::<ProcessBasicInformation>() as u32,
        std::ptr::null_mut(),
    );
    if status != 0 {
        let _ = CloseHandle(handle);
        return None;
    }

    let parent_pid = pbi.inherited_from_unique_process_id as u32;

    // Read ProcessParameters pointer from PEB (offset 0x20 on x64)
    let params_ptr_addr = (pbi.peb_base_address as usize + 0x20) as *const c_void;
    let mut params_ptr: *mut c_void = std::ptr::null_mut();
    let mut bytes_read: usize = 0;

    let ok = ReadProcessMemory(
        handle,
        params_ptr_addr,
        &mut params_ptr as *mut _ as *mut c_void,
        mem::size_of::<*mut c_void>(),
        &mut bytes_read,
    );
    if ok == 0 || params_ptr.is_null() {
        let _ = CloseHandle(handle);
        return Some((parent_pid, None));
    }

    // Read CommandLine UNICODE_STRING from RTL_USER_PROCESS_PARAMETERS (offset 0x70 on x64)
    let cmdline_addr = (params_ptr as usize + 0x70) as *const c_void;
    let mut cmdline_us: UnicodeString = mem::zeroed();
    let ok = ReadProcessMemory(
        handle,
        cmdline_addr,
        &mut cmdline_us as *mut _ as *mut c_void,
        mem::size_of::<UnicodeString>(),
        &mut bytes_read,
    );
    if ok == 0 || cmdline_us.buffer.is_null() || cmdline_us.length == 0 {
        let _ = CloseHandle(handle);
        return Some((parent_pid, None));
    }

    let char_count = (cmdline_us.length / 2) as usize;
    let mut buffer = vec![0u16; char_count];
    let ok = ReadProcessMemory(
        handle,
        cmdline_us.buffer as *const c_void,
        buffer.as_mut_ptr() as *mut c_void,
        cmdline_us.length as usize,
        &mut bytes_read,
    );
    let _ = CloseHandle(handle);

    if ok == 0 {
        Some((parent_pid, None))
    } else {
        Some((parent_pid, Some(String::from_utf16_lossy(&buffer))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_args() {
        assert_eq!(
            split_args(r#""C:\Program Files\agy\agy.exe" -p "do a thing""#),
            vec![r"C:\Program Files\agy\agy.exe", "-p", "do a thing"]
        );
        assert_eq!(split_args("agy   --continue"), vec!["agy", "--continue"]);
        assert!(split_args("   ").is_empty());
    }

    #[test]
    fn test_executable_stem() {
        assert_eq!(
            executable_stem(r#""C:\Users\u\AppData\Local\agy\bin\AGY.EXE" --print hi"#).as_deref(),
            Some("agy")
        );
        assert_eq!(executable_stem("/usr/local/bin/agy -p hi").as_deref(), Some("agy"));
        assert_eq!(executable_stem("pwsh -NoProfile").as_deref(), Some("pwsh"));
        assert_eq!(executable_stem("").as_deref(), None);
    }

    #[cfg(windows)]
    #[test]
    fn test_ancestry_includes_self() {
        let chain = ancestry(8);
        assert_eq!(chain.first().map(|e| e.pid), Some(std::process::id()));
    }
}
