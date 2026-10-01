# Native checks must run in the background desktop. Never uses player saves.
[CmdletBinding()]
param([Parameter(Mandatory = $true)][string]$Executable)
$ErrorActionPreference = 'Stop'
$clockRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $clockRoot
try {
    $Executable = (Resolve-Path -LiteralPath $Executable).Path
    New-Item -ItemType Directory -Force -Path private/clock | Out-Null
    $report = [ordered]@{
        executable = $Executable
        sha256 = (Get-FileHash -LiteralPath $Executable -Algorithm SHA256).Hash
        checks = @()
    }
    foreach ($check in @('clock-check', 'clock-render-check', 'clock-save-write', 'clock-save-read')) {
        $ErrorActionPreference = 'Continue'
        & $Executable "--$check" --no-audio *> "private/clock/$check.log"
        $code = $LASTEXITCODE
        $ErrorActionPreference = 'Stop'
        $report.checks += [ordered]@{ name = $check; exit_code = $code }
        $report | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 private/clock/report.json
        if ($code -ne 0) { throw "Clockwork regression failed: $check ($code). See private/clock/$check.log" }
        Write-Host "PASS $check"
    }
}
finally { Pop-Location }
