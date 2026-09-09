# Antigravity (`agy`) CLI Statusline Payload Reference

This document provides a comprehensive, permanent reference for all telemetry and state fields passed by the Google Antigravity CLI (`agy`) to custom statusline executables on standard input (`stdin`).

---

## 1. Overview & Data Flow

* **Protocol**: Single-line JSON object delivered via `stdin` on every statusline refresh event.
* **Frequency**: Triggered on state transitions, tool executions, turn completions, and terminal window resizes.
* **Deserialization Strategy**: Deserialized in single-pass Rust serde structs using `#[serde(rename_all = "camelCase")]` with snake_case aliases.

---

## 2. Complete Live Sample Payload

```json
{
  "cwd": "C:\\Users\\dev\\code\\my-project",
  "session_id": "00000000-0000-0000-0000-000000000000",
  "conversation_id": "00000000-0000-0000-0000-000000000000",
  "transcript_path": "C:\\Users\\dev\\.gemini\\antigravity\\brain\\00000000-0000-0000-0000-000000000000\\.system_generated\\logs\\transcript.jsonl",
  "model": {
    "id": "Gemini 3.7 Flash (Medium)",
    "display_name": "Gemini 3.7 Flash (Medium)",
    "effort": "medium"
  },
  "workspace": {
    "current_dir": "C:\\Users\\dev\\code\\my-project",
    "project_dir": "C:\\Users\\dev\\code\\my-project"
  },
  "version": "1.1.21",
  "context_window": {
    "total_input_tokens": 187425,
    "total_output_tokens": 74188,
    "context_window_size": 1048576,
    "used_percentage": 17.87424087524414,
    "remaining_percentage": 82.12575912475586,
    "current_usage": {
      "input_tokens": 4147,
      "output_tokens": 147,
      "cache_creation_input_tokens": 0,
      "cache_read_input_tokens": 197309
    }
  },
  "exceeds_200k_tokens": false,
  "product": "antigravity",
  "quota": {
    "3p-5h": {
      "remaining_fraction": 1.0,
      "reset_time": "2026-08-27T03:51:07Z",
      "reset_in_seconds": 17849
    },
    "3p-weekly": {
      "remaining_fraction": 0.48964626,
      "reset_time": "2026-08-28T20:43:45Z",
      "reset_in_seconds": 165007
    },
    "gemini-5h": {
      "remaining_fraction": 0.2872033,
      "reset_time": "2026-08-27T00:18:43Z",
      "reset_in_seconds": 5105
    },
    "gemini-weekly": {
      "remaining_fraction": 0.3601719,
      "reset_time": "2026-08-27T02:41:14Z",
      "reset_in_seconds": 13656
    }
  },
  "agent_state": "working",
  "vcs": {
    "type": "git"
  },
  "sandbox": {
    "enabled": false
  },
  "artifact_count": 2,
  "plan_tier": "<subscription tier>",
  "email": "user@example.com",
  "terminal_width": 156
}
```

---

## 3. Schema & Field Definitions

### 3.1. `context_window` Object

| Field | Type | Description |
|---|---|---|
| `context_window_size` | `u64` | Total context memory limit of the active model in tokens (e.g. `1,048,576` for Gemini $1\text{ MiB}$, `250,000` for Claude Sonnet). |
| `total_input_tokens` | `u64` | Total active context tokens loaded into the model for the current prompt (system prompt + past turns + cache). |
| `total_output_tokens` | `u64` | Cumulative generation tokens across all session turns. **Do NOT add this to `total_input_tokens` for context capacity calculations.** |
| `used_percentage` | `f64` | Pre-calculated exact percentage of context filled: `(total_input_tokens / context_window_size) * 100.0`. |
| `remaining_percentage` | `f64` | Inverse percentage: `100.0 - used_percentage`. |
| `current_usage` | `object` | Granular token metrics for the **most recent API call**. |

### 3.2. `context_window.current_usage` Object

| Field | Type | Description |
|---|---|---|
| `input_tokens` | `u64` | Fresh uncached input tokens sent on the last turn (e.g. `4,147`). |
| `output_tokens` | `u64` | Generated response tokens on the last turn (e.g. `147`). |
| `cache_read_input_tokens` | `u64` | Tokens served from the prompt cache on the last turn (e.g. `197,309`). |
| `cache_creation_input_tokens` | `u64` | Tokens newly written to the prompt cache on the last turn. |

