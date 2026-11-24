# Captures real terminal sessions of the simulator and renders them as PNGs.
# Usage:  powershell -ExecutionPolicy Bypass -File scripts/capture-screenshots.ps1
# Output: docs/screenshots/*.png (committed); raw transcripts stay in target/shots.
# Each shot may define Start/End regex markers to crop the transcript to the
# interesting window (a screenshot of a scrolled terminal, not a mock-up).
$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

$repo = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $repo "target\release\mini-os-kernel.exe"
if (-not (Test-Path $exe)) {
    $exe = Join-Path $repo "target\debug\mini-os-kernel.exe"
}
if (-not (Test-Path $exe)) {
    throw "Build first: cargo build --release"
}

$inDir = Join-Path $repo "target\shots"
$outDir = Join-Path $repo "docs\screenshots"
New-Item -ItemType Directory -Force $inDir, $outDir | Out-Null

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
        Start = "^PID\s+NAME"
        End = "^Select an option: 9"
    }
    "03-srtf-scheduling" = @{
        Title = "SRTF scheduling - Gantt chart and metrics"
        Input = $procs4 + @("9", "2", "3", "7", "9")
        Start = "^\[9\] Scheduler SRTF"
        End = "^--- CPU Scheduling ---"
    }
    "04-algorithm-comparison" = @{
        Title = "Compare all five scheduling algorithms"
        Input = $procs4 + @("9", "2", "6", "", "7", "9")
        Start = "^=== COMPARISON"
        End = "^--- CPU Scheduling ---"
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
        End = "^--- Memory Management ---"
    }
    "06-paging-translation" = @{
        Title = "Paging - page tables, translation, page faults"
        Input = @(
            "1", "1", "P1", "0", "8", "1", "20",
            "9", "3", "5", "1", "20", "6", "1", "5", "2", "6", "1", "0", "2",
            "7", "8", "", "1 2 3 4 1 2 5 1 2 3 4 5", "9", "9"
        )
        Start = "^P1: page table created"
        End = "^Select an option: 8"
    }
    "07-page-replacement" = @{
        Title = "Page replacement - FIFO vs LRU vs Optimal (Belady string)"
        Input = @(
            "1", "1", "P1", "0", "8", "1", "20",
            "9", "3", "5", "1", "20", "6", "1", "5", "2", "6", "1", "0", "2",
            "7", "8", "", "1 2 3 4 1 2 5 1 2 3 4 5", "9", "9"
        )
        Start = "^PAGING \(page size 4, 8 frames, 2 used\)"
        End = "^Kernel simulator shutting down"
    }
    "08-synchronization" = @{
        Title = "Synchronization - races and producer-consumer"
        Input = @("4", "1", "2", "", "", "3", "", "", "", "", "5", "9")
        Start = "^Two processes each run"
        End = "^Select an option: 5"
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
        Start = "^Message body: Third"
        End = "^Select an option: 8"
    }
    "10-deadlock-banker" = @{
        Title = "Deadlock - Banker's Algorithm (safe sequence, safe grant)"
        Input = @("6", "1", "5", "6", "1", "1 0 2", "2", "7", "8", "9")
        Start = "^Total     :"
        End = "^Select an option: 2"
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
        Start = "^============= KERNEL DASHBOARD"
        End = "^=+$"
        EndInclusive = $true
    }
}

foreach ($name in $shots.Keys) {
    $shot = $shots[$name]
    $txt = Join-Path $inDir "$name.txt"
    ($shot.Input -join "`n") | & $exe 2>&1 | Out-File -FilePath $txt -Encoding utf8
    Write-Host "captured $name ($((Get-Content $txt).Count) lines)"
}

# ---------- render transcripts into terminal-style PNGs ----------
$font = New-Object System.Drawing.Font("Consolas", 13, [System.Drawing.FontStyle]::Regular, [System.Drawing.GraphicsUnit]::Pixel)
$titleFont = New-Object System.Drawing.Font("Consolas", 12, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
$lineH = 18
$pad = 18
$titleH = 28

$probe = New-Object System.Drawing.Bitmap(10, 10)
$gProbe = [System.Drawing.Graphics]::FromImage($probe)
$charW = [double]$gProbe.MeasureString("M", $font).Width
$gProbe.Dispose(); $probe.Dispose()

$bg = [System.Drawing.Color]::FromArgb(12, 12, 12)
$titleBg = [System.Drawing.Color]::FromArgb(34, 34, 34)
$textBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(216, 216, 216))
$accentBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(106, 190, 106))
$errBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(230, 120, 120))
$mutedBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(150, 150, 150))

foreach ($name in $shots.Keys) {
    $shot = $shots[$name]
    $txt = Join-Path $inDir "$name.txt"
    $lines = @(Get-Content $txt | ForEach-Object { $_.TrimEnd() })

    # Crop to the interesting window, if markers are defined.
    if ($shot.Start) {
        $idx = @(for ($i = 0; $i -lt $lines.Count; $i++) { if ($lines[$i] -match $shot.Start) { $i } })
        if ($idx.Count -gt 0) {
            $startIdx = if ($shot.StartLast) { $idx[-1] } else { $idx[0] }
            $lines = @($lines[$startIdx..($lines.Count - 1)])
        }
    }
    if ($shot.End) {
        $endIdx = -1
        for ($i = 1; $i -lt $lines.Count; $i++) {
            if ($lines[$i] -match $shot.End) { $endIdx = $i; break }
        }
        if ($endIdx -gt 0) {
            $cut = if ($shot.EndInclusive) { $endIdx } else { $endIdx - 1 }
            $lines = @($lines[0..$cut])
        }
    }

    # Drop leading/trailing blank lines.
    while ($lines.Count -gt 0 -and $lines[0] -eq "") { $lines = @($lines | Select-Object -Skip 1) }
    while ($lines.Count -gt 0 -and $lines[-1] -eq "") { $lines = @($lines | Select-Object -SkipLast 1) }

    $cols = if ($lines.Count) { ($lines | Measure-Object -Property Length -Maximum).Maximum } else { 60 }
    $width = [int][Math]::Min(1500, [Math]::Max(560, $cols * $charW + 2 * $pad))
    $height = $lines.Count * $lineH + 2 * $pad + $titleH

    $bmp = New-Object System.Drawing.Bitmap($width, $height)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::ClearTypeGridFit
    $g.Clear($bg)

    $titleBar = New-Object System.Drawing.SolidBrush($titleBg)
    $g.FillRectangle($titleBar, 0, 0, $width, $titleH)
    $g.DrawString($shot.Title, $titleFont, $mutedBrush, [float]$pad, [float]7)

    $y = [float]($titleH + $pad)
    foreach ($line in $lines) {
        $brush = $textBrush
        if ($line -match '^\[|^={5,}|^----') { $brush = $accentBrush }
        elseif ($line -cmatch 'ERROR|DENIED|UNSAFE|DEADLOCK detected|queue full') { $brush = $errBrush }
        $g.DrawString($line, $font, $brush, [float]$pad, $y)
        $y += $lineH
    }
    $g.Dispose()
    $titleBar.Dispose()

    $out = Join-Path $outDir "$name.png"
    $bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    Write-Host "rendered $out"
}

Write-Host "done."
