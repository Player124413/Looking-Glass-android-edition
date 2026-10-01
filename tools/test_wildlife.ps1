# Rendering checks must run inside the background desktop; user saves are not used.
[CmdletBinding()]
param([Parameter(Mandatory = $true)][string]$Executable)
$ErrorActionPreference = 'Stop'
$creatureRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $creatureRoot
try {
    $Executable = (Resolve-Path -LiteralPath $Executable).Path
    New-Item -ItemType Directory -Force -Path private/wildlife | Out-Null
    $report = [ordered]@{
        executable = $Executable
        sha256 = (Get-FileHash -LiteralPath $Executable -Algorithm SHA256).Hash
        checks = @()
    }
    foreach ($check in @('wildlife-check', 'wildlife-save-write', 'wildlife-save-read', 'wildlife-render-check', 'render-fx-check')) {
        $ErrorActionPreference = 'Continue'
        & $Executable "--$check" --no-audio *> "private/wildlife/$check.log"
        $code = $LASTEXITCODE
        $ErrorActionPreference = 'Stop'
        $report.checks += [ordered]@{ name = $check; exit_code = $code }
        $report | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 private/wildlife/report.json
        if ($code -ne 0) { throw "Creature regression failed: $check ($code). See private/wildlife/$check.log" }
        Write-Host "PASS $check"
    }
}
finally { Pop-Location }
