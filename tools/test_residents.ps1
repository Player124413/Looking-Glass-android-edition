# Native checks must run in the background desktop. Never uses player saves.
[CmdletBinding()]
param([Parameter(Mandatory = $true)][string]$Executable)
$ErrorActionPreference = 'Stop'
$residentRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $residentRoot
try {
    $Executable = (Resolve-Path -LiteralPath $Executable).Path
    New-Item -ItemType Directory -Force -Path private/residents | Out-Null
    $report = [ordered]@{
        executable = $Executable
        sha256 = (Get-FileHash -LiteralPath $Executable -Algorithm SHA256).Hash
        checks = @()
    }
    foreach ($check in @('resident-check', 'resident-render-check', 'resident-save-write', 'resident-save-read')) {
        $ErrorActionPreference = 'Continue'
        & $Executable "--$check" --no-audio *> "private/residents/$check.log"
        $code = $LASTEXITCODE
        $ErrorActionPreference = 'Stop'
        $report.checks += [ordered]@{ name = $check; exit_code = $code }
        $report | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 private/residents/report.json
        if ($code -ne 0) { throw "Shared resident regression failed: $check ($code). See private/residents/$check.log" }
        Write-Host "PASS $check"
    }
}
finally { Pop-Location }