### 3.3. `quota` Object (Map of Quota Windows)

Keys correspond to model families and time windows (e.g., `gemini-5h`, `gemini-weekly`, `3p-5h`, `3p-weekly`):

| Field | Type | Description |
|---|---|---|
| `remaining_fraction` | `f64` | Quota remaining fraction between `0.0` and `1.0` (e.g. `0.2872` $\rightarrow$ `29%`). |
| `reset_in_seconds` | `u64` | Duration in seconds until quota resets (e.g. `5105` $\rightarrow$ `1h25m`). |
| `reset_time` | `string` | ISO 8601 UTC timestamp of quota reset. |

### 3.4. Session & Workspace Metadata

| Field | Type | Description |
|---|---|---|
| `agent_state` | `string` | Agent lifecycle state: `"idle"`, `"ready"`, `"thinking"`, `"working"`, `"tool_use"`. |
| `model.id` | `string` | Model identifier string (e.g. `"Gemini 3.7 Flash (Medium)"`, `"claude-3-7-sonnet"`). |
| `model.display_name` | `string` | Formatted model display name. |
| `model.effort` | `string` | Model reasoning / thinking effort level (e.g. `"low"`, `"medium"`, `"high"`). |
| `cwd` / `workspace.current_dir` | `string` | Active working directory path. |
| `workspace.project_dir` | `string` | Root project workspace directory path. |
| `conversation_id` / `session_id` | `string` | UUID of the active conversation. |
| `transcript_path` | `string` | Absolute path to the conversation `transcript.jsonl`. |
| `artifact_count` | `u64` | Number of artifacts created in this conversation. |
| `subagents` | `number \| array` | Active/total subagents spawned in this conversation. |
| `task_count` / `tasks` | `number \| array` | Background tasks running in this conversation. |
| `sandbox.enabled` | `bool` | Whether environment sandbox isolation is enabled (`false` is standard on Windows). |
| `sandbox.allow_network` | `bool` | Whether sandbox network access is allowed. |
| `vcs.type` | `string` | Version control system (e.g. `"git"`). |
| `terminal_width` | `usize` | Live terminal column width reported by the terminal emulator. |
| `plan_tier` | `string` | Account subscription tier (e.g. `"Google AI Pro"`, `"Google AI Ultra"`). |
| `email` | `string` | User account email address. |
| `exceeds_200k_tokens` | `bool` | High-context warning threshold flag. |
| `cycle_mode` | `string` | Active execution cycle mode (e.g. `"accept-edits"`, `"plan"`). |

---

## 4. Architectural Rules & Math Formulas

### 4.1. Context Window Calculation

* **Active Context Memory**: Equal to `total_input_tokens`.
* **Formula**:
  $$\text{used\_percentage} = \frac{\text{total\_input\_tokens}}{\text{context\_window\_size}} \times 100.0$$
* **Gemini Binary Limit**: Gemini models specify capacity in binary units ($1\text{ MiB} = 1{,}048{,}576$ tokens). Dividing $187{,}425 / 1{,}048{,}576$ yields $17.87\%$.

### 4.2. Token Throughput vs Context Capacity

* **Context Capacity Ratio**: Displayed on Line 1 as `(total_input_tokens / context_window_size)` $\rightarrow$ `(187.4K/1.0M)`.
* **Turn Breakdown**: Displayed on Line 2 as `(input_tokens in / output_tokens out / cache_read_input_tokens cache)` $\rightarrow$ `(4.1K in/147 out/197.3K cache)`.
* **Why Output Tokens Are Not Added**: Previous turn assistant responses are converted into input prompt tokens for subsequent turns. Adding `total_output_tokens` causes a double count.

### 4.3. Vertical Column Alignment

* Line 1 left-prefix (`YOLO | READY | Model`) and Line 2 left-prefix (`Dir | Branch | SessionID`) are padded with spaces to $\max(\text{len}_1, \text{len}_2)$ before appending the ` | ` delimiter.
* This ensures that the Line 1 Context Bar (`󱍏 ███░░...`) and Line 2 Token Metrics (`󰞋 (4.1K in...)`) start at the identical horizontal column.
