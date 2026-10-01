# Run native renderer checks on one exact executable; never touches player saves.
[CmdletBinding()]
param([Parameter(Mandatory = $true)][string]$Executable)
$ErrorActionPreference = 'Stop'
$renderRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $renderRoot
try {
    $Executable = (Resolve-Path -LiteralPath $Executable).Path
    New-Item -ItemType Directory -Force -Path private/render-fx | Out-Null
    $report = [ordered]@{
        started_utc = [DateTime]::UtcNow.ToString('o')
        executable = $Executable
        sha256 = (Get-FileHash -LiteralPath $Executable -Algorithm SHA256).Hash
        status = 'running'
        checks = @()
    }
    foreach ($check in @('billboard-check', 'render-fx-check', 'fidelity-corpus-check', 'visibility-check')) {
        $log = Join-Path $renderRoot "private/render-fx/$check.log"
        $ErrorActionPreference = 'Continue'
        & $Executable "--$check" --no-audio *> $log
        $result = $LASTEXITCODE
        $ErrorActionPreference = 'Stop'
        $report.checks += [ordered]@{ name = $check; exit_code = $result; log = $log }
        $report.status = if ($result -eq 0) { 'running' } else { 'failed' }
        $report | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 private/render-fx/run-summary.json
        if ($result -ne 0) { throw "Renderer check failed: $check ($result). See $log" }
        Write-Host "PASS $check"
    }
    $report.status = 'passed'
    $report | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 private/render-fx/run-summary.json
}
finally { Pop-Location }
