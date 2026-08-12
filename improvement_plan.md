# Statusline Performance & Refactoring Plan

## Goal

Improve the runtime performance of `agy-statusline-win` and refactor the monolithic 1408-line `src/main.rs` into a well-structured, maintainable multi-module crate. The result must remain a single binary, pass all existing tests, and produce identical ANSI output.

---

## Current State Summary

| Metric | Value |
|---|---|
| Source file | `src/main.rs` — 1408 lines, 53 KB |
| Dependencies | `serde`, `serde_json`, `sysinfo`, `unicode-width` |
| Tests | 13 unit tests, all pass |
| Binary purpose | Read JSON from stdin → emit 1–2 ANSI-formatted statusline rows |

### Identified Performance Bottlenecks

1. **`sysinfo` process scan** (`check_process_cmdline_for_yolo`, line 717–749)
   - Creates a new `sysinfo::System` and calls `refresh_processes_specifics()` **every invocation**.
   - This enumerates all OS processes to find YOLO flags — the single most expensive call in the binary.
   - The statusline runs on every prompt refresh, so this adds ~20–50ms of latency each time.

2. **Double JSON deserialization** (line 755–756)
   - The stdin input is parsed twice: once into `serde_json::Value` (for `detect_yolo_in_json`) and once into the typed `InputData` struct.
   - `serde_json::from_value(raw_val.clone())` also clones the entire JSON tree.

3. **Excessive `String` allocations in layout candidates** (lines 1102–1230)
   - All segment variants (wide/med/narrow) are built eagerly, even though only one layout is used.
   - Every `.clone()` call on ANSI-colored strings allocates a new heap string.
   - The layout candidate vectors allocate many intermediate `String`s through `join_with_dot` and `join_with_space`.

4. **`convert_color_value` char-by-char parsing** (line 130–172)
   - Collects all chars into a `Vec<char>` first, then iterates by index.
   - Uses `format!()` to build ANSI escape strings for each color — allocates per call.

5. **`get_icon_str` allocates on every call** (line 174–221)
   - Returns `String` (heap allocation) instead of `&str` or `Cow<str>`.
   - Called ~20 times per render cycle.

6. **`strip_ansi` / `visible_len` called multiple times on same strings** (lines 239–259)
   - `visible_len` strips ANSI and computes width — called on each layout candidate during the fitting loop.
   - No caching of visible lengths.

---

## Proposed Changes

### Phase 1 — Performance (High Impact)

#### 1.1 Eliminate `sysinfo` process scanning

**Problem**: `check_process_cmdline_for_yolo()` is the #1 bottleneck. It creates a `System`, enumerates processes, and inspects command lines.

**Solution**: Remove the `sysinfo` crate entirely. Replace with a lightweight parent-process-only check using the Windows API directly (`GetCurrentProcessId` → walk up via `PROCESSENTRY32` snapshots). Only inspect the direct parent chain, not all processes.

Alternatively (simpler): Use a **YOLO cache file**. Write `~/.gemini/antigravity-cli/.yolo_flag` when YOLO is first detected. Check the file timestamp and only re-scan if it is stale (>30 seconds old). This avoids the heavy scan on most invocations.

**Simplest approach**: Check for a `--dangerously-skip-permissions` argument in the raw JSON payload or `std::env::args()` first. If the CLI does not send it, use `std::env::var("AGY_YOLO")` as a lightweight signal. Only fall back to process scanning if no other signal is found, and cache the result.

```diff
 [dependencies]
 serde = { version = "1.0", features = ["derive"] }
 serde_json = "1.0"
-sysinfo = "0.30"
+windows-sys = { version = "0.59", features = ["Win32_System_Diagnostics_ToolHelp", "Win32_System_Threading", "Win32_Foundation"] }
 unicode-width = "0.2"
```

