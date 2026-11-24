# Captures real terminal sessions of the simulator and renders them as PNGs.
# Usage:  powershell -ExecutionPolicy Bypass -File scripts/capture-screenshots.ps1
# Output: docs/screenshots/*.png (committed); raw transcripts stay in target/shots.
#
# Pipeline: scripted stdin -> real release binary (FORCE_COLOR) -> transcript
# -> crop window -> ANSI-to-HTML -> headless Chrome screenshot.
$ErrorActionPreference = "Stop"

# PowerShell 5.1 decodes native-command output with the OEM codepage by
# default, which mangles the simulator's UTF-8 box drawing. Force UTF-8 for
# the decoding direction. (Input stays ASCII: default pipe encoding, because
# Encoding.UTF8 would prepend a BOM the simulator cannot parse.)
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$repo = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $repo "target\release\mini-os-kernel.exe"
if (-not (Test-Path $exe)) {
    $exe = Join-Path $repo "target\debug\mini-os-kernel.exe"
}
if (-not (Test-Path $exe)) {
    throw "Build first: cargo build --release"
}

$chrome = @(
    "C:\Program Files\Google\Chrome\Application\chrome.exe",
    "C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    "C:\Program Files\Microsoft\Edge\Application\msedge.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $chrome) {
    throw "Chrome or Edge not found for screenshot rendering."
}

$inDir = Join-Path $repo "target\shots"
$outDir = Join-Path $repo "docs\screenshots"
$htmlDir = Join-Path $repo "target\shots-html"
New-Item -ItemType Directory -Force $inDir, $outDir, $htmlDir | Out-Null

$procs3 = @("1", "1", "P1", "0", "8", "1", "100", "1", "P2", "1", "4", "2", "80", "1", "P3", "2", "9", "1", "120")
$procs4 = $procs3 + @("1", "P4", "3", "5", "1", "60")

# Each entry: display title + scripted stdin (exactly what a user types) +
# optional crop window markers.
$shots = [ordered]@{
    "01-main-menu" = @{
        Title = "Mini OS Kernel Simulator - Main Menu"
        Input = @("9")
    }
    "02-process-management" = @{
        Title = "Process Management - process table and event log"
        Input = $procs3 + @("6", "8", "9", "9")
        Start = "PID\s+NAME"
        End = "Select an option: 9"
    }
    "03-srtf-scheduling" = @{
        Title = "SRTF scheduling - Gantt chart and metrics"
        Input = $procs4 + @("9", "2", "3", "7", "9")
        Start = "^\[9\] Scheduler SRTF"
        End = "── CPU SCHEDULING"
    }
    "04-algorithm-comparison" = @{
        Title = "Compare all five scheduling algorithms"
        Input = $procs4 + @("9", "2", "6", "", "7", "9")
        Start = "=== COMPARISON"
        End = "── CPU SCHEDULING"
    }
    "05-memory-map" = @{
        Title = "Contiguous memory - Best Fit allocation and freeing"
        Input = @(
            "1", "1", "P1", "0", "8", "1", "300",
            "1", "P2", "1", "4", "2", "200",
            "1", "P3", "2", "9", "1", "100",
            "1", "P4", "3", "5", "1", "0",
            "9", "3", "3", "4", "2", "1", "4", "150", "3", "2", "2", "3", "9", "9"
        )
        Start = "^PHYSICAL MEMORY"
        StartLast = $true
        End = "── MEMORY MANAGEMENT"
    }
    "06-paging-translation" = @{
        Title = "Paging - page tables, translation, page faults"
        Input = @(
            "1", "1", "P1", "0", "8", "1", "20",
            "9", "3", "5", "1", "20", "6", "1", "5", "2", "6", "1", "0", "2",
            "7", "8", "", "1 2 3 4 1 2 5 1 2 3 4 5", "9", "9"
        )
        Start = "page table created"
        End = "Select an option: 8"
    }
    "07-page-replacement" = @{
        Title = "Page replacement - FIFO vs LRU vs Optimal (Belady string)"
        Input = @(
            "1", "1", "P1", "0", "8", "1", "20",
            "9", "3", "5", "1", "20", "6", "1", "5", "2", "6", "1", "0", "2",
            "7", "8", "", "1 2 3 4 1 2 5 1 2 3 4 5", "9", "9"
        )
        Start = "^PAGING \(page size"
        End = "^Kernel simulator shutting down"
    }
    "08-synchronization" = @{
        Title = "Synchronization - races and producer-consumer"
        Input = @("4", "1", "2", "", "", "3", "", "", "", "", "5", "9")
        Start = "^Two processes each run"
        End = "Select an option: 5"
    }
    "09-ipc" = @{
        Title = "IPC - bounded queue, blocked sender, receive"
        Input = @(
            "1", "1", "P1", "0", "1", "1", "0",
            "1", "P2", "0", "1", "1", "0",
            "9",
            "5", "1", "2",
            "2", "1", "1", "2", "Hello from Process 1",
            "2", "1", "1", "2", "Second message",
            "2", "1", "1", "2", "Third message (queue full)",
            "4", "3", "1", "2", "4", "8", "9"
        )
        Start = "Third message"
        End = "Select an option: 8"
    }
    "10-deadlock-banker" = @{
        Title = "Deadlock - Banker's Algorithm (safe sequence, safe grant)"
        Input = @("6", "1", "5", "6", "1", "1 0 2", "2", "7", "8", "9")
        Start = "^Total     :"
        End = "Select an option: 2"
    }
    "11-deadlock-detection" = @{
        Title = "Deadlock - detection finds the deadlocked processes"
        Input = @("6", "1", "5", "6", "1", "1 0 2", "2", "7", "8", "9")
        Start = "^Total     :"
        StartLast = $true
        End = "^Kernel simulator shutting down"
    }
    "12-kernel-dashboard" = @{
        Title = "Kernel Dashboard"
        Input = @(
            "1", "1", "P1", "0", "8", "1", "120",
            "1", "P2", "2", "4", "2", "80",
            "5", "1", "9", "8", "9"
        )
        Start = "^║\s+KERNEL DASHBOARD"
        End = "^╚"
        EndInclusive = $true
    }
}

