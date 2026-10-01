# Run only inside the background desktop. Saves/captures are isolated from play.
[CmdletBinding()]
param([Parameter(Mandatory = $true)][string]$Executable)
$ErrorActionPreference = 'Stop'
$snarkRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $snarkRoot
try {
    $Executable = (Resolve-Path -LiteralPath $Executable).Path
    New-Item -ItemType Directory -Force -Path private/snarks | Out-Null
    $report = [ordered]@{
        executable = $Executable
        sha256 = (Get-FileHash -LiteralPath $Executable -Algorithm SHA256).Hash
        checks = @()
    }
    foreach ($check in @('snark-render-check', 'snark-save-write', 'snark-save-read')) {
        $ErrorActionPreference = 'Continue'
        & $Executable "--$check" --no-audio *> "private/snarks/$check.log"
        $code = $LASTEXITCODE
        $ErrorActionPreference = 'Stop'
        $report.checks += [ordered]@{ name = $check; exit_code = $code }
        $report | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 private/snarks/report.json
        if ($code -ne 0) { throw "Snark regression failed: $check ($code). See private/snarks/$check.log" }
        Write-Host "PASS $check"
    }
}
finally { Pop-Location }
