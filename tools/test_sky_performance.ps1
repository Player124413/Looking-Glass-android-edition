# Run on a background test desktop. Original artwork stays under private/.
[CmdletBinding()]
param([Parameter(Mandatory = $true)][string]$Executable)
$ErrorActionPreference = 'Stop'
$renderCheckRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $renderCheckRoot
try {
    $Executable = (Resolve-Path -LiteralPath $Executable).Path
    New-Item -ItemType Directory -Force -Path private/sky-performance | Out-Null
    $report = [ordered]@{
        started_utc = [DateTime]::UtcNow.ToString('o')
        executable = $Executable
        sha256 = (Get-FileHash -LiteralPath $Executable -Algorithm SHA256).Hash
        status = 'running'
        checks = @()
    }
    foreach ($check in @('sky-performance-check', 'fortress-render-check', 'render-fx-check', 'fidelity-corpus-check', 'level-swap-check', 'visibility-check')) {
        $log = Join-Path $renderCheckRoot "private/sky-performance/$check.log"
        $ErrorActionPreference = 'Continue'
        & $Executable "--$check" --no-audio *> $log
        $result = $LASTEXITCODE
        $ErrorActionPreference = 'Stop'
        $report.checks += [ordered]@{ name = $check; exit_code = $result; log = $log }
        $report.status = if ($result -eq 0) { 'running' } else { 'failed' }
        $report | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 private/sky-performance/run-summary.json
        if ($result -ne 0) { throw "Rendering check failed: $check ($result). See $log" }
        if ((Get-FileHash -LiteralPath $Executable -Algorithm SHA256).Hash -ne $report.sha256) { throw 'Executable changed during checks' }
        Write-Host "PASS $check"
    }
    $report.status = 'passed'
    $report | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 private/sky-performance/run-summary.json
}
finally { Pop-Location }