foreach ($name in $shots.Keys) {
    $shot = $shots[$name]
    $txt = Join-Path $inDir "$name.txt"
    # FORCE_COLOR makes the simulator emit ANSI codes even though stdout is a
    # pipe; the HTML renderer below paints them into the PNG.
    $env:FORCE_COLOR = "1"
    ($shot.Input -join "`n") | & $exe 2>&1 | Out-File -FilePath $txt -Encoding utf8
    Remove-Item Env:\FORCE_COLOR
    Write-Host "captured $name ($((Get-Content $txt).Count) lines)"
}

# ---------- render transcripts: ANSI -> HTML -> Chrome ----------
$esc = [string][char]27
$sgr = $esc + '\[([0-9;]*)m'

$fgClasses = @{
    "30" = "c30"; "31" = "c31"; "32" = "c32"; "33" = "c33"; "34" = "c34"
    "35" = "c35"; "36" = "c36"; "37" = "c37"; "90" = "c90"; "91" = "c91"
    "92" = "c92"; "93" = "c93"; "94" = "c94"; "95" = "c95"; "96" = "c96"
    "97" = "c97"
}
$bgClasses = @{
    "40" = "b40"; "41" = "b41"; "42" = "b42"; "43" = "b43"
    "44" = "b44"; "45" = "b45"; "46" = "b46"
}

function Convert-LineToHtml([string]$line) {
    $sb = New-Object System.Text.StringBuilder
    $classes = @()
    $pos = 0
    foreach ($m in [regex]::Matches($line, $script:sgr)) {
        $text = $line.Substring($pos, $m.Index - $pos)
        if ($text.Length -gt 0) {
            $encoded = $text.Replace('&', '&amp;').Replace('<', '&lt;').Replace('>', '&gt;')
            if ($classes.Count -gt 0) {
                [void]$sb.Append('<span class="' + ($classes -join ' ') + '">' + $encoded + '</span>')
            } else {
                [void]$sb.Append($encoded)
            }
        }
        $params = $m.Groups[1].Value
        $codes = if ($params -eq "") { @("0") } else { $params -split ";" }
        foreach ($code in $codes) {
            if ($code -eq "0") { $classes = @() }
            elseif ($code -eq "1") { if ($classes -notcontains "bold") { $classes += "bold" } }
            elseif ($code -eq "2") { if ($classes -notcontains "dim") { $classes += "dim" } }
            elseif ($script:fgClasses.ContainsKey($code)) {
                $classes = @($classes | Where-Object { -not $_.StartsWith("c") }) + $script:fgClasses[$code]
            }
            elseif ($script:bgClasses.ContainsKey($code)) {
                $classes = @($classes | Where-Object { -not $_.StartsWith("b") }) + $script:bgClasses[$code]
            }
        }
        $pos = $m.Index + $m.Length
    }
    $rest = $line.Substring($pos)
    if ($rest.Length -gt 0) {
        $encoded = $rest.Replace('&', '&amp;').Replace('<', '&lt;').Replace('>', '&gt;')
        if ($classes.Count -gt 0) {
            [void]$sb.Append('<span class="' + ($classes -join ' ') + '">' + $encoded + '</span>')
        } else {
            [void]$sb.Append($encoded)
        }
    }
    return $sb.ToString()
}