**New function** (replaces `check_process_cmdline_for_yolo`):
```rust
#[cfg(windows)]
fn check_parent_cmdline_for_yolo() -> bool {
    use windows_sys::Win32::System::Diagnostics::ToolHelp::*;
    use windows_sys::Win32::System::Threading::*;
    // Walk parent chain only (max 10 levels), read cmdline of node/agy processes
    // Much faster than sysinfo full-process refresh
}
```

> [!IMPORTANT]
> Removing `sysinfo` is the single biggest performance win. The `sysinfo` crate pulls in many system calls and is designed for monitoring dashboards, not sub-50ms CLI tools.

---

#### 1.2 Single-pass JSON deserialization

**Problem**: Input is parsed twice and the `Value` tree is cloned.

**Solution**: Parse once into `serde_json::Value`, run `detect_yolo_in_json` on that, then deserialize the typed `InputData` from the same `Value` **without cloning**.

```diff
-let raw_val: serde_json::Value = serde_json::from_str(&input).unwrap_or(serde_json::Value::Null);
-let data: InputData = serde_json::from_value(raw_val.clone()).unwrap_or_default();
+let raw_val: serde_json::Value = serde_json::from_str(&input).unwrap_or_default();
+let yolo_from_json = detect_yolo_in_json(&raw_val);
+let data: InputData = serde_json::from_value(raw_val).unwrap_or_default();
```

This eliminates the `.clone()` of the entire JSON tree. The `raw_val` is consumed by `from_value` after YOLO detection finishes. Update `render_statusline` to accept `yolo_from_json: bool` as a parameter instead of `raw_val: &serde_json::Value`.

> [!NOTE]
> This also means `render_statusline` no longer needs the `raw_val` parameter for the ASCII-mode detection on lines 830–832. Move those checks to `main()` before the value is consumed, or read them from `InputData` fields directly (they are already deserialized as `use_ascii` and `mode` fields).

---

#### 1.3 Lazy layout construction

**Problem**: All three density variants (wide/med/narrow) of every segment are built eagerly, plus multiple layout candidates are constructed and measured.

**Solution**: Build segments on demand. Use a `LayoutBuilder` struct that:
1. Computes segments at the currently-attempted density level only.
2. Measures visible length.
3. Falls back to the next density only if the current layout does not fit.

```rust
enum Density { Wide, Medium, Narrow }

struct LayoutBuilder<'a> {
    theme: &'a Theme,
    data: &'a InputData,
    target_cols: usize,
}

impl<'a> LayoutBuilder<'a> {
    fn build_line1(&self, density: Density) -> (String, String) { ... }
    fn build_line2(&self, density: Density) -> (String, String) { ... }
    fn try_fit(&self) -> Vec<String> {
        // Try single-line first, then two-line at decreasing densities
    }
}
```

This eliminates ~80% of intermediate `String` allocations in the layout loop.

---

### Phase 2 — Refactoring (Structure)

Split `src/main.rs` into focused modules. Recommended structure:

```
src/
├── main.rs          # Entry point: stdin read, config load, output
├── config.rs        # Config, ConfigColors structs, load_config()
├── data.rs          # InputData, ContextWindow, Model, Sandbox, QuotaEntry structs
├── theme.rs         # Theme struct (resolved colors + icons), convert_color_value()
├── icons.rs         # get_icon_str(), icon set selection logic
├── layout.rs        # LayoutBuilder, render_statusline(), line construction
├── segments.rs      # Individual segment formatters (state, model, quota, sandbox, etc.)
├── yolo.rs          # detect_yolo_in_json(), parent-process YOLO check
├── ansi.rs          # strip_ansi(), visible_len(), truncate_to_visible_width()
└── util.rs          # format_human(), format_seconds(), get_tokens(), path helpers
```

#### Module responsibilities

