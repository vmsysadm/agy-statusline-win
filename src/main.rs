mod ansi;
mod config;
mod data;
mod layout;
mod segments;
mod theme;
mod yolo;

use std::env;
use std::io::{self, Read};

fn main() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);

    // Single-pass JSON: parse once, detect YOLO and ASCII mode, then consume
    let raw_val: serde_json::Value = serde_json::from_str(&input).unwrap_or_default();
    let yolo_from_json = yolo::detect_yolo_in_json(&raw_val);
    let raw_use_ascii = raw_val.get("mode").and_then(|v| v.as_str()).map(|m| m.eq_ignore_ascii_case("ascii")).unwrap_or(false)
        || raw_val.get("use_ascii").and_then(|v| v.as_bool()).unwrap_or(false)
        || raw_val.get("useAscii").and_then(|v| v.as_bool()).unwrap_or(false);
    let data: data::InputData = serde_json::from_value(raw_val).unwrap_or_default();

    let home_path = env::var("USERPROFILE").or_else(|_| env::var("HOME")).unwrap_or_default();
    let cfg = config::load_config(&home_path);

    let output_lines = layout::render_statusline(&data, &cfg, yolo_from_json, raw_use_ascii, &home_path);

    for line in output_lines {
        println!("{}", line);
    }
}