$css = @"
html, body { margin: 0; padding: 0; background: #0C0C0C; }
.title { background: #222222; color: #969696; font: bold 12px Consolas, monospace;
         padding: 7px 14px; }
pre { margin: 0; padding: 14px 16px; color: #D8D8D8;
      font: 14px/18px Consolas, "Cascadia Mono", "Courier New", monospace;
      white-space: pre; }
.bold { font-weight: bold; }
.dim { opacity: 0.62; }
.c30 { color: #0C0C0C; } .c31 { color: #E06C75; } .c32 { color: #98C379; }
.c33 { color: #E5C07B; } .c34 { color: #61AFEF; } .c35 { color: #C678DD; }
.c36 { color: #56B6C2; } .c37 { color: #D8D8D8; } .c90 { color: #7F7F7F; }
.c91 { color: #E06C75; } .c92 { color: #98C379; } .c93 { color: #E5C07B; }
.c94 { color: #61AFEF; } .c95 { color: #C678DD; } .c96 { color: #56B6C2; }
.c97 { color: #FFFFFF; }
.b40 { background: #333333; } .b41 { background: #7A3E48; }
.b42 { background: #3E6B46; } .b43 { background: #7A6A3E; }
.b44 { background: #3E5A7A; } .b45 { background: #6B3E7A; }
.b46 { background: #3E6B73; }
"@

foreach ($name in $shots.Keys) {
    $shot = $shots[$name]
    $txt = Join-Path $inDir "$name.txt"
    $lines = @(Get-Content $txt -Encoding UTF8 | ForEach-Object { $_.TrimEnd() })

    # Crop to the interesting window, if markers are defined. Match against the
    # ANSI-stripped text, since lines start with color escape sequences.
    $plain = @($lines | ForEach-Object { [regex]::Replace($_, $sgr, "") })
    if ($shot.Start) {
        $idx = @(for ($i = 0; $i -lt $plain.Count; $i++) { if ($plain[$i] -cmatch $shot.Start) { $i } })
        if ($idx.Count -gt 0) {
            $startIdx = if ($shot.StartLast) { $idx[-1] } else { $idx[0] }
            $lines = @($lines[$startIdx..($lines.Count - 1)])
        }
    }
    if ($shot.End) {
        $plain = @($lines | ForEach-Object { [regex]::Replace($_, $sgr, "") })
        $endIdx = -1
        for ($i = 2; $i -lt $plain.Count; $i++) {
            if ($plain[$i] -cmatch $shot.End) { $endIdx = $i; break }
        }
        if ($endIdx -gt 0) {
            $cut = if ($shot.EndInclusive) { $endIdx } else { $endIdx - 1 }
            $lines = @($lines[0..$cut])
        }
    }

    # Drop leading/trailing blank lines.
    while ($lines.Count -gt 0 -and $lines[0] -eq "") { $lines = @($lines | Select-Object -Skip 1) }
    while ($lines.Count -gt 0 -and $lines[-1] -eq "") { $lines = @($lines | Select-Object -SkipLast 1) }

    $stripped = $lines | ForEach-Object { [regex]::Replace($_, $sgr, "") }
    $cols = if (@($stripped).Count) { (@($stripped) | Measure-Object -Property Length -Maximum).Maximum } else { 60 }

    $body = ($lines | ForEach-Object { Convert-LineToHtml $_ }) -join "`n"
    $html = @"
<!doctype html>
<html><head><meta charset="utf-8"><style>$css</style></head>
<body>
<div class="title">$($shot.Title)</div>
<pre>$body</pre>
</body></html>
"@
    $htmlPath = Join-Path $htmlDir "$name.html"
    [System.IO.File]::WriteAllText($htmlPath, $html, (New-Object System.Text.UTF8Encoding($false)))

    # Consolas 14px advance is ~7.9px; add room for glyph bearings and padding.
    $width = [int][Math]::Min(1700, [Math]::Max(640, $cols * 8.2 + 60))
    $height = [int]($lines.Count * 18 + 28 + 60)
    $out = Join-Path $outDir "$name.png"
    $url = ([System.Uri]::new($htmlPath)).AbsoluteUri
    $prev = $ErrorActionPreference
    $ErrorActionPreference = "Continue" # chrome logs to stderr; not a failure
    & $chrome --headless=new --disable-gpu --disable-lcd-text --hide-scrollbars --no-first-run `
        --user-data-dir="$env:TEMP\mosk-shot-profile" `
        --window-size="$width,$height" --screenshot="$out" $url 2>&1 | Out-Null
    $ErrorActionPreference = $prev
    if (Test-Path $out) {
        Write-Host "rendered $out ($width x $height)"
    } else {
        Write-Host "FAILED to render $name"
    }
}

Write-Host "done."
