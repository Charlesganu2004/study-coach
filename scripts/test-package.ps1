[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$ArchivePath,
    [string]$ReceiptPath = (Join-Path (Split-Path -Parent $PSScriptRoot) 'docs\PACKAGE_VERIFICATION.json')
)

$ErrorActionPreference = 'Stop'
$CoachArchive = [IO.Path]::GetFullPath($ArchivePath)
if (-not (Test-Path -LiteralPath $CoachArchive -PathType Leaf)) { throw 'Build the package archive before running this check.' }
$CoachTestRoot = Join-Path ([IO.Path]::GetTempPath()) ('study-coach-package-test-' + [Guid]::NewGuid().ToString('N'))
$CoachExtract = Join-Path $CoachTestRoot 'extracted'
$CoachInstall = Join-Path $CoachTestRoot 'installed'
$CoachData = Join-Path $CoachTestRoot 'data'
$CoachProcess = $null
$CoachChecks = [Collections.Generic.List[string]]::new()

function Assert-CoachPackage {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

function Get-CoachImportedLibraries {
    param([string]$BinaryPath)
    $CoachBytes = [IO.File]::ReadAllBytes($BinaryPath)
    $CoachPe = [BitConverter]::ToInt32($CoachBytes, 60)
    $CoachSectionCount = [BitConverter]::ToUInt16($CoachBytes, $CoachPe + 6)
    $CoachOptionalLength = [BitConverter]::ToUInt16($CoachBytes, $CoachPe + 20)
    $CoachOptional = $CoachPe + 24
    Assert-CoachPackage ([BitConverter]::ToUInt16($CoachBytes, $CoachOptional) -eq 523) 'Expected a PE32+ application.'
    $CoachSections = for ($CoachIndex = 0; $CoachIndex -lt $CoachSectionCount; $CoachIndex++) {
        $CoachOffset = $CoachOptional + $CoachOptionalLength + 40 * $CoachIndex
        [pscustomobject]@{
            Start = [BitConverter]::ToUInt32($CoachBytes, $CoachOffset + 12)
            Length = [Math]::Max([BitConverter]::ToUInt32($CoachBytes, $CoachOffset + 8), [BitConverter]::ToUInt32($CoachBytes, $CoachOffset + 16))
            Raw = [BitConverter]::ToUInt32($CoachBytes, $CoachOffset + 20)
        }
    }
    function Convert-CoachRva([uint32]$Rva) {
        foreach ($CoachSection in $CoachSections) {
            if ($Rva -ge $CoachSection.Start -and $Rva -lt $CoachSection.Start + $CoachSection.Length) {
                return [int]($Rva - $CoachSection.Start + $CoachSection.Raw)
            }
        }
        throw 'The binary contains an import outside its sections.'
    }
    $CoachImportOffset = Convert-CoachRva ([BitConverter]::ToUInt32($CoachBytes, $CoachOptional + 120))
    for ($CoachIndex = 0; $CoachIndex -lt 256; $CoachIndex++) {
        $CoachNameRva = [BitConverter]::ToUInt32($CoachBytes, $CoachImportOffset + 20 * $CoachIndex + 12)
        if ($CoachNameRva -eq 0) { break }
        $CoachNameOffset = Convert-CoachRva $CoachNameRva
        $CoachEnd = $CoachNameOffset
        while ($CoachBytes[$CoachEnd] -ne 0) { $CoachEnd++ }
        [Text.Encoding]::ASCII.GetString($CoachBytes, $CoachNameOffset, $CoachEnd - $CoachNameOffset)
    }
}

try {
    [IO.Directory]::CreateDirectory($CoachExtract) | Out-Null
    Expand-Archive -LiteralPath $CoachArchive -DestinationPath $CoachExtract
    $CoachFolders = @(Get-ChildItem -LiteralPath $CoachExtract -Directory)
    Assert-CoachPackage ($CoachFolders.Count -eq 1) 'The package must contain one application folder.'
    $CoachSource = $CoachFolders[0].FullName
    $CoachExpected = @('README.txt', 'SHA256SUMS', 'StartStudyCoach.cmd', 'install.ps1', 'study-coach-app.exe', 'uninstall.ps1') | Sort-Object
    $CoachActual = @(Get-ChildItem -LiteralPath $CoachSource -Force | Select-Object -ExpandProperty Name | Sort-Object)
    Assert-CoachPackage (($CoachActual -join '|') -eq ($CoachExpected -join '|')) 'The archive contains unexpected or missing files.'
    Assert-CoachPackage (@(Get-ChildItem -LiteralPath $CoachSource -Recurse -Force | Where-Object { $_.Name -like '.env*' }).Count -eq 0) 'A credential file entered the archive.'
    $CoachChecks.Add('Archive contains exactly six expected program files and no credential files.')
    $CoachDependencies = @(Get-CoachImportedLibraries (Join-Path $CoachSource 'study-coach-app.exe') | Sort-Object -Unique)
    Assert-CoachPackage ($CoachDependencies.Count -gt 0) 'The binary import check found no runtime libraries.'
    foreach ($CoachDependency in $CoachDependencies) {
        Assert-CoachPackage ($CoachDependency -like 'api-ms-win-*' -or (Test-Path -LiteralPath (Join-Path $env:SystemRoot ('System32\' + $CoachDependency)))) "A runtime dependency is not a Windows system library: $CoachDependency"
    }
    $CoachChecks.Add('The compiled binary imports Windows system libraries only; no Rust or MinGW runtime DLL is required.')

    & (Join-Path $CoachSource 'install.ps1') -InstallDir $CoachInstall -NoShortcut
    Assert-CoachPackage (Test-Path -LiteralPath (Join-Path $CoachInstall 'installation.json')) 'Installation marker missing.'
    Assert-CoachPackage ((Get-FileHash -LiteralPath (Join-Path $CoachInstall 'study-coach-app.exe')).Hash -eq (Get-FileHash -LiteralPath (Join-Path $CoachSource 'study-coach-app.exe')).Hash) 'Installed binary differs from the archive.'
    $CoachMarker = Get-Content -Raw -LiteralPath (Join-Path $CoachInstall 'installation.json') | ConvertFrom-Json
    Assert-CoachPackage ($null -eq $CoachMarker.shortcut) 'Isolated installation unexpectedly created a shortcut.'
    $CoachChecks.Add('Isolated installation succeeded without a Start menu shortcut; binary hash matches.')

    $CoachStart = New-Object Diagnostics.ProcessStartInfo
    $CoachStart.FileName = Join-Path $CoachInstall 'study-coach-app.exe'
    $CoachStart.Arguments = '--data-dir "' + $CoachData + '" --no-open --port 0'
    $CoachStart.UseShellExecute = $false
    $CoachStart.CreateNoWindow = $true
    $CoachStart.WindowStyle = [Diagnostics.ProcessWindowStyle]::Hidden
    $CoachStart.RedirectStandardOutput = $true
    $CoachStart.RedirectStandardError = $true
    $CoachStart.EnvironmentVariables.Remove('GROQ_API_KEY')
    $CoachStart.EnvironmentVariables.Remove('NVIDIA_API_KEY')
    $CoachProcess = [Diagnostics.Process]::Start($CoachStart)
    $CoachOutputLine = $CoachProcess.StandardOutput.ReadLineAsync()
    $CoachUrl = $null
    for ($CoachAttempt = 0; $CoachAttempt -lt 100; $CoachAttempt++) {
        if ($CoachOutputLine.IsCompleted) {
            if ($CoachOutputLine.Result -match 'http://127\.0\.0\.1:\d+') { $CoachUrl = $Matches[0]; break }
            $CoachOutputLine = $CoachProcess.StandardOutput.ReadLineAsync()
        }
        if ($CoachProcess.HasExited) { throw 'The installed application exited before opening its local server.' }
        Start-Sleep -Milliseconds 100
    }
    Assert-CoachPackage ($null -ne $CoachUrl) 'The installed application did not report a local server URL.'
    $CoachState = Invoke-RestMethod -Uri ($CoachUrl + '/api/state') -Method Get
    Assert-CoachPackage (@($CoachState.sources).Count -gt 0) 'Installed app did not load bundled notes.'
    Assert-CoachPackage (-not $CoachState.keys.groq -and -not $CoachState.keys.nvidia) 'Fresh app data unexpectedly has credentials.'
    Assert-CoachPackage (Test-Path -LiteralPath (Join-Path $CoachData '.env')) 'First launch did not create the key file.'
    $CoachKeyFile = [IO.File]::ReadAllLines((Join-Path $CoachData '.env'))
    foreach ($CoachLine in $CoachKeyFile) {
        if ($CoachLine -match '^\s*(GROQ_API_KEY|NVIDIA_API_KEY)\s*=\s*(.*?)\s*$') {
            Assert-CoachPackage ($Matches[2] -eq '') 'A fresh key file contains a nonempty credential.'
        }
    }
    $CoachChecks.Add('Installed binary served bundled notes on loopback and created a blank key file in isolated app data.')
    Stop-Process -Id $CoachProcess.Id
    $CoachProcess.WaitForExit()
    $CoachProcess = $null

    # Tamper only with this test's extracted copy; verification must fail before installation.
    [IO.File]::AppendAllText((Join-Path $CoachSource 'README.txt'), "`nchecksum test`n")
    $CoachRejected = $false
    $CoachRejectedDestination = Join-Path $CoachTestRoot 'must-not-install'
    try { & (Join-Path $CoachSource 'install.ps1') -InstallDir $CoachRejectedDestination -NoShortcut | Out-Null } catch {
        if ($_.Exception.Message -like 'Package checksum mismatch:*') { $CoachRejected = $true } else { throw }
    }
    Assert-CoachPackage $CoachRejected 'The installer accepted a damaged package.'
    Assert-CoachPackage (-not (Test-Path -LiteralPath $CoachRejectedDestination)) 'A failed integrity check created an installation.'
    $CoachChecks.Add('A damaged package was rejected before destination creation.')

    [IO.File]::WriteAllText((Join-Path $CoachInstall 'keep-user-file.txt'), 'This extra file must survive removal.')
    & (Join-Path $CoachInstall 'uninstall.ps1')
    Assert-CoachPackage (-not (Test-Path -LiteralPath (Join-Path $CoachInstall 'study-coach-app.exe'))) 'Removal left the executable.'
    Assert-CoachPackage (Test-Path -LiteralPath (Join-Path $CoachInstall 'keep-user-file.txt')) 'Removal deleted an unrelated file.'
    Assert-CoachPackage (Test-Path -LiteralPath (Join-Path $CoachData '.env')) 'Removal deleted app data.'
    $CoachChecks.Add('Removal preserved unrelated files and the separate app data directory.')
    Write-Output 'Package checks PASS'
    foreach ($CoachCheck in $CoachChecks) { Write-Output $CoachCheck }
    $CoachReceipt = [ordered]@{
        archive = [IO.Path]::GetFileName($CoachArchive)
        archive_sha256 = (Get-FileHash -LiteralPath $CoachArchive -Algorithm SHA256).Hash.ToLowerInvariant()
        platform = 'Windows x86_64 GNU'
        status = 'pass'
        checks = $CoachChecks.ToArray()
        windows_runtime_dependencies = $CoachDependencies
        limits = @('No real provider request was made by this package test.', 'Linux and macOS runtime behavior was not verified by this Windows test.', 'Checksums detect corruption and do not authenticate a publisher.')
    }
    $CoachReceiptPath = [IO.Path]::GetFullPath($ReceiptPath)
    [IO.File]::WriteAllText($CoachReceiptPath, ($CoachReceipt | ConvertTo-Json -Depth 4), (New-Object Text.UTF8Encoding($false)))
} finally {
    if ($CoachProcess -and -not $CoachProcess.HasExited) { Stop-Process -Id $CoachProcess.Id }
    # This test owns only its generated GUID folder. Validate the absolute target before recursive cleanup.
    $CoachResolvedTest = [IO.Path]::GetFullPath($CoachTestRoot)
    $CoachTempParent = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\', '/')
    if (-not $CoachResolvedTest.StartsWith($CoachTempParent + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase) -or
        [IO.Path]::GetFileName($CoachResolvedTest) -notmatch '^study-coach-package-test-[0-9a-f]{32}$') {
        throw 'The package test cleanup boundary check failed.'
    }
    if (Test-Path -LiteralPath $CoachResolvedTest) { Remove-Item -LiteralPath $CoachResolvedTest -Recurse }
}