| Module | Contents | Lines (approx) |
|---|---|---|
| `main.rs` | `fn main()`, stdin read, config load, call `render_statusline`, print | ~30 |
| `config.rs` | `Config`, `ConfigColors` structs + `load_config()` | ~60 |
| `data.rs` | `InputData`, `ContextWindow`, `Model`, `Sandbox`, `QuotaEntry` | ~100 |
| `theme.rs` | `Theme` struct holding resolved ANSI color strings + icon strings | ~120 |
| `icons.rs` | `get_icon_str()`, icon-set selection, `IconSet` enum | ~80 |
| `layout.rs` | `LayoutBuilder`, responsive layout logic, `render_statusline()` | ~200 |
| `segments.rs` | `format_sandbox()`, `format_branch()`, `format_quota()`, `make_bar()`, state/model segment builders | ~200 |
| `yolo.rs` | `detect_yolo_in_json()`, `check_parent_cmdline_for_yolo()` | ~80 |
| `ansi.rs` | `strip_ansi()`, `visible_len()`, `truncate_to_visible_width()` | ~60 |
| `util.rs` | `format_human()`, `format_seconds()`, `get_tokens()`, `get_shortened_path()`, `join_with_dot()` | ~80 |

---

### Phase 3 — Micro-optimizations

#### 3.1 Use `Cow<'a, str>` for icon lookups

Change `get_icon_str` to return `Cow<'static, str>` to avoid heap allocation when the default string literal is used (which is the common case when no custom config overrides icons).

#### 3.2 Cache `visible_len` results

Wrap formatted segments in a `Segment` struct that caches the visible width:

```rust
struct Segment {
    text: String,
    visible_width: usize,
}

impl Segment {
    fn new(text: String) -> Self {
        let visible_width = visible_len(&text);
        Self { text, visible_width }
    }
}
```

#### 3.3 Replace `format!` in `make_bar` with direct string building

The current `make_bar` calls `format!()` inside a loop for each bar cell. Replace with `push_str` of pre-computed ANSI sequences.

#### 3.4 Use `write!` to stdout directly

Instead of collecting lines into a `Vec<String>` and printing with `println!`, use `BufWriter<Stdout>` and `write!` directly to avoid extra allocations and lock contention.

```rust
use std::io::{BufWriter, Write};

fn main() {
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    // ... render directly to `out`
}
```

---

## Verification Plan

### Automated Tests

```powershell
cargo test
```

All 13 existing tests must continue to pass. New tests should be added for:
- Each new module's public functions
- The `check_parent_cmdline_for_yolo()` replacement (mocked or integration)
- The lazy layout builder at various terminal widths

### Manual Verification

1. Build release binary: `cargo build --release`
2. Feed `last_statusline_input.json` to the binary and compare output before/after:
   ```powershell
   Get-Content last_statusline_input.json | .\target\release\agy-statusline.exe
   ```
3. Run install script and verify live statusline in Antigravity CLI
4. Test at terminal widths: 50, 75, 100, 135, 160, 200
5. Test with `USE_ASCII=true` and `USE_NERD_FONTS=false` environment variables

### Performance Verification

Compare execution time before and after (use `Measure-Command` in PowerShell):

```powershell
Measure-Command { Get-Content last_statusline_input.json | .\target\release\agy-statusline.exe }
```

Target: < 10ms per invocation (current: ~30–60ms due to `sysinfo`).

---

## Execution Order

1. **Phase 1.1** — Remove `sysinfo`, implement lightweight YOLO check ← biggest win
2. **Phase 1.2** — Single-pass JSON deserialization
3. **Phase 2** — Module split (can be done incrementally, one module at a time)
4. **Phase 1.3** — Lazy layout construction (easier after module split)
5. **Phase 3** — Micro-optimizations (last, as gains are smaller)

---

## Decisions

> [!NOTE]
> **YOLO detection strategy**: **Option 1 — Windows API parent-chain walk**. Add `windows-sys` dependency, walk only the parent process chain. Remove `sysinfo` crate entirely.

> [!NOTE]
> **Module granularity**: **8 modules**. Merge `icons.rs` into `theme.rs` and `util.rs` into `ansi.rs`. Final module list: `main.rs`, `config.rs`, `data.rs`, `theme.rs`, `layout.rs`, `segments.rs`, `yolo.rs`, `ansi.rs`.
