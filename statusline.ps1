#!/usr/bin/env pwsh
param(
    [Parameter(ValueFromPipeline)]
    [string]$inputJson
)

[System.Console]::OutputEncoding = [System.Text.Encoding]::UTF8

try {
    if ([string]::IsNullOrWhiteSpace($inputJson)) {
        $inputJson = [System.Console]::In.ReadToEnd()
    }
    if ([string]::IsNullOrWhiteSpace($inputJson)) {
        $inputJson = '{}'
    }

    try {
        $data = ConvertFrom-Json $inputJson -ErrorAction SilentlyContinue
    } catch {
        $data = @{}
    }

    function Write-Log ($message) {}

    $homePath = $env:USERPROFILE
    if ([string]::IsNullOrEmpty($homePath)) { $homePath = $env:HOME }

    $cliDir = Join-Path $homePath ".gemini\antigravity-cli"
    $CONFIG_PATH = Join-Path $cliDir "statusline_config.json"

    function Convert-ColorValue([string]$colorVal) {
        if ([string]::IsNullOrEmpty($colorVal)) { return $colorVal }
        return [regex]::Replace($colorVal, '#([0-9a-fA-F]{6}|[0-9a-fA-F]{3})', {
            param($match)
            $hex = $match.Groups[1].Value
            if ($hex.Length -eq 3) {
                $r = [Convert]::ToInt32("$($hex[0])$($hex[0])", 16)
                $g = [Convert]::ToInt32("$($hex[1])$($hex[1])", 16)
                $b = [Convert]::ToInt32("$($hex[2])$($hex[2])", 16)
            } else {
                $r = [Convert]::ToInt32($hex.Substring(0, 2), 16)
                $g = [Convert]::ToInt32($hex.Substring(2, 2), 16)
                $b = [Convert]::ToInt32($hex.Substring(4, 2), 16)
            }
            return "$([char]0x1b)[38;2;${r};${g};${b}m"
        })
    }

    if (Test-Path $CONFIG_PATH -PathType Leaf) {
        try {
            $cfg = Get-Content $CONFIG_PATH -Raw -Encoding utf8 | ConvertFrom-Json
            if ($cfg.colors) {
                if ($cfg.colors.reset) { $UI_RESET = Convert-ColorValue $cfg.colors.reset }
                if ($cfg.colors.bold) { $UI_BOLD = Convert-ColorValue $cfg.colors.bold }
                if ($cfg.colors.dim) { $UI_DIM = Convert-ColorValue $cfg.colors.dim }
                if ($cfg.colors.italic) { $UI_ITALIC = Convert-ColorValue $cfg.colors.italic }
                
                if ($cfg.colors.foreground) {
                    foreach ($prop in $cfg.colors.foreground.PSObject.Properties) {
                        $varName = "FG_" + $prop.Name.ToUpper()
                        $val = Convert-ColorValue $prop.Value
                        Set-Variable -Name $varName -Value $val -Scope Script -ErrorAction SilentlyContinue
                    }
                }
                if ($cfg.colors.ui) {
                    foreach ($prop in $cfg.colors.ui.PSObject.Properties) {
                        $varName = "UI_" + $prop.Name.ToUpper()
                        $val = Convert-ColorValue $prop.Value
                        Set-Variable -Name $varName -Value $val -Scope Script -ErrorAction SilentlyContinue
                    }
                }
            }
            if ($cfg.icons) {
                if ($cfg.icons.nerd_fonts) {
                    foreach ($cat in $cfg.icons.nerd_fonts.PSObject.Properties) {
                        if ($cat.Value.PSObject) {
                            foreach ($prop in $cat.Value.PSObject.Properties) {
                                $varName = "ICON_NF_" + $prop.Name.ToUpper()
                                Set-Variable -Name $varName -Value $prop.Value -Scope Script -ErrorAction SilentlyContinue
                            }
                        }
                    }
                }
                if ($cfg.icons.emoji_fallback) {
                    foreach ($cat in $cfg.icons.emoji_fallback.PSObject.Properties) {
                        if ($cat.Value.PSObject) {
                            foreach ($prop in $cat.Value.PSObject.Properties) {
                                $varName = "ICON_EMOJI_" + $prop.Name.ToUpper()
                                Set-Variable -Name $varName -Value $prop.Value -Scope Script -ErrorAction SilentlyContinue
                            }
                        }
                    }
                }
            }
        } catch {}
    }

    $R = if ($UI_RESET) { $UI_RESET } else { "$([char]0x1b)[0m" }
    $B = if ($UI_BOLD) { $UI_BOLD } else { "$([char]0x1b)[1m" }
    $D = if ($UI_DIM) { $UI_DIM } else { "$([char]0x1b)[2m" }
    $I = if ($UI_ITALIC) { $UI_ITALIC } else { "$([char]0x1b)[3m" }

    $FG_BLACK = if ($FG_BLACK) { $FG_BLACK } else { "$([char]0x1b)[30m" }
    $FG_RED = if ($FG_RED) { $FG_RED } else { "$([char]0x1b)[31m" }
    $FG_GREEN = if ($FG_GREEN) { $FG_GREEN } else { "$([char]0x1b)[32m" }
    $FG_YELLOW = if ($FG_YELLOW) { $FG_YELLOW } else { "$([char]0x1b)[33m" }
    $FG_BLUE = if ($FG_BLUE) { $FG_BLUE } else { "$([char]0x1b)[34m" }
    $FG_MAGENTA = if ($FG_MAGENTA) { $FG_MAGENTA } else { "$([char]0x1b)[35m" }
    $FG_CYAN = if ($FG_CYAN) { $FG_CYAN } else { "$([char]0x1b)[36m" }
    $FG_WHITE = if ($FG_WHITE) { $FG_WHITE } else { "$([char]0x1b)[37m" }

    $FG_GRAY = if ($FG_GRAY) { $FG_GRAY } else { "$([char]0x1b)[90m" }
    $FG_BRIGHT_RED = if ($FG_BRIGHT_RED) { $FG_BRIGHT_RED } else { "$([char]0x1b)[91m" }
    $FG_BRIGHT_GREEN = if ($FG_BRIGHT_GREEN) { $FG_BRIGHT_GREEN } else { "$([char]0x1b)[92m" }
    $FG_BRIGHT_YELLOW = if ($FG_BRIGHT_YELLOW) { $FG_BRIGHT_YELLOW } else { "$([char]0x1b)[93m" }
    $FG_BRIGHT_BLUE = if ($FG_BRIGHT_BLUE) { $FG_BRIGHT_BLUE } else { "$([char]0x1b)[94m" }
    $FG_BRIGHT_MAGENTA = if ($FG_BRIGHT_MAGENTA) { $FG_BRIGHT_MAGENTA } else { "$([char]0x1b)[95m" }
    $FG_BRIGHT_CYAN = if ($FG_BRIGHT_CYAN) { $FG_BRIGHT_CYAN } else { "$([char]0x1b)[96m" }
    $FG_BRIGHT_WHITE = if ($FG_BRIGHT_WHITE) { $FG_BRIGHT_WHITE } else { "$([char]0x1b)[97m" }

    $NUM_COLOR = "${FG_BRIGHT_WHITE}${B}"
    $DOT = if ($UI_SEPARATOR) { $UI_SEPARATOR } else { "${FG_GRAY} | ${R}" }

    $USE_NERD_FONTS = $true
    if ($data -and $data.nerd_fonts_supported -ne $null) {
        $USE_NERD_FONTS = [bool]$data.nerd_fonts_supported
    } elseif ($env:USE_NERD_FONTS -eq "false") {
        $USE_NERD_FONTS = $false
    }

    function Get-Char([long]$code) {
        if ($code -le 0xFFFF) { return [char]$code }
        return [char]::ConvertFromUtf32($code)
    }

    if ($USE_NERD_FONTS) {
        $ICON_READY = if ($ICON_NF_READY) { $ICON_NF_READY } else { Get-Char 0xF192 }
        $ICON_THINKING = if ($ICON_NF_THINKING) { $ICON_NF_THINKING } else { Get-Char 0xF07F7 }
        $ICON_WORKING = if ($ICON_NF_WORKING) { $ICON_NF_WORKING } else { Get-Char 0xF423 }
        $ICON_TOOL = if ($ICON_NF_TOOL) { $ICON_NF_TOOL } else { Get-Char 0xF425 }
        $ICON_UNKNOWN = if ($ICON_NF_UNKNOWN) { $ICON_NF_UNKNOWN } else { Get-Char 0xF252 }
        
        $ICON_FOLDER = if ($ICON_NF_FOLDER) { $ICON_NF_FOLDER } else { Get-Char 0xEA83 }
        $ICON_MODEL = if ($ICON_NF_MODEL) { $ICON_NF_MODEL } else { Get-Char 0xF400 }
        $ICON_BRANCH = if ($ICON_NF_BRANCH) { $ICON_NF_BRANCH } else { Get-Char 0xF418 }
        $ICON_CONV = if ($ICON_NF_CONVERSATION) { $ICON_NF_CONVERSATION } elseif ($ICON_NF_CONV) { $ICON_NF_CONV } else { Get-Char 0xF036A }
        $ICON_CTX = if ($ICON_NF_CONTEXT) { $ICON_NF_CONTEXT } elseif ($ICON_NF_CTX) { $ICON_NF_CTX } else { Get-Char 0xF134F }
        $ICON_TOK = if ($ICON_NF_TOKEN) { $ICON_NF_TOKEN } elseif ($ICON_NF_TOK) { $ICON_NF_TOK } else { Get-Char 0xE26B }
        $ICON_ART = if ($ICON_NF_ARTIFACT) { $ICON_NF_ARTIFACT } elseif ($ICON_NF_ART) { $ICON_NF_ART } else { Get-Char 0xF0F6 }
        $ICON_SUB = if ($ICON_NF_SUBAGENT) { $ICON_NF_SUBAGENT } elseif ($ICON_NF_SUB) { $ICON_NF_SUB } else { Get-Char 0xF167A }
        $ICON_BG = if ($ICON_NF_BACKGROUND_TASK) { $ICON_NF_BACKGROUND_TASK } elseif ($ICON_NF_BG) { $ICON_NF_BG } else { Get-Char 0xF0AE }
        
        $ICON_SB_NET = if ($ICON_NF_NET) { $ICON_NF_NET } else { Get-Char 0xF0499 }
        $ICON_SB_NONET = if ($ICON_NF_NO_NET) { $ICON_NF_NO_NET } else { Get-Char 0xF0D34 }
        $ICON_SB_OFF = if ($ICON_NF_OFF) { $ICON_NF_OFF } else { Get-Char 0xF099C }

        $ICON_CYCLE_ACCEPT = if ($ICON_NF_ACCEPT) { $ICON_NF_ACCEPT } else { Get-Char 0xF012C }
        $ICON_CYCLE_PLAN = if ($ICON_NF_PLAN) { $ICON_NF_PLAN } else { Get-Char 0xF0349 }

        $ICON_YOLO = if ($ICON_NF_YOLO) { $ICON_NF_YOLO } else { Get-Char 0xF06D }
    } else {
        $ICON_READY = if ($ICON_EMOJI_READY) { $ICON_EMOJI_READY } else { Get-Char 0x1F7E2 }
        $ICON_THINKING = if ($ICON_EMOJI_THINKING) { $ICON_EMOJI_THINKING } else { Get-Char 0x1F4AD }
        $ICON_WORKING = if ($ICON_EMOJI_WORKING) { $ICON_EMOJI_WORKING } else { Get-Char 0x2699 }
        $ICON_TOOL = if ($ICON_EMOJI_TOOL) { $ICON_EMOJI_TOOL } else { Get-Char 0x2692 }
        $ICON_UNKNOWN = if ($ICON_EMOJI_UNKNOWN) { $ICON_EMOJI_UNKNOWN } else { Get-Char 0x23F3 }
        
        $ICON_FOLDER = if ($ICON_EMOJI_FOLDER) { $ICON_EMOJI_FOLDER } else { Get-Char 0x1F4C1 }
        $ICON_MODEL = if ($ICON_EMOJI_MODEL) { $ICON_EMOJI_MODEL } else { Get-Char 0x1F4A1 }
        $ICON_BRANCH = if ($ICON_EMOJI_BRANCH) { $ICON_EMOJI_BRANCH } else { Get-Char 0x2387 }
        $ICON_CONV = if ($ICON_EMOJI_CONVERSATION) { $ICON_EMOJI_CONVERSATION } elseif ($ICON_EMOJI_CONV) { $ICON_EMOJI_CONV } else { Get-Char 0x1F4AC }
        $ICON_CTX = if ($ICON_EMOJI_CONTEXT) { $ICON_EMOJI_CONTEXT } elseif ($ICON_EMOJI_CTX) { $ICON_EMOJI_CTX } else { Get-Char 0x1F4CA }
        $ICON_TOK = if ($ICON_EMOJI_TOKEN) { $ICON_EMOJI_TOKEN } elseif ($ICON_EMOJI_TOK) { $ICON_EMOJI_TOK } else { Get-Char 0x1FA99 }
        $ICON_ART = if ($ICON_EMOJI_ARTIFACT) { $ICON_EMOJI_ARTIFACT } elseif ($ICON_EMOJI_ART) { $ICON_EMOJI_ART } else { Get-Char 0x1F4C4 }
        $ICON_SUB = if ($ICON_EMOJI_SUBAGENT) { $ICON_EMOJI_SUBAGENT } elseif ($ICON_EMOJI_SUB) { $ICON_EMOJI_SUB } else { Get-Char 0x1F916 }
        $ICON_BG = if ($ICON_EMOJI_BACKGROUND_TASK) { $ICON_EMOJI_BACKGROUND_TASK } elseif ($ICON_EMOJI_BG) { $ICON_EMOJI_BG } else { Get-Char 0x1F4CB }
        
        $ICON_SB_NET = if ($ICON_EMOJI_NET) { $ICON_EMOJI_NET } else { Get-Char 0x1F4E6 }
        $ICON_SB_NONET = if ($ICON_EMOJI_NO_NET) { $ICON_EMOJI_NO_NET } else { (Get-Char 0x1F4E6) + (Get-Char 0x1F512) }
        $ICON_SB_OFF = if ($ICON_EMOJI_OFF) { $ICON_EMOJI_OFF } else { Get-Char 0x1F6AB }

        $ICON_CYCLE_ACCEPT = if ($ICON_EMOJI_ACCEPT) { $ICON_EMOJI_ACCEPT } else { Get-Char 0x2705 }
        $ICON_CYCLE_PLAN = if ($ICON_EMOJI_PLAN) { $ICON_EMOJI_PLAN } else { Get-Char 0x1F50D }

        $ICON_YOLO = if ($ICON_EMOJI_YOLO) { $ICON_EMOJI_YOLO } else { Get-Char 0x26A0 }
    }

    $BLOCK_FULL = Get-Char 0x2588
    $BLOCK_DARK = Get-Char 0x2593
    $BLOCK_MED  = Get-Char 0x2592
    $BLOCK_LIGHT = Get-Char 0x2591
    $BOX_SLASH = Get-Char 0x2571

    $STATE = if ($data.agent_state) { $data.agent_state } else { "idle" }
    $USED_PCT = if ($data.context_window -and $data.context_window.used_percentage) { [double]$data.context_window.used_percentage } else { 0 }
    $SANDBOX = if ($data.sandbox -and $data.sandbox.enabled -ne $null) { [bool]$data.sandbox.enabled } else { $false }
    $SANDBOX_NET = if ($data.sandbox -and $data.sandbox.allow_network) { [bool]$data.sandbox.allow_network } else { $false }

    $ARTIFACTS = if ($data.artifact_count) { [int]$data.artifact_count } else { 0 }
    $BG_TASKS = if ($data.task_count) { [int]$data.task_count } else { 0 }
    $MODEL_ID = if ($data.model -and $data.model.id) { $data.model.id } else { "" }
    $MODEL_NAME = if ($data.model -and $data.model.display_name) { $data.model.display_name } else { "" }
    $MODEL_EFFORT = if ($data.model -and $data.model.effort) { $data.model.effort } elseif ($data.model -and $data.model.effort_level) { $data.model.effort_level } else { "" }
    $COLS = if ($data.terminal_width) { [int]$data.terminal_width } else { 80 }
    $CWD = if ($data.cwd) { $data.cwd } else { "" }
    $CONV_ID = if ($data.conversation_id) { $data.conversation_id } else { "" }
    $INPUT_TOKENS = if ($data.context_window -and $data.context_window.total_input_tokens) { [int]$data.context_window.total_input_tokens } else { 0 }
    $OUTPUT_TOKENS = if ($data.context_window -and $data.context_window.total_output_tokens) { [int]$data.context_window.total_output_tokens } else { 0 }
    $TXT_LIMIT = if ($data.context_window -and $data.context_window.context_window_size) { [int]$data.context_window.context_window_size } else { 0 }

    $CTX_USED = $INPUT_TOKENS + $OUTPUT_TOKENS

    $SUBAGENTS = 0
    if ($data.subagents) {
        if ($data.subagents -is [array]) {
            $SUBAGENTS = $data.subagents.Count
        } else {
            $SUBAGENTS = [int]$data.subagents
        }
    }

    $CYCLE_MODE = if ($data.cycle_mode) { $data.cycle_mode } else { "" }

    $MATCHED_QUOTAS = @()

    if ($data.quota) {
        $modelDisp = if ($MODEL_NAME) { $MODEL_NAME } elseif ($MODEL_ID) { $MODEL_ID } else { "" }
        
        function Get-Tokens ($str) {
            if ([string]::IsNullOrEmpty($str)) { return @() }
            return [regex]::Matches($str.ToLower(), "[a-z0-9]+") | ForEach-Object { $_.Value }
        }
        
        $modelTokens = Get-Tokens $modelDisp
        
        $quotaProps = $data.quota.PSObject.Properties
        if (-not $quotaProps) {
            $quotaProps = $data.quota.Keys | ForEach-Object { [PSCustomObject]@{ Name = $_; Value = $data.quota[$_] } }
        }
        
        $allEntries = @()
        foreach ($prop in $quotaProps) {
            $qVal = $prop.Value
            if ($qVal -and $qVal.remaining_fraction -ne $null) {
                $key = $prop.Name
                $kt = Get-Tokens $key
                
                $matches = 0
                foreach ($t in $kt) {
                    if ($modelTokens -contains $t) {
                        $matches++
                    } elseif (($t -eq "3p") -and ($modelTokens -match "(claude|gpt|opus|sonnet|haiku|o1|o3|deepseek)")) {
                        $matches++
                    }
                }
                
                $score = 0.0
                if ($matches -ge 1) {
                    $score = ($matches * 100) + ($matches / $kt.Count)
                    if ($key -like "*5h*" -or $key -like "*five*") {
                        $score += 10
                    }
                }
                
                $allEntries += [PSCustomObject]@{
                    Key = $key
                    Value = $qVal
                    Matches = $matches
                    Score = $score
                    RemainingFraction = [double]$qVal.remaining_fraction
                    ResetInSeconds = if ($qVal.reset_in_seconds -ne $null) { [int]$qVal.reset_in_seconds } else { 0 }
                }
            }
        }
        
        if ($allEntries.Count -gt 0) {
            $matched = $allEntries | Where-Object { $_.Score -gt 0 }
            if ($matched) {
                $MATCHED_QUOTAS = $matched | Sort-Object Score -Descending
            } else {
                $MATCHED_QUOTAS = @($allEntries | Sort-Object RemainingFraction | Select-Object -First 1)
            }
        }
    }

    $VCS_BRANCH = ""
    if (![string]::IsNullOrEmpty($CWD) -and (Test-Path (Join-Path $CWD ".git"))) {
        try {
            $branchObj = git -C "$CWD" branch --show-current 2>$null
            if ($LastExitCode -eq 0 -and $branchObj) {
                $VCS_BRANCH = ($branchObj | Out-String).Trim()
            }
        } catch {}
    }

    function Get-VisibleLength ($str) {
        if ([string]::IsNullOrEmpty($str)) { return 0 }
        $ansiPattern = "$([char]0x1b)\[[0-9;]*m"
        $clean = $str -replace $ansiPattern, ""
        return [System.Globalization.StringInfo]::new($clean).LengthInTextElements
    }

    function Get-ShortenedPath ($path, $maxLen) {
        if ([string]::IsNullOrEmpty($path)) { return "" }
        $shortPath = $path
        if (![string]::IsNullOrEmpty($homePath) -and $path.StartsWith($homePath)) {
            $shortPath = "~" + $path.Substring($homePath.Length)
        }
        if ($maxLen -eq 0) {
            if ($shortPath -eq "~") { return "~" } else { return Split-Path $path -Leaf }
        } elseif ($shortPath.Length -gt $maxLen) {
            return "..." + (Split-Path $path -Leaf)
        } else {
            return $shortPath
        }
    }

    function Format-Branch ($maxLen) {
        if ([string]::IsNullOrEmpty($VCS_BRANCH)) { return "" }
        $name = $VCS_BRANCH
        if ($maxLen -gt 0 -and $name.Length -gt $maxLen) {
            $name = $name.Substring(0, $maxLen) + ".."
        }
        return "${FG_BRIGHT_BLUE}${ICON_BRANCH} ${name}${R}"
    }

    function Format-Sandbox ($mode) {
        if ($SANDBOX) {
            $icon = if ($SANDBOX_NET) { $ICON_SB_NET } else { $ICON_SB_NONET }
            if ($mode -eq "wide") {
                $label = if ($SANDBOX_NET) { "ON (net)" } else { "ON (no-net)" }
                return "${FG_GREEN}${icon} ${FG_BRIGHT_GREEN}${B}${label}${R}"
            } elseif ($mode -eq "med") {
                return "${FG_GREEN}${icon} ${FG_BRIGHT_GREEN}${B}ON${R}"
            } else {
                return "${FG_GREEN}${icon}${R}"
            }
        } else {
            if ($mode -eq "wide" -or $mode -eq "med") {
                return "${FG_RED}${ICON_SB_OFF} ${FG_BRIGHT_RED}${B}OFF${R}"
            } else {
                return "${FG_RED}${ICON_SB_OFF}${R}"
            }
        }
    }

    function Format-Seconds ($s) {
        if ($s -le 0) { return "0s" }
        if ($s -ge 3600) {
            $h = [math]::Floor($s / 3600)
            $m = [math]::Floor(($s % 3600) / 60)
            return "${h}h${m}m"
        } elseif ($s -ge 60) {
            $m = [math]::Floor($s / 60)
            return "${m}m"
        } else {
            return "${s}s"
        }
    }

    function Format-Quota ($mode) {
        if (-not $MATCHED_QUOTAS -or $MATCHED_QUOTAS.Count -eq 0) { return "" }
        
        function Format-SingleQuota ($entry, $mode, $showIcon) {
            $pct = [int][math]::Round($entry.RemainingFraction * 100)
            $q_reset = Format-Seconds $entry.ResetInSeconds
            
            $clean_name = $entry.Key -replace "^gemini-", "" -replace "^3p-", ""
            if ($clean_name -eq "5h") { $clean_name = "5h" }
            elseif ($clean_name -eq "weekly") { $clean_name = "wk" }
            
            $q_color = if ($pct -le 10) { $FG_BRIGHT_RED } elseif ($pct -le 40) { $FG_BRIGHT_CYAN } else { $FG_CYAN }
            $icon_str = if ($showIcon) { "${FG_CYAN}${ICON_UNKNOWN}  ${R}" } else { "" }
            
            if ($mode -eq "narrow") {
                return "${icon_str}${NUM_COLOR}${pct}%${R} ${FG_GRAY}${clean_name}${R}"
            }
            
            if ($mode -eq "med") {
                return "${icon_str}${NUM_COLOR}${pct}%${R} ${FG_GRAY}${clean_name}${R} ${FG_GRAY}${q_reset}${R}"
            }
            
            $len = 5
            $filled = [math]::Floor($pct * $len / 100)
            $remainder = ($pct * $len) % 100
            
            $bar = ""
            for ($i = 0; $i -lt $len; $i++) {
                if ($i -lt $filled) {
                    $bar += "${q_color}${BLOCK_FULL}${R}"
                } elseif ($i -eq $filled) {
                    if ($remainder -ge 75) { $bar += "${q_color}${BLOCK_DARK}${R}${FG_GRAY}" }
                    elseif ($remainder -ge 50) { $bar += "${q_color}${BLOCK_MED}${R}${FG_GRAY}" }
                    else { $bar += "${q_color}${BLOCK_LIGHT}${R}${FG_GRAY}" }
                } else {
                    $bar += "${FG_GRAY}${BLOCK_LIGHT}${R}"
                }
            }
            return "${icon_str}${bar} ${NUM_COLOR}${pct}%${R} ${FG_GRAY}${clean_name}${R} ${FG_GRAY}${q_reset}${R}"
        }

        if ($mode -eq "narrow") {
            return Format-SingleQuota $MATCHED_QUOTAS[0] $mode $true
        }
        
        $formattedParts = @()
        $first = $true
        foreach ($entry in $MATCHED_QUOTAS) {
            $formattedParts += Format-SingleQuota $entry $mode $first
            $first = $false
        }
        
        return $formattedParts -join " "
    }

    $PCT_INT = [int]$USED_PCT
    $FILL_COLOR = if ($PCT_INT -ge 90) { $FG_BRIGHT_RED } elseif ($PCT_INT -ge 60) { $FG_BRIGHT_YELLOW } else { $FG_YELLOW }

    function Make-Bar ($len) {
        $filled = [math]::Floor($PCT_INT * $len / 100)
        $remainder = ($PCT_INT * $len) % 100
        
        $bar = ""
        for ($i = 0; $i -lt $len; $i++) {
            if ($i -lt $filled) {
                $bar += "${FILL_COLOR}${BLOCK_FULL}${R}"
            } elseif ($i -eq $filled) {
                if ($remainder -ge 75) { $bar += "${FILL_COLOR}${BLOCK_DARK}${R}${FG_GRAY}" }
                elseif ($remainder -ge 50) { $bar += "${FILL_COLOR}${BLOCK_MED}${R}${FG_GRAY}" }
                else { $bar += "${FILL_COLOR}${BLOCK_LIGHT}${R}${FG_GRAY}" }
            } else {
                $bar += "${FG_GRAY}${BLOCK_LIGHT}${R}"
            }
        }
        return $bar
    }

    function Join-WithDot {
        $items = @()
        foreach ($arg in $args) {
            if (![string]::IsNullOrEmpty($arg)) { $items += $arg }
        }
        return $items -join $DOT
    }

    function Join-WithSpace {
        $items = @()
        foreach ($arg in $args) {
            if (![string]::IsNullOrEmpty($arg)) { $items += $arg }
        }
        return $items -join "  "
    }

    function Get-HumanFormat ($num) {
        if ([string]::IsNullOrEmpty($num) -or $num -eq 0) { return "0" }
        try { $n = [int64]$num } catch { return $num }
        
        if ($n -ge 1000000) {
            $main = [math]::Floor($n / 1000000)
            $frac = [math]::Floor(($n % 1000000) / 100000)
            return "${main}.${frac}M"
        } elseif ($n -ge 1000) {
            $main = [math]::Floor($n / 1000)
            $frac = [math]::Floor(($n % 1000) / 100)
            return "${main}.${frac}K"
        } else {
            return "$n"
        }
    }

    $INPUT_TOK_FMT = Get-HumanFormat $INPUT_TOKENS
    $OUTPUT_TOK_FMT = Get-HumanFormat $OUTPUT_TOKENS
    $TXT_LIMIT_FMT = Get-HumanFormat $TXT_LIMIT
    $CTX_USED_FMT = Get-HumanFormat $CTX_USED

    $S = ""
    switch ($STATE) {
        "idle"     { $S = "${FG_BRIGHT_GREEN}${B}${ICON_READY} READY${R}" }
        "thinking" { $S = "${FG_BRIGHT_YELLOW}${B}${ICON_THINKING} THINKING${R}" }
        "working"  { $S = "${FG_BRIGHT_CYAN}${B}${ICON_WORKING} WORKING${R}" }
        "tool_use" { $S = "${FG_BRIGHT_MAGENTA}${B}${ICON_TOOL} TOOL${R}" }
        default    { $S = "${FG_WHITE}${B}${ICON_UNKNOWN} $($STATE.ToUpper())${R}" }
    }

    $CWD_WIDE_VAL = Get-ShortenedPath $CWD 25
    $DIR_WIDE = if (![string]::IsNullOrEmpty($CWD_WIDE_VAL)) { "${FG_CYAN}${ICON_FOLDER} ${R}${CWD_WIDE_VAL}${R}" } else { "" }

    $CWD_MED_VAL = Get-ShortenedPath $CWD 15
    $DIR_MED = if (![string]::IsNullOrEmpty($CWD_MED_VAL)) { "${FG_CYAN}${ICON_FOLDER} ${R}${CWD_MED_VAL}${R}" } else { "" }

    $CWD_NARROW_VAL = Get-ShortenedPath $CWD 0
    $DIR_NARROW = if (![string]::IsNullOrEmpty($CWD_NARROW_VAL)) { "${FG_CYAN}${ICON_FOLDER} ${R}${CWD_NARROW_VAL}${R}" } else { "" }

    if ([string]::IsNullOrEmpty($MODEL_EFFORT) -and $MODEL_NAME -match "\(([^)]+)\)") {
        $MODEL_EFFORT = $Matches[1]
    }

    $BASE_MODEL = if (![string]::IsNullOrEmpty($MODEL_ID)) { $MODEL_ID } else { $MODEL_NAME }
    $CLEAN_BASE = if (![string]::IsNullOrEmpty($BASE_MODEL)) { $BASE_MODEL -replace " \([^)]+\)", "" } else { "" }
    $MODEL_SHORT = if (![string]::IsNullOrEmpty($CLEAN_BASE)) { $CLEAN_BASE -replace "(?i)^gemini-", "" -replace "^Gemini ", "" } else { "" }

    if (![string]::IsNullOrEmpty($MODEL_EFFORT)) {
        $MODEL_WIDE_STR = "${CLEAN_BASE} (${MODEL_EFFORT})"
        $MODEL_MED_STR = "${MODEL_SHORT} (${MODEL_EFFORT})"
    } else {
        $MODEL_WIDE_STR = "${CLEAN_BASE}"
        $MODEL_MED_STR = "${MODEL_SHORT}"
    }

    $M_WIDE = if (![string]::IsNullOrEmpty($MODEL_WIDE_STR)) { "${FG_BRIGHT_MAGENTA}${I}${ICON_MODEL} ${MODEL_WIDE_STR}${R}" } else { "" }
    $M_MED = if (![string]::IsNullOrEmpty($MODEL_MED_STR)) { "${FG_BRIGHT_MAGENTA}${I}${ICON_MODEL} ${MODEL_MED_STR}${R}" } else { "" }
    $M_NARROW = if (![string]::IsNullOrEmpty($MODEL_MED_STR)) {
        $len = [math]::Min($MODEL_MED_STR.Length, 12)
        "${FG_BRIGHT_MAGENTA}${I}${ICON_MODEL} $($MODEL_MED_STR.Substring(0, $len))${R}"
    } else { "" }

    $V_WIDE = Format-Branch 15
    $V_MED = Format-Branch 10
    $V_NARROW = Format-Branch 6

    $CONV_WIDE = if (![string]::IsNullOrEmpty($CONV_ID)) { "${FG_GRAY}${ICON_CONV} $($CONV_ID.Substring(0, [math]::Min($CONV_ID.Length, 8)))${R}" } else { "" }
    $CONV_MED = if (![string]::IsNullOrEmpty($CONV_ID)) { "${FG_GRAY}${ICON_CONV} $($CONV_ID.Substring(0, [math]::Min($CONV_ID.Length, 4)))${R}" } else { "" }
    $CONV_NARROW = ""

    $SB_WIDE = Format-Sandbox "wide"
    $SB_MED = Format-Sandbox "med"
    $SB_NARROW = Format-Sandbox "narrow"

    $BAR_WIDE = Make-Bar 15
    $BAR_MED = Make-Bar 10
    $BAR_NARROW = Make-Bar 6

    $PCT_FMT = "{0:F1}" -f $USED_PCT

    $CTX_BAR_WIDE = "${FG_YELLOW}${ICON_CTX}  ${R}${BAR_WIDE} ${NUM_COLOR}${PCT_FMT}%${R}"
    $CTX_BAR_MED = "${FG_YELLOW}${ICON_CTX}  ${R}${BAR_MED} ${NUM_COLOR}${PCT_FMT}%${R}"
    $CTX_BAR_NARROW = "${FG_YELLOW}${ICON_CTX}  ${R}${BAR_NARROW} ${NUM_COLOR}$([int]$USED_PCT)%${R}"

    $TOK_DETAILS_WIDE = ""
    if ($CTX_USED -gt 0) {
        $TOK_DETAILS_WIDE = " (${CTX_USED_FMT}/${TXT_LIMIT_FMT})${DOT}${FG_YELLOW}${ICON_TOK} ${R} (${INPUT_TOK_FMT} in/${OUTPUT_TOK_FMT} out)"
    }

    $TOK_DETAILS_MED = ""
    if ($CTX_USED -gt 0) {
        $TOK_DETAILS_MED = " (${CTX_USED_FMT}/${TXT_LIMIT_FMT})"
    }

    $ART_WIDE = "${FG_BLUE}${ICON_ART} ${NUM_COLOR}${ARTIFACTS}${R}"
    $SUB_WIDE = "${FG_CYAN}${ICON_SUB} ${NUM_COLOR}${SUBAGENTS}${R}"
    $BG_WIDE = "${FG_MAGENTA}${ICON_BG} ${NUM_COLOR}${BG_TASKS}${R}"

    $ART_MED = "${FG_BLUE}${ICON_ART} ${NUM_COLOR}${ARTIFACTS}${R}"
    $SUB_MED = "${FG_CYAN}${ICON_SUB} ${NUM_COLOR}${SUBAGENTS}${R}"
    $BG_MED = "${FG_MAGENTA}${ICON_BG} ${NUM_COLOR}${BG_TASKS}${R}"

    $ART_NARROW = "${FG_BLUE}${ICON_ART}${NUM_COLOR}${ARTIFACTS}${R}"
    $SUB_NARROW = "${FG_CYAN}${ICON_SUB}${NUM_COLOR}${SUBAGENTS}${R}"
    $BG_NARROW = "${FG_MAGENTA}${ICON_BG}${NUM_COLOR}${BG_TASKS}${R}"

    $QUOTA_WIDE = Format-Quota "wide"
    $QUOTA_MED = Format-Quota "med"
    $QUOTA_NARROW = Format-Quota "narrow"

    $CYCLE_SEG = ""
    if ($CYCLE_MODE -eq "accept-edits") {
        $CYCLE_SEG = "${FG_BRIGHT_YELLOW}${B}${ICON_CYCLE_ACCEPT} ACCEPT-EDITS${R}"
    } elseif ($CYCLE_MODE -eq "plan") {
        $CYCLE_SEG = "${FG_BRIGHT_BLUE}${B}${ICON_CYCLE_PLAN} PLAN${R}"
    }

    $LINE1_WIDE = Join-WithDot $S $CYCLE_SEG $M_WIDE $DIR_WIDE $V_WIDE $CONV_WIDE
    $LINE2_WIDE = Join-WithDot $ART_WIDE $SUB_WIDE $BG_WIDE $SB_WIDE "${CTX_BAR_WIDE}${TOK_DETAILS_WIDE}" $QUOTA_WIDE

    $LINE1_MED = Join-WithDot $S $CYCLE_SEG $M_MED $DIR_MED $V_MED
    $LINE2_MED = Join-WithDot $ART_MED $SUB_MED $BG_MED $SB_MED "${CTX_BAR_MED}${TOK_DETAILS_MED}" $QUOTA_MED

    function Print-RightAligned ($left, $right, $totalCols) {
        $left_vis = Get-VisibleLength $left
        $right_vis = Get-VisibleLength $right
        
        $pad = $totalCols - $left_vis - $right_vis
        if ($pad -lt 1) { $pad = 1 }
        
        $spaces = " " * $pad
        return "${left}${spaces}${right}"
    }

    $MARGIN = 8

    $LEN1_WIDE = Get-VisibleLength $LINE1_WIDE
    $LEN2_WIDE = Get-VisibleLength $LINE2_WIDE

    $LEN1_MED = Get-VisibleLength $LINE1_MED
    $LEN2_MED = Get-VisibleLength $LINE2_MED

    $OUTPUT_LINES = @()

    if ($COLS -ge 135 -and $COLS -ge ($LEN1_WIDE + $LEN2_WIDE + $MARGIN)) {
        $OUTPUT_LINES += Print-RightAligned $LINE1_WIDE $LINE2_WIDE $COLS
    } elseif ($COLS -ge 100) {
        $R1_LEFT = Join-WithDot $S $CYCLE_SEG $M_WIDE
        $R1_RIGHT = Join-WithDot $ART_WIDE $SUB_WIDE $BG_WIDE $SB_WIDE
        $R2_LEFT = Join-WithDot $DIR_WIDE $V_WIDE $CONV_WIDE
        $R2_RIGHT = Join-WithDot "${CTX_BAR_WIDE}${TOK_DETAILS_WIDE}" $QUOTA_WIDE
        
        $OUTPUT_LINES += Print-RightAligned $R1_LEFT $R1_RIGHT $COLS
        $OUTPUT_LINES += Print-RightAligned $R2_LEFT $R2_RIGHT $COLS
    } elseif ($COLS -ge 75) {
        $R1_LEFT = Join-WithDot $S $CYCLE_SEG $M_MED
        $R1_RIGHT = Join-WithDot $ART_MED $SUB_MED $BG_MED $SB_MED
        $R2_LEFT = Join-WithDot $DIR_MED $V_MED $CONV_MED
        $R2_RIGHT = Join-WithDot "${CTX_BAR_MED}${TOK_DETAILS_MED}" $QUOTA_MED
        
        $OUTPUT_LINES += Print-RightAligned $R1_LEFT $R1_RIGHT $COLS
        $OUTPUT_LINES += Print-RightAligned $R2_LEFT $R2_RIGHT $COLS
    } elseif ($COLS -ge 50) {
        $R1_LEFT = Join-WithDot $S $CYCLE_SEG $M_NARROW
        $R1_RIGHT = Join-WithSpace $ART_NARROW $SUB_NARROW $BG_NARROW $SB_NARROW
        $R2_LEFT = Join-WithDot $DIR_NARROW $V_NARROW
        $R2_RIGHT = Join-WithDot "${CTX_BAR_NARROW}" $QUOTA_NARROW
        
        $OUTPUT_LINES += Print-RightAligned $R1_LEFT $R1_RIGHT $COLS
        $OUTPUT_LINES += Print-RightAligned $R2_LEFT $R2_RIGHT $COLS
    } else {
        $CYC_SHORT = ""
        if (![string]::IsNullOrEmpty($CYCLE_SEG)) {
            $CYC_SHORT = "${FG_GRAY} ${BOX_SLASH} ${CYCLE_SEG}"
        }
        $M_SHORT = ""
        if (![string]::IsNullOrEmpty($MODEL_SHORT)) {
            $len = [math]::Min($MODEL_SHORT.Length, 8)
            $M_SHORT = "${FG_GRAY} ${BOX_SLASH} ${FG_BRIGHT_MAGENTA}$($MODEL_SHORT.Substring(0, $len))${R}"
        }
        $OUTPUT_LINES += "${S}${CYC_SHORT}${M_SHORT}"
        $OUTPUT_LINES += "${CTX_BAR_NARROW}"
    }

    foreach ($line in $OUTPUT_LINES) {
        Write-Output $line
    }
} catch {
    Write-Output ""
}
