[CmdletBinding()]
param([switch]$CheckOnly, [string]$OutDirectory = (Join-Path $PSScriptRoot '..\results\milestone2-retrieval-2026-11-08'))
$ErrorActionPreference = 'Stop'
$CoachRoot = Split-Path -Parent $PSScriptRoot
$CoachCargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
$CoachCargo = if ($CoachCargoCommand) { $CoachCargoCommand.Source } else { Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe' }
if (-not (Test-Path -LiteralPath $CoachCargo)) { throw 'Install Rust Cargo before running this development evaluation.' }
$CoachToolchain = @()
if ($env:OS -eq 'Windows_NT' -and -not (Get-Command link.exe -ErrorAction SilentlyContinue)) {
    $CoachRustup = Join-Path (Split-Path -Parent $CoachCargo) 'rustup.exe'
    if ((Test-Path -LiteralPath $CoachRustup) -and ((& $CoachRustup toolchain list) -match '^stable-x86_64-pc-windows-gnu')) { $CoachToolchain = @('+stable-x86_64-pc-windows-gnu') }
}
Push-Location -LiteralPath $CoachRoot
try {
    & $CoachCargo @CoachToolchain build --locked --release --bin study-coach-retrieval-eval
    if ($LASTEXITCODE -ne 0) { throw 'Retrieval evaluator build failed.' }
    $CoachBinaryName = if ($env:OS -eq 'Windows_NT') { 'study-coach-retrieval-eval.exe' } else { 'study-coach-retrieval-eval' }
    $CoachBinary = Join-Path $CoachRoot ('target/release/' + $CoachBinaryName)
    $CoachArguments = @('--out', [System.IO.Path]::GetFullPath($OutDirectory))
    if ($CheckOnly) { $CoachArguments += '--check-only' }
    & $CoachBinary @CoachArguments
    if ($LASTEXITCODE -ne 0) { throw 'Retrieval study stopped. Checkpoint remains saved; rerun unchanged after resolving the error.' }
} finally { Pop-Location }
