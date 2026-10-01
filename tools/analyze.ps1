param(
    [string]$GhidraHome = (Join-Path $PSScriptRoot '..\.tools\ghidra_11.4.2_PUBLIC'),
    [string]$GameBin = (Join-Path $PSScriptRoot '..\alice_202106\Alice1\bin'),
    [string[]]$Binaries = @('alice.exe', 'cgamex86.dll', 'fgamex86.dll')
)
$ErrorActionPreference = 'Stop'
$project = Join-Path $PSScriptRoot '..\private\ghidra'
$reports = Join-Path $PSScriptRoot '..\private\analysis'
New-Item -ItemType Directory -Force -Path $project, $reports | Out-Null
$headless = Join-Path $GhidraHome 'support\analyzeHeadless.bat'
if (-not (Test-Path -LiteralPath $headless)) { throw 'Runnable Ghidra release not found; run tools/setup_ghidra.py or supply -GhidraHome.' }
foreach ($binary in $Binaries) {
    $inputFile = Join-Path $GameBin $binary
    & $headless $project 'AliceResearch' -import $inputFile -overwrite -analysisTimeoutPerFile 180 -max-cpu 2 -scriptPath (Join-Path $PSScriptRoot 'ghidra') -postScript AliceInventory.java $reports -log (Join-Path $reports "$binary.log")
    if ($LASTEXITCODE -ne 0) { throw "Ghidra failed for $binary with exit code $LASTEXITCODE" }
}
