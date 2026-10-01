# Run only inside the background desktop. Saves/captures are isolated from play.
[CmdletBinding()]
param([Parameter(Mandatory = $true)][string]$Executable)
$ErrorActionPreference = 'Stop'
$burrowRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $burrowRoot
try {
    $Executable = (Resolve-Path -LiteralPath $Executable).Path
    New-Item -ItemType Directory -Force -Path private/burrow | Out-Null
    $report = [ordered]@{
        executable = $Executable
        sha256 = (Get-FileHash -LiteralPath $Executable -Algorithm SHA256).Hash
        checks = @()
    }
    foreach ($check in @('burrow-render-check', 'burrow-save-write', 'burrow-save-read')) {
        $ErrorActionPreference = 'Continue'
        & $Executable "--$check" --no-audio *> "private/burrow/$check.log"
        $code = $LASTEXITCODE
        $ErrorActionPreference = 'Stop'
        $report.checks += [ordered]@{ name = $check; exit_code = $code }
        $report | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 private/burrow/report.json
        if ($code -ne 0) { throw "Insect regression failed: $check ($code). See private/burrow/$check.log" }
        Write-Host "PASS $check"
    }
}
finally { Pop-Location }
