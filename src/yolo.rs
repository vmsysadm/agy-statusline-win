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

    walk_parent_chain_for_yolo()
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

/// Walk the ancestry looking for a permission-skipping flag on any invoking process.
#[cfg(windows)]
fn walk_parent_chain_for_yolo() -> bool {
    crate::procinfo::ancestry(10)
        .iter()
        .filter_map(|e| e.cmdline.as_deref())
        .any(|cmd| {
            contains_ignore_ascii_case(cmd, "dangerously")
                || contains_ignore_ascii_case(cmd, "skip-permissions")
                || contains_ignore_ascii_case(cmd, "skippermissions")
        })
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
