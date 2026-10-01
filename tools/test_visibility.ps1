# Build and run the actor/cutscene regression suite without touching player saves.
[CmdletBinding()]
param(
    [string]$Executable,
    [string]$Data,
    [switch]$SkipBuild
)
$ErrorActionPreference = 'Stop'
$visibilityRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $visibilityRoot
try {
    if (-not $Executable) {
        $Executable = Join-Path $visibilityRoot 'private/visibility-test-target/release/looking-glass.exe'
    }
    if (-not $SkipBuild) {
        if ($PSBoundParameters.ContainsKey('Executable')) {
            throw 'Use -SkipBuild with an explicit -Executable, or omit both to build the current source.'
        }
        & cargo build --release --locked --target-dir private/visibility-test-target
        if ($LASTEXITCODE -ne 0) { throw "Visibility build failed ($LASTEXITCODE)." }
    }
    $Executable = (Resolve-Path -LiteralPath $Executable).Path
    New-Item -ItemType Directory -Force -Path private/visibility | Out-Null
    $visibilityReport = [ordered]@{
        started_utc = [DateTime]::UtcNow.ToString('o')
        executable = $Executable
        sha256 = (Get-FileHash -LiteralPath $Executable -Algorithm SHA256).Hash
        status = 'running'
        exit_code = $null
    }
    $visibilityReport | ConvertTo-Json | Set-Content -Encoding UTF8 private/visibility/run-summary.json
    $visibilityArguments = @('--visibility-check')
    if ($Data) { $visibilityArguments += @('--data', $Data) }
    # Windows PowerShell treats native stderr as ErrorRecords. Capture the
    # diagnostic and exit code instead of terminating before writing the log.
    $ErrorActionPreference = 'Continue'
    & $Executable @visibilityArguments 2>&1 | Tee-Object -FilePath private/visibility/run.log
    $visibilityExitCode = $LASTEXITCODE
    $ErrorActionPreference = 'Stop'
    $visibilityReport.status = if ($visibilityExitCode -eq 0) { 'passed' } else { 'failed' }
    $visibilityReport.exit_code = $visibilityExitCode
    $visibilityReport | ConvertTo-Json | Set-Content -Encoding UTF8 private/visibility/run-summary.json
    if ($visibilityExitCode -ne 0) { throw "Visibility regression failed ($visibilityExitCode). See private/visibility/run.log." }
    Write-Host 'Visibility checks passed. Report: private/visibility/village-report.json'
}
finally {
    Pop-Location
}
