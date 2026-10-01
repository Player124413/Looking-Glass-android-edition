# Run the campaign chain (docs/CAMPAIGN.md): headless on the host, the native save chain only inside
# the Anode seat. Nothing here touches player saves or settings; every output stays under
# private/campaign-chain and private/campaign-save-chain.
#
#   Headless (any shell, never opens a window):
#     --campaign-graph-check, --campaign-route-check --campaign-strict (Normal or -Difficulty),
#     the same with --campaign-skip-cinematics, and with -Determinism a repeat run whose
#     per-leg state digests must equal the first run's (the script first removes the checkpoints
#     of earlier runs from private/campaign-chain, so the directory holds only this run's chain).
#     -Expect pins the frontier of those
#     chain runs (--campaign-expect), so a chain that is known to stop honestly exits 0 while
#     the pinned frontier holds and exits 1 the day it moves.
#   Visit 4 and 6 gates and pinned failures (-Pinned, Normal only): a chain gated on visit 4 (Beyond
#     the Wall) and one gated on visit 6 (school one, with its Croquet Mallet grant) at Normal, then
#     the same gates at Easy and at Hard in scratch working directories, all of which pass and none
#     of which is pinned (each exits 1 the day its visit stops passing), then the
#     checks whose documented outcome is a failure, each run with --campaign-expect so that it
#     exits 0 exactly while that outcome holds (the table $pins below is the one place to move
#     when a driver change moves the frontier): the strict chain resumed from visit 4, which stops
#     at school two, a --campaign-allow-retry diagnosis from visit 4, and a strict chain from
#     visit 6 with no checkpoint. The diagnosis and the refusal run in scratch working
#     directories, so a diagnosis never writes next to the proof checkpoints and the refusal
#     cannot find one.
#   Native (opens a window, so only with -InsideAnodeSeat, which the recipe-13c agent passes
#   through seat_exec and nobody passes on the host): --campaign-save-chain-write, then
#   --campaign-save-chain-read as a separate process.
#
# The executable's SHA-256 and every exit code go to private/campaign-chain/run-summary.json.
[CmdletBinding()]
param(
    [string]$Executable,
    [string]$Data,
    [string]$Difficulty = 'normal',
    [string]$To,
    [string]$Expect,
    [switch]$Pinned,
    [switch]$SkipBuild,
    [switch]$Determinism,
    [switch]$InsideAnodeSeat
)
$ErrorActionPreference = 'Stop'
$chainRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $chainRoot
try {
    if (-not $Executable) {
        $Executable = Join-Path $chainRoot 'private/campaign-chain-target/release/looking-glass.exe'
    }
    if (-not $SkipBuild) {
        if ($PSBoundParameters.ContainsKey('Executable')) {
            throw 'Use -SkipBuild with an explicit -Executable, or omit both to build the current source.'
        }
        & cargo build --release --locked --target-dir private/campaign-chain-target
        if ($LASTEXITCODE -ne 0) { throw "Campaign chain build failed ($LASTEXITCODE)." }
    }
    $Executable = (Resolve-Path -LiteralPath $Executable).Path
    # Scratch working directories cannot see private/data-path.txt: always pass the data folder.
    if (-not $Data) {
        $dataFile = Join-Path $chainRoot 'private/data-path.txt'
        if (Test-Path -LiteralPath $dataFile) { $Data = (Get-Content -Raw -LiteralPath $dataFile).Trim() }
    }
    if ($Data) { $Data = (Resolve-Path -LiteralPath $Data).Path }
    if ($Pinned -and $Difficulty -ne 'normal') { throw '-Pinned records Normal-difficulty outcomes: run it without -Difficulty.' }
    $dir = Join-Path $chainRoot 'private/campaign-chain'
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    # The directory is this run's output: a checkpoint that an earlier run left beyond this chain's
    # frontier would be read by the native save chain as if this chain had made it.
    Get-ChildItem -Path (Join-Path $dir '*') -Include '*.json' |
        Where-Object { $_.Name -match '^\d\d-.*\.json$' } |
        Remove-Item -Force
    $summaryPath = Join-Path $dir 'run-summary.json'
    $summary = [ordered]@{
        started_utc = [DateTime]::UtcNow.ToString('o')
        executable = $Executable
        sha256 = (Get-FileHash -LiteralPath $Executable -Algorithm SHA256).Hash
        data = $Data
        difficulty = $Difficulty
        inside_anode_seat = [bool]$InsideAnodeSeat
        status = 'running'
        headless = @()
        native = @()
        native_skipped = $null
    }
    function Save-Summary { $summary | ConvertTo-Json -Depth 6 | Set-Content -Encoding UTF8 $summaryPath }
    Save-Summary

    # Windows PowerShell treats native stderr as ErrorRecords: capture the log and the exit code
    # instead of terminating before the summary is written. A step with a $WorkDir runs there, so
    # its private/campaign-chain is that directory's own.
    function Invoke-Step([string]$Name, [string[]]$StepArguments, [bool]$Native, [string]$WorkDir) {
        $log = Join-Path $dir "$Name.log"
        $all = @($StepArguments)
        if ($Data) { $all += @('--data', $Data) }
        $script:ErrorActionPreference = 'Continue'
        if ($WorkDir) {
            New-Item -ItemType Directory -Force -Path $WorkDir | Out-Null
            Push-Location -LiteralPath $WorkDir
        }
        # Windows PowerShell writes UTF-16 for `*>`; keep the logs plain text (UTF-8).
        & $Executable @all *>&1 | ForEach-Object { $_.ToString() } | Out-File -FilePath $log -Encoding utf8
        $code = $LASTEXITCODE
        if ($WorkDir) { Pop-Location }
        $script:ErrorActionPreference = 'Stop'
        $entry = [ordered]@{ name = $Name; args = ($all -join ' '); exit_code = $code; log = $log }
        if ($WorkDir) { $entry['work_dir'] = $WorkDir }
        if ($Native) { $summary.native += $entry } else { $summary.headless += $entry }
        Save-Summary
        if ((Get-FileHash -LiteralPath $Executable -Algorithm SHA256).Hash -ne $summary.sha256) {
            throw 'The executable changed during the run.'
        }
        Write-Host ("{0} {1} (exit {2})" -f $(if ($code -eq 0) { 'PASS' } else { 'FAIL' }), $Name, $code)
        return $code
    }

    $chain = @('--campaign-route-check', '--campaign-strict', '--difficulty', $Difficulty, '--campaign-log', (Join-Path $dir 'route-check.log'))
    if ($To) { $chain += @('--campaign-to', $To) }
    if ($Expect) { $chain += @('--campaign-expect', $Expect) }
    $failed = @()
    if ((Invoke-Step 'graph-check' @('--campaign-graph-check') $false) -ne 0) { $failed += 'graph-check' }
    if ((Invoke-Step 'route-check' $chain $false) -ne 0) { $failed += 'route-check' }
    Copy-Item -LiteralPath (Join-Path $dir 'report.json') -Destination (Join-Path $dir "report-$Difficulty.json") -Force -ErrorAction SilentlyContinue
    $skip = @('--campaign-route-check', '--campaign-strict', '--campaign-skip-cinematics', '--difficulty', $Difficulty, '--campaign-log', (Join-Path $dir 'route-check-skip.log'))
    if ($To) { $skip += @('--campaign-to', $To) }
    if ($Expect) { $skip += @('--campaign-expect', $Expect) }
    if ((Invoke-Step 'route-check-skip' $skip $false) -ne 0) { $failed += 'route-check-skip' }
    Copy-Item -LiteralPath (Join-Path $dir 'report.json') -Destination (Join-Path $dir "report-$Difficulty-skip.json") -Force -ErrorAction SilentlyContinue

    if ($Determinism) {
        # The same chain again: every leg must end in the same state.
        Copy-Item -LiteralPath (Join-Path $dir "report-$Difficulty.json") -Destination (Join-Path $dir 'report-first.json') -Force
        $again = @('--campaign-route-check', '--campaign-strict', '--difficulty', $Difficulty, '--campaign-log', (Join-Path $dir 'route-check-repeat.log'))
        if ($To) { $again += @('--campaign-to', $To) }
        if ($Expect) { $again += @('--campaign-expect', $Expect) }
        if ((Invoke-Step 'route-check-repeat' $again $false) -ne 0) { $failed += 'route-check-repeat' }
        $first = Get-Content -Raw (Join-Path $dir 'report-first.json') | ConvertFrom-Json
        $second = Get-Content -Raw (Join-Path $dir 'report.json') | ConvertFrom-Json
        $digests = { param($r) ($r.legs | ForEach-Object { '{0}:{1}:{2}' -f $_.index, $_.entrance_digest, $_.exit_digest }) -join ';' }
        $equal = (& $digests $first) -eq (& $digests $second)
        $summary['determinism'] = [ordered]@{ equal = $equal; legs = @($first.legs).Count }
        Save-Summary
        Write-Host ("{0} determinism: {1} legs, digests {2}" -f $(if ($equal) { 'PASS' } else { 'FAIL' }), @($first.legs).Count, $(if ($equal) { 'equal' } else { 'DIFFER' }))
        if (-not $equal) { $failed += 'determinism' }
    }

    if ($Pinned) {
        # Beyond the Wall and school one pass chained on what the visits before leave Alice with, at
        # every difficulty, so the chains gated on visit 4 and on visit 6 are plain gates: each exits 0
        # only while its visit passes.
        $gates = @(
            @{ n = 4; to = 'fortress2$fortress2_start1' },
            @{ n = 6; to = 'skool1$skool1_start1' }
        )
        foreach ($g in $gates) {
            $n = $g.n
            $gate = @('--campaign-route-check', '--campaign-strict', '--campaign-to', $g.to,
                '--campaign-log', (Join-Path $dir "gate-visit-$n.log"))
            if ((Invoke-Step "gate-visit-$n" $gate $false) -ne 0) { $failed += "gate-visit-$n" }
            Copy-Item -LiteralPath (Join-Path $dir 'report.json') -Destination (Join-Path $dir "report-gate-visit-$n.json") -Force -ErrorAction SilentlyContinue
        }

        # The same gates at Easy and at Hard. Each starts from New Game state in its own scratch working
        # directory, so its checkpoints (written at its own difficulty) never replace the Normal proof
        # checkpoints above, which the pinned runs below resume from.
        foreach ($g in $gates) {
            $n = $g.n
            foreach ($other in @('easy', 'hard')) {
                $scratchGate = Join-Path $dir "scratch-gate-$n-$other"
                if (Test-Path -LiteralPath $scratchGate) { Remove-Item -LiteralPath $scratchGate -Recurse -Force }
                $gateOther = @('--campaign-route-check', '--campaign-strict', '--difficulty', $other,
                    '--campaign-to', $g.to, '--campaign-log', (Join-Path $dir "gate-visit-$n-$other.log"))
                if ((Invoke-Step "gate-visit-$n-$other" $gateOther $false $scratchGate) -ne 0) { $failed += "gate-visit-$n-$other" }
                Copy-Item -LiteralPath (Join-Path $scratchGate 'private/campaign-chain/report.json') -Destination (Join-Path $dir "report-gate-visit-$n-$other.json") -Force -ErrorAction SilentlyContinue
            }
        }

        # The documented failures, pinned. Each command exits 0 only while its outcome holds; move a
        # pin (and docs/CAMPAIGN.md) in the same change that moves the frontier.
        $pins = [ordered]@{
            # The strict chain passes Beyond the Wall, the fortress return and school one (with
            # its Croquet Mallet), and stops at school two: its recorded route dies in the first
            # fights on the Sanity and Will it is handed.
            frontier7 = 'skool2$skool2_start1'
            # The same, as a diagnosis that may refill once (Beyond the Wall, the fortress return
            # and school one need no refill): school two is then driven again from the refill and
            # stops on its missing required Demon Dice grant.
            retry4 = 'skool2$skool2_start1'
        }
        $onward = @('--campaign-route-check', '--campaign-strict', '--campaign-from', '4',
            '--campaign-expect', "frontier=$($pins.frontier7)", '--campaign-log', (Join-Path $dir 'pinned-frontier-from-4.log'))
        if ((Invoke-Step 'pinned-frontier-from-4' $onward $false) -ne 0) { $failed += 'pinned-frontier-from-4' }
        Copy-Item -LiteralPath (Join-Path $dir 'report.json') -Destination (Join-Path $dir 'report-pinned-frontier-from-4.json') -Force -ErrorAction SilentlyContinue

        # A diagnosis run in its own working directory, resumed from the visit 4 checkpoint just written.
        $scratch = Join-Path $dir 'scratch-retry'
        if (Test-Path -LiteralPath $scratch) { Remove-Item -LiteralPath $scratch -Recurse -Force }
        $checkpoints = Join-Path $scratch 'private/campaign-chain'
        New-Item -ItemType Directory -Force -Path $checkpoints | Out-Null
        Copy-Item -LiteralPath (Join-Path $dir '04-fortress2-first.json') -Destination $checkpoints
        $retry = @('--campaign-route-check', '--campaign-strict', '--campaign-from', '4', '--campaign-allow-retry',
            '--campaign-expect', "frontier=$($pins.retry4)", '--campaign-log', (Join-Path $dir 'pinned-retry-from-4.log'))
        if ((Invoke-Step 'pinned-retry-from-4' $retry $false $scratch) -ne 0) { $failed += 'pinned-retry-from-4' }

        # A strict chain never starts from a chapter loadout: with no checkpoint it refuses.
        $empty = Join-Path $dir 'scratch-refusal'
        if (Test-Path -LiteralPath $empty) { Remove-Item -LiteralPath $empty -Recurse -Force }
        $refuse = @('--campaign-route-check', '--campaign-strict', '--campaign-from', '6',
            '--campaign-expect', 'no-checkpoint', '--campaign-log', (Join-Path $dir 'pinned-no-checkpoint.log'))
        if ((Invoke-Step 'pinned-no-checkpoint' $refuse $false $empty) -ne 0) { $failed += 'pinned-no-checkpoint' }
    }

    if ($InsideAnodeSeat) {
        # Two separate processes: the reader must be a fresh process.
        if ((Invoke-Step 'save-chain-write' @('--campaign-save-chain-write', '--no-audio') $true) -ne 0) { $failed += 'save-chain-write' }
        if ((Invoke-Step 'save-chain-read' @('--campaign-save-chain-read', '--no-audio') $true) -ne 0) { $failed += 'save-chain-read' }
    }
    else {
        $summary.native_skipped = 'not invoked inside the Anode seat (-InsideAnodeSeat): --campaign-save-chain-write/read open a window and were not run'
    }
    $summary.status = if ($failed.Count -eq 0) { 'passed' } else { 'failed' }
    $summary['failed'] = $failed
    Save-Summary
    if ($failed.Count -gt 0) { throw ("Campaign chain steps failed: {0}. See private/campaign-chain/run-summary.json." -f ($failed -join ', ')) }
    Write-Host 'Campaign chain steps passed. Summary: private/campaign-chain/run-summary.json'
}
finally {
    Pop-Location
}
