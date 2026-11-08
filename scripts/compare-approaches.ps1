# Evaluate the unchanged rubric against both prompt strategies. Credentials are
# loaded by the Rust CLI; this script never opens or displays credential files.
[CmdletBinding()]
param(
    [ValidateRange(1, 10)]
    [int]$Runs = 1,
    [string]$OutDirectory = (Join-Path $PSScriptRoot '..\results\milestone2-2026-11-08')
)

$ErrorActionPreference = 'Stop'
$CoachRoot = Split-Path -Parent $PSScriptRoot
$CoachCargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
$CoachCargo = if ($CoachCargoCommand) { $CoachCargoCommand.Source } else {
    Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
}
if (-not (Test-Path -LiteralPath $CoachCargo)) {
    throw 'Rust Cargo is missing. Follow README.md to install Rust.'
}
$CoachToolchain = @()
if ($env:OS -eq 'Windows_NT' -and -not (Get-Command link.exe -ErrorAction SilentlyContinue)) {
    $CoachRustup = Join-Path (Split-Path -Parent $CoachCargo) 'rustup.exe'
    if ((Test-Path -LiteralPath $CoachRustup) -and ((& $CoachRustup toolchain list) -match '^stable-x86_64-pc-windows-gnu')) {
        $CoachToolchain = @('+stable-x86_64-pc-windows-gnu')
    }
}
$CoachOutput = [System.IO.Path]::GetFullPath($OutDirectory)
New-Item -ItemType Directory -Path $CoachOutput -Force | Out-Null
Push-Location -LiteralPath $CoachRoot
try {
    & $CoachCargo @CoachToolchain build --locked --release --bin study-coach
    if ($LASTEXITCODE -ne 0) { throw 'Rust build failed; evaluation did not start.' }
    $CoachBinaryName = if ($env:OS -eq 'Windows_NT') { 'study-coach.exe' } else { 'study-coach' }
    $CoachBinary = Join-Path $CoachRoot ('target\release\' + $CoachBinaryName)
    & $CoachBinary validate
    if ($LASTEXITCODE -ne 0) { throw 'Dataset validation failed; evaluation did not start.' }
    $CoachRows = @()
    $CoachManifests = @()
    foreach ($CoachApproach in @('baseline', 'grounded')) {
        $CoachRunDirectory = Join-Path $CoachOutput $CoachApproach
        $CoachLocationFlag = if (Test-Path -LiteralPath (Join-Path $CoachRunDirectory 'manifest.json')) { '--resume' } else { '--out' }
        & $CoachBinary evaluate --models groq-120b --runs $Runs --approach $CoachApproach $CoachLocationFlag $CoachRunDirectory
        if ($LASTEXITCODE -ne 0) {
            throw "Evaluation stopped for $CoachApproach. Evidence remains saved; rerun this command unchanged after fixing free access or quota."
        }
        $CoachSummary = Get-Content -Raw -LiteralPath (Join-Path $CoachRunDirectory 'summary.json') | ConvertFrom-Json
        $CoachManifest = Get-Content -Raw -LiteralPath (Join-Path $CoachRunDirectory 'manifest.json') | ConvertFrom-Json
        if (-not $CoachSummary.complete -or $CoachManifest.case_count -ne 50 -or $CoachManifest.runs -ne $Runs -or $CoachManifest.approach -ne $CoachApproach) {
            throw "Incomplete or mismatched $CoachApproach evidence. No final comparison was written."
        }
        $CoachManifests += $CoachManifest
        $CoachModel = $CoachSummary.models[0]
        $CoachRows += [pscustomobject][ordered]@{
            approach = $CoachApproach
            model = $CoachModel.model
            cases_per_run = $CoachSummary.cases_per_run
            runs = $CoachSummary.runs_per_model
            passed = $CoachModel.passed
            completed_requests = $CoachModel.successful_requests
            score_percent = $CoachModel.score_percent
            score_min_percent = $CoachModel.score_min_percent
            score_max_percent = $CoachModel.score_max_percent
            average_response_ms = $CoachModel.average_response_ms
            estimated_usd_per_1000_free_tier = $CoachModel.estimated_usd_per_1000_free_tier
            cost_condition = $CoachModel.cost_condition
            fingerprint = $CoachManifest.fingerprint
            evidence = "$CoachApproach/records.jsonl"
        }
    }
    $CoachBeforeSettings = $CoachManifests[0].settings | ConvertTo-Json -Depth 20 -Compress
    $CoachAfterSettings = $CoachManifests[1].settings | ConvertTo-Json -Depth 20 -Compress
    if ($CoachBeforeSettings -cne $CoachAfterSettings) {
        throw 'Generation settings differ between approaches. No final comparison was written.'
    }
    foreach ($CoachControlledField in @('dataset_fingerprint', 'notes_fingerprint', 'scorer_fingerprint')) {
        if (-not $CoachManifests[0].$CoachControlledField -or $CoachManifests[0].$CoachControlledField -cne $CoachManifests[1].$CoachControlledField) {
            throw "Controlled input $CoachControlledField differs between approaches. No final comparison was written."
        }
    }
    $CoachComparison = [pscustomobject][ordered]@{
        schema_version = 1
        model = 'openai/gpt-oss-120b'
        provider = 'groq'
        dataset = 'data/eval.json'
        dataset_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $CoachRoot 'data\eval.json')).Hash.ToLowerInvariant()
        notes_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $CoachRoot 'data\notes.json')).Hash.ToLowerInvariant()
        changed_variable = 'System prompt only; baseline instructions versus explicit task and contiguous quote verification instructions'
        rubric_changed = $false
        rows = $CoachRows
        improvement_percentage_points = $CoachRows[1].score_percent - $CoachRows[0].score_percent
        limitation = if ($Runs -eq 1) { 'One run per approach; this does not estimate output variability or statistical significance. Milestone 1 has three runs per model.' } else { 'Repeated runs measure observed output variation, not a guarantee of statistical significance.' }
    }
    $CoachEncoding = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText((Join-Path $CoachOutput 'comparison.json'), ($CoachComparison | ConvertTo-Json -Depth 30) + [Environment]::NewLine, $CoachEncoding)
    $CoachRows | Export-Csv -LiteralPath (Join-Path $CoachOutput 'comparison.csv') -NoTypeInformation -Encoding UTF8
    $CoachRows | Format-Table approach, passed, completed_requests, score_percent, average_response_ms -AutoSize
    Write-Output ('Score change: {0:N2} percentage points.' -f $CoachComparison.improvement_percentage_points)
    Write-Output $CoachComparison.limitation
} finally {
    Pop-Location
}
