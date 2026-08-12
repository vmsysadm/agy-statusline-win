/// Recursively search a JSON value for YOLO / auto-approve signals.
pub(crate) fn detect_yolo_in_json(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Object(map) => {
            for (key, val) in map {
                let k_lower = key.to_lowercase();

                if k_lower == "sandbox" || k_lower == "cwd" || k_lower == "conversation_id" || k_lower == "conversationid" {
                    continue;
                }

                let is_yolo_key = k_lower.contains("yolo")
                    || k_lower.contains("dangerously")
                    || k_lower.contains("skippermission")
                    || k_lower.contains("skip_permission")
                    || k_lower.contains("autoapprove")
                    || k_lower.contains("auto_approve")
                    || k_lower.contains("approval")
                    || k_lower.contains("permission")
                    || k_lower == "mode";

                if is_yolo_key {
                    match val {
                        serde_json::Value::Bool(b) => {
                            if *b {
                                return true;
                            }
                        }
                        serde_json::Value::String(s) => {
                            let s_lower = s.to_lowercase();
                            if s_lower == "yolo"
                                || s_lower == "auto_approve"
                                || s_lower == "auto-approve"
                                || s_lower == "autoapprove"
                                || s_lower == "skip"
                                || s_lower == "true"
                                || s_lower == "enabled"
                            {
                                return true;
                            }
                        }
                        _ => {}
                    }
                } else if let serde_json::Value::String(s) = val {
                    let s_lower = s.to_lowercase();
                    if s_lower == "yolo" || s_lower == "auto_approve" || s_lower == "auto-approve" {
                        return true;
                    }
                }

                if detect_yolo_in_json(val) {
                    return true;
                }
            }
            false
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                if let serde_json::Value::String(s) = item {
                    let s_lower = s.to_lowercase();
                    if s_lower.contains("dangerously") || s_lower.contains("yolo") || s_lower.contains("skip_permission") {
                        return true;
                    }
                }
                if detect_yolo_in_json(item) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

/// Check the parent process chain for YOLO flags.
///
/// On Windows this walks up the parent process tree using the ToolHelp API
/// and reads each parent's command line via NtQueryInformationProcess.
/// On other platforms it only checks the AGY_YOLO environment variable.
#[cfg(windows)]
pub(crate) fn check_parent_cmdline_for_yolo() -> bool {
    // Fast path: environment variable
    if let Ok(v) = std::env::var("AGY_YOLO") {
        if v == "1" || v.eq_ignore_ascii_case("true") {
            return true;
        }
    }

    unsafe { walk_parent_chain_for_yolo() }
}

#[cfg(not(windows))]
pub(crate) fn check_parent_cmdline_for_yolo() -> bool {
    if let Ok(v) = std::env::var("AGY_YOLO") {
        if v == "1" || v.eq_ignore_ascii_case("true") {
            return true;
        }
    }
    false
}

#[cfg(windows)]
unsafe fn walk_parent_chain_for_yolo() -> bool {
    use std::collections::HashMap;
    use std::mem;
    use windows_sys::Win32::Foundation::*;
    use windows_sys::Win32::System::Diagnostics::ToolHelp::*;

    let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if snapshot == INVALID_HANDLE_VALUE {
        return false;
    }

    // Build a PID -> (parent_pid, exe_name) map from the process snapshot
    let mut process_map: HashMap<u32, (u32, String)> = HashMap::new();
    let mut entry: PROCESSENTRY32W = mem::zeroed();
    entry.dwSize = mem::size_of::<PROCESSENTRY32W>() as u32;

    if Process32FirstW(snapshot, &mut entry) != 0 {
        loop {
            let name_len = entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len());
            let exe_name =
                String::from_utf16_lossy(&entry.szExeFile[..name_len]).to_lowercase();
            process_map.insert(
                entry.th32ProcessID,
                (entry.th32ParentProcessID, exe_name),
            );
            if Process32NextW(snapshot, &mut entry) == 0 {
                break;
            }
        }
    }
    let _ = CloseHandle(snapshot);

    // Walk up the parent chain from the current PID (max 10 levels)
    let mut current_pid = std::process::id();
    for _ in 0..10 {
        let (parent_pid, exe_name) = match process_map.get(&current_pid) {
            Some(info) => info.clone(),
            None => break,
        };

        // Only inspect processes that look like the Antigravity CLI
        if exe_name.contains("node")
            || exe_name.contains("agy")
            || exe_name.contains("antigravity")
        {
            if let Some(cmdline) = read_process_command_line(current_pid) {
                let lower = cmdline.to_lowercase();
                if lower.contains("dangerously")
                    || lower.contains("skip-permissions")
                    || lower.contains("skippermissions")
                {
                    return true;
                }
            }
        }

        if parent_pid == 0 || parent_pid == current_pid {
            break;
        }
        current_pid = parent_pid;
    }
    false
}

/// Read a remote process's command line via NtQueryInformationProcess + ReadProcessMemory.
#[cfg(windows)]
unsafe fn read_process_command_line(pid: u32) -> Option<String> {
    use std::ffi::c_void;
    use std::mem;
    use windows_sys::Win32::Foundation::*;
    use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
    use windows_sys::Win32::System::Threading::*;

    // NtQueryInformationProcess function pointer type
    type NtQueryFn = unsafe extern "system" fn(
        HANDLE,  // ProcessHandle
        u32,     // ProcessInformationClass
        *mut c_void, // ProcessInformation
        u32,     // ProcessInformationLength
        *mut u32, // ReturnLength
    ) -> i32;

    const PROCESS_BASIC_INFORMATION_CLASS: u32 = 0;

    #[repr(C)]
    struct ProcessBasicInformation {
        _reserved1: *mut c_void,
        peb_base_address: *mut c_void,
        _reserved2: [*mut c_void; 2],
        _unique_process_id: usize,
        _reserved3: *mut c_void,
    }

    #[repr(C)]
    struct UnicodeString {
        length: u16,
        _maximum_length: u16,
        buffer: *mut u16,
    }

    // Load NtQueryInformationProcess from ntdll.dll
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

    // Get PEB address
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
        return None;
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
        return None;
    }

    // Read the actual command line string
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
        return None;
    }

    Some(String::from_utf16_lossy(&buffer))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_yolo_in_json() {
        let v1: serde_json::Value = serde_json::from_str(r#"{"sandbox": {"enabled": false}}"#).unwrap();
        assert!(!detect_yolo_in_json(&v1));

        let v2: serde_json::Value = serde_json::from_str(r#"{"flags": {"dangerouslySkipPermissions": true}}"#).unwrap();
        assert!(detect_yolo_in_json(&v2));

        let v3: serde_json::Value = serde_json::from_str(r#"{"config": {"autoApprove": true}}"#).unwrap();
        assert!(detect_yolo_in_json(&v3));

        let v4: serde_json::Value = serde_json::from_str(r#"{"mode": "yolo"}"#).unwrap();
        assert!(detect_yolo_in_json(&v4));
    }
}
