#[inline]
fn contains_ignore_ascii_case(haystack: &str, needle: &str) -> bool {
    if needle.len() > haystack.len() {
        return false;
    }
    haystack.as_bytes().windows(needle.len()).any(|w| w.eq_ignore_ascii_case(needle.as_bytes()))
}

/// Recursively search a JSON value for YOLO / auto-approve signals.
pub(crate) fn detect_yolo_in_json(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Object(map) => {
            for (key, val) in map {
                if key.eq_ignore_ascii_case("sandbox")
                    || key.eq_ignore_ascii_case("cwd")
                    || key.eq_ignore_ascii_case("conversation_id")
                    || key.eq_ignore_ascii_case("conversationid")
                {
                    continue;
                }

                let is_yolo_key = contains_ignore_ascii_case(key, "yolo")
                    || contains_ignore_ascii_case(key, "dangerously")
                    || contains_ignore_ascii_case(key, "skippermission")
                    || contains_ignore_ascii_case(key, "skip_permission")
                    || contains_ignore_ascii_case(key, "autoapprove")
                    || contains_ignore_ascii_case(key, "auto_approve")
                    || contains_ignore_ascii_case(key, "approval")
                    || contains_ignore_ascii_case(key, "permission")
                    || key.eq_ignore_ascii_case("mode");

                if is_yolo_key {
                    match val {
                        serde_json::Value::Bool(b) => {
                            if *b {
                                return true;
                            }
                        }
                        serde_json::Value::String(s) => {
                            if s.eq_ignore_ascii_case("yolo")
                                || s.eq_ignore_ascii_case("auto_approve")
                                || s.eq_ignore_ascii_case("auto-approve")
                                || s.eq_ignore_ascii_case("autoapprove")
                                || s.eq_ignore_ascii_case("skip")
                                || s.eq_ignore_ascii_case("true")
                                || s.eq_ignore_ascii_case("enabled")
                            {
                                return true;
                            }
                        }
                        _ => {}
                    }
                } else if let serde_json::Value::String(s) = val {
                    if s.eq_ignore_ascii_case("yolo")
                        || s.eq_ignore_ascii_case("auto_approve")
                        || s.eq_ignore_ascii_case("auto-approve")
                    {
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
                    if contains_ignore_ascii_case(s, "dangerously")
                        || contains_ignore_ascii_case(s, "yolo")
                        || contains_ignore_ascii_case(s, "skip_permission")
                    {
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
/// On Windows this walks up the parent process tree directly using
/// NtQueryInformationProcess (without snapshotting all OS processes).
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
    let mut current_pid = std::process::id();

    for _ in 0..10 {
        let (parent_pid, cmdline) = match read_process_info(current_pid) {
            Some(info) => info,
            None => break,
        };

        if let Some(cmd) = cmdline {
            if contains_ignore_ascii_case(&cmd, "dangerously")
                || contains_ignore_ascii_case(&cmd, "skip-permissions")
                || contains_ignore_ascii_case(&cmd, "skippermissions")
            {
                return true;
            }
        }

        if parent_pid == 0 || parent_pid == current_pid {
            break;
        }
        current_pid = parent_pid;
    }
    false
}

/// Read a remote process's parent PID and command line via NtQueryInformationProcess + ReadProcessMemory.
#[cfg(windows)]
unsafe fn read_process_info(pid: u32) -> Option<(u32, Option<String>)> {
    use std::ffi::c_void;
    use std::mem;
    use windows_sys::Win32::Foundation::*;
    use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
    use windows_sys::Win32::System::Threading::*;

    type NtQueryFn = unsafe extern "system" fn(
        HANDLE,
        u32,
        *mut c_void,
        u32,
        *mut u32,
    ) -> i32;

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
