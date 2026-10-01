# Native checks must run in the background desktop. Never uses player saves.
[CmdletBinding()]
param([Parameter(Mandatory = $true)][string]$Executable)
$ErrorActionPreference = 'Stop'
$impRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $impRoot
try {
    $Executable = (Resolve-Path -LiteralPath $Executable).Path
    New-Item -ItemType Directory -Force -Path private/imp | Out-Null
    $report = [ordered]@{
        executable = $Executable
        sha256 = (Get-FileHash -LiteralPath $Executable -Algorithm SHA256).Hash
        checks = @()
    }
    foreach ($check in @('imp-check', 'imp-render-check', 'imp-save-write', 'imp-save-read')) {
        $ErrorActionPreference = 'Continue'
        & $Executable "--$check" --no-audio *> "private/imp/$check.log"
        $code = $LASTEXITCODE
        $ErrorActionPreference = 'Stop'
        $report.checks += [ordered]@{ name = $check; exit_code = $code }
        $report | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 private/imp/report.json
        if ($code -ne 0) { throw "Fire Imp regression failed: $check ($code). See private/imp/$check.log" }
        Write-Host "PASS $check"
    }
}
finally { Pop-Location }
