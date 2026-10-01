# Run only inside the background desktop. Saves/captures are isolated from play.
[CmdletBinding()]
param([Parameter(Mandatory = $true)][string]$Executable)
$ErrorActionPreference = 'Stop'
$magmaRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $magmaRoot
try {
    $Executable = (Resolve-Path -LiteralPath $Executable).Path
    New-Item -ItemType Directory -Force -Path private/magma | Out-Null
    $report = [ordered]@{
        executable = $Executable
        sha256 = (Get-FileHash -LiteralPath $Executable -Algorithm SHA256).Hash
        checks = @()
    }
    foreach ($check in @('magma-render-check', 'magma-save-write', 'magma-save-read')) {
        $ErrorActionPreference = 'Continue'
        & $Executable "--$check" --no-audio *> "private/magma/$check.log"
        $code = $LASTEXITCODE
        $ErrorActionPreference = 'Stop'
        $report.checks += [ordered]@{ name = $check; exit_code = $code }
        $report | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 private/magma/report.json
        if ($code -ne 0) { throw "Magma regression failed: $check ($code). See private/magma/$check.log" }
        Write-Host "PASS $check"
    }
}
finally { Pop-Location }
