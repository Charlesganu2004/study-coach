[CmdletBinding()]
param([string]$OutputDir = (Join-Path (Split-Path -Parent $PSScriptRoot) 'dist'))

$ErrorActionPreference = 'Stop'
$CoachRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$CoachOutput = [IO.Path]::GetFullPath($OutputDir)
$CoachCargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
$CoachCargoPath = if ($CoachCargoCommand) { $CoachCargoCommand.Source } else { Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe' }
if (-not (Test-Path -LiteralPath $CoachCargoPath)) { throw 'Cargo is required to build a package. Users of the package do not need Rust.' }
$CoachToolchain = @()
if ($env:OS -eq 'Windows_NT' -and -not (Get-Command link.exe -ErrorAction SilentlyContinue)) {
    $CoachRustup = Join-Path (Split-Path -Parent $CoachCargoPath) 'rustup.exe'
    if ((Test-Path -LiteralPath $CoachRustup) -and ((& $CoachRustup toolchain list) -match '^stable-x86_64-pc-windows-gnu')) {
        $CoachToolchain = @('+stable-x86_64-pc-windows-gnu')
    }
}
Push-Location -LiteralPath $CoachRoot
try {
    $CoachMetadataText = & $CoachCargoPath @CoachToolchain metadata --locked --offline --no-deps --format-version 1
    if ($LASTEXITCODE -ne 0) { throw 'Cargo metadata failed.' }
    $CoachMetadata = ($CoachMetadataText -join "`n") | ConvertFrom-Json
    $CoachPackage = @($CoachMetadata.packages | Where-Object { $_.name -eq 'study-coach' })[0]
    if (-not $CoachPackage -or $CoachPackage.version -notmatch '^\d+\.\d+\.\d+([-.][A-Za-z0-9.-]+)?$') {
        throw 'The application package metadata is missing or invalid.'
    }
    & $CoachCargoPath @CoachToolchain check --offline --locked --bin study-coach-app
    if ($LASTEXITCODE -ne 0) { throw 'Application static check failed.' }
    & $CoachCargoPath @CoachToolchain build --offline --locked --release --bin study-coach-app
    if ($LASTEXITCODE -ne 0) { throw 'Application release build failed.' }
    $CoachRustc = Join-Path (Split-Path -Parent $CoachCargoPath) 'rustc.exe'
    $CoachHostLine = & $CoachRustc @CoachToolchain -vV | Where-Object { $_ -like 'host: *' }
    if ($LASTEXITCODE -ne 0 -or $CoachHostLine -notmatch '^host: ([A-Za-z0-9_-]+)$') { throw 'Cannot identify the native Rust host target.' }
    $CoachTarget = $Matches[1]
    if ($CoachTarget -notlike '*-windows-*') { throw 'Use scripts/package.sh for a native Linux or macOS package.' }
    $CoachFolderName = "StudyCoach-$($CoachPackage.version)-$CoachTarget"
    [IO.Directory]::CreateDirectory($CoachOutput) | Out-Null
    $CoachStageParent = Join-Path $CoachOutput ('.stage-' + [Guid]::NewGuid().ToString('N'))
    $CoachStage = Join-Path $CoachStageParent $CoachFolderName
    [IO.Directory]::CreateDirectory($CoachStage) | Out-Null
    Copy-Item -LiteralPath (Join-Path $CoachMetadata.target_directory 'release\study-coach-app.exe') -Destination (Join-Path $CoachStage 'study-coach-app.exe')
    foreach ($CoachName in @('StartStudyCoach.cmd', 'install.ps1', 'uninstall.ps1', 'README.txt')) {
        Copy-Item -LiteralPath (Join-Path (Join-Path $CoachRoot 'packaging') $CoachName) -Destination (Join-Path $CoachStage $CoachName)
    }
    $CoachSums = foreach ($CoachFile in (Get-ChildItem -LiteralPath $CoachStage -File | Sort-Object Name)) {
        '{0}  {1}' -f (Get-FileHash -LiteralPath $CoachFile.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), $CoachFile.Name
    }
    [IO.File]::WriteAllLines((Join-Path $CoachStage 'SHA256SUMS'), [string[]]$CoachSums, (New-Object Text.UTF8Encoding($false)))
    $CoachArchive = Join-Path $CoachOutput ($CoachFolderName + '.zip')
    Compress-Archive -LiteralPath $CoachStage -DestinationPath $CoachArchive -CompressionLevel Optimal -Force
    $CoachArchiveHash = (Get-FileHash -LiteralPath $CoachArchive -Algorithm SHA256).Hash.ToLowerInvariant()
    [IO.File]::WriteAllText(($CoachArchive + '.sha256'), "$CoachArchiveHash  $([IO.Path]::GetFileName($CoachArchive))`n", (New-Object Text.UTF8Encoding($false)))
    Write-Output "Package: $CoachArchive"
    Write-Output "SHA256: $CoachArchiveHash"
    # Remove only this invocation's generated staging folder, after validating its boundary.
    $CoachResolvedStage = [IO.Path]::GetFullPath($CoachStageParent)
    if (-not $CoachResolvedStage.StartsWith($CoachOutput.TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase) -or
        [IO.Path]::GetFileName($CoachResolvedStage) -notmatch '^\.stage-[0-9a-f]{32}$') {
        throw 'The generated staging path failed its cleanup boundary check.'
    }
    Remove-Item -LiteralPath $CoachResolvedStage -Recurse
} finally {
    Pop-Location
}
