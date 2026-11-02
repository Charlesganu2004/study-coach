# Use an existing Rust toolchain. This script never installs software.
$ErrorActionPreference = 'Stop'
$CoachArguments = $args
$CoachRoot = Split-Path -Parent $PSScriptRoot
$CoachCargo = Get-Command cargo -ErrorAction SilentlyContinue
if ($CoachCargo) {
    $CoachCargoPath = $CoachCargo.Source
} else {
    $CoachCargoPath = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
}
if (-not (Test-Path -LiteralPath $CoachCargoPath)) {
    throw 'Rust Cargo is missing. Install Rust using the official instructions in README.md.'
}
$CoachToolchain = @()
if ($env:OS -eq 'Windows_NT' -and -not (Get-Command link.exe -ErrorAction SilentlyContinue)) {
    $CoachRustup = Join-Path (Split-Path -Parent $CoachCargoPath) 'rustup.exe'
    if ((Test-Path -LiteralPath $CoachRustup) -and ((& $CoachRustup toolchain list) -match '^stable-x86_64-pc-windows-gnu')) {
        $CoachToolchain = @('+stable-x86_64-pc-windows-gnu')
    }
}
Push-Location -LiteralPath $CoachRoot
try {
    & $CoachCargoPath @CoachToolchain run --locked --release -- @CoachArguments
    $CoachExitCode = $LASTEXITCODE
} finally {
    Pop-Location
}
exit $CoachExitCode
