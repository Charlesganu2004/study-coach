[CmdletBinding()]
param([switch]$Push, [switch]$TagAlpha, [switch]$CheckOnly)

$ErrorActionPreference = 'Stop'
if ($CheckOnly -and ($Push -or $TagAlpha)) { throw 'Use -CheckOnly alone. It never stages, commits, tags, or pushes.' }

function Invoke-CoachGit {
    param([Parameter(Mandatory = $true)][string[]]$GitArguments)
    & git --no-pager @GitArguments
    if ($LASTEXITCODE -ne 0) { throw "Git command failed: $($GitArguments[0])." }
}

function Test-CoachLedger {
    param([string]$Directory, [int]$Runs, [string]$Approach, [switch]$Retrieval)
    $CoachIssues = [Collections.Generic.List[string]]::new()
    $CoachLatest = [Collections.Generic.Dictionary[string,object]]::new([StringComparer]::Ordinal)
    $CoachReader = $null
    try {
        $CoachCases = Get-Content -Raw -Encoding UTF8 -LiteralPath 'data/eval.json' | ConvertFrom-Json
        $CoachInputs = [Collections.Generic.Dictionary[string,string]]::new([StringComparer]::Ordinal)
        foreach ($CoachCase in $CoachCases) { $CoachInputs.Add($CoachCase.id, $CoachCase.input) }
        if ($CoachInputs.Count -ne 50) { throw 'The frozen dataset must contain 50 unique case IDs.' }
        $CoachAbsoluteDirectory = if ([IO.Path]::IsPathRooted($Directory)) { $Directory } else { Join-Path (Get-Location).Path $Directory }
        $CoachReader = [IO.File]::OpenText((Join-Path $CoachAbsoluteDirectory 'records.jsonl'))
        while ($null -ne ($CoachLine = $CoachReader.ReadLine())) {
            $CoachRecord = $CoachLine | ConvertFrom-Json
            if (([string]$CoachRecord.run) -notmatch '^[1-9][0-9]*$' -or $CoachRecord.run -gt $Runs -or
                -not $CoachInputs.ContainsKey([string]$CoachRecord.case_id) -or
                $CoachRecord.profile -cne 'groq-120b' -or $CoachRecord.provider -cne 'groq' -or
                $CoachRecord.model -cne 'openai/gpt-oss-120b' -or $CoachRecord.approach -cne $Approach -or
                $CoachRecord.PSObject.Properties.Name -notcontains 'error') {
                throw 'An unexpected record appears in the ledger.'
            }
            if ($CoachRecord.input -cne $CoachInputs[[string]$CoachRecord.case_id] -or
                ($Retrieval -and $CoachRecord.context_mode -cne 'lexical_top4')) {
                throw 'A record does not match the frozen input or context mode.'
            }
            $CoachLatest[([string]$CoachRecord.run + '|' + $CoachRecord.case_id)] = $CoachRecord
        }
        $CoachComplete = 0
        for ($CoachRun = 1; $CoachRun -le $Runs; $CoachRun++) {
            foreach ($CoachId in $CoachInputs.Keys) {
                $CoachKey = [string]$CoachRun + '|' + $CoachId
                if ($CoachLatest.ContainsKey($CoachKey) -and $null -eq $CoachLatest[$CoachKey].error) { $CoachComplete++ }
            }
        }
        if ($CoachComplete -ne (50 * $Runs) -or $CoachLatest.Count -ne (50 * $Runs)) {
            $CoachIssues.Add("$Directory has $CoachComplete/$([int](50 * $Runs)) completed final responses for the original case IDs")
        }
    } catch { $CoachIssues.Add("$Directory ledger is unreadable, malformed, or differs from the original 50 case inputs") }
    finally { if ($CoachReader) { $CoachReader.Dispose() } }
    return $CoachIssues.ToArray()
}

function Get-CoachEvidence {
    $CoachPending = [Collections.Generic.List[string]]::new()
    $CoachPaths = [Collections.Generic.List[string]]::new()
    foreach ($CoachName in @('manifest.json', 'records.jsonl', 'summary.json', 'MODEL_SELECTION.md', 'QUALITATIVE_AUDIT.md', 'VERIFICATION.json', 'model_comparison.csv', 'evaluation_results.csv')) {
        $CoachPath = "results/milestone1-2026-11-01/$CoachName"
        if (Test-Path -LiteralPath $CoachPath -PathType Leaf) { $CoachPaths.Add($CoachPath) } else { $CoachPending.Add($CoachPath) }
    }
    if (-not (Test-Path -LiteralPath 'data/eval.csv' -PathType Leaf)) { $CoachPending.Add('data/eval.csv') }
    $CoachM2 = 'results/milestone2-2026-11-08'
    foreach ($CoachName in @('comparison.json', 'comparison.csv', 'graduate_comparison.csv')) {
        $CoachPath = "$CoachM2/$CoachName"
        if (Test-Path -LiteralPath $CoachPath -PathType Leaf) { $CoachPaths.Add($CoachPath) } else { $CoachPending.Add($CoachPath) }
    }
    $CoachManifests = @()
    foreach ($CoachApproach in @('baseline', 'grounded')) {
        foreach ($CoachName in @('manifest.json', 'records.jsonl', 'summary.json', 'MODEL_SELECTION.md')) {
            $CoachPath = "$CoachM2/$CoachApproach/$CoachName"
            if (Test-Path -LiteralPath $CoachPath -PathType Leaf) { $CoachPaths.Add($CoachPath) } else { $CoachPending.Add($CoachPath) }
        }
        $CoachSummaryPath = "$CoachM2/$CoachApproach/summary.json"
        $CoachManifestPath = "$CoachM2/$CoachApproach/manifest.json"
        if ((Test-Path -LiteralPath $CoachSummaryPath) -and (Test-Path -LiteralPath $CoachManifestPath)) {
            $CoachSummary = Get-Content -Raw -Encoding UTF8 -LiteralPath $CoachSummaryPath | ConvertFrom-Json
            $CoachManifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $CoachManifestPath | ConvertFrom-Json
            $CoachManifests += $CoachManifest
            if (-not $CoachSummary.complete -or $CoachManifest.case_count -ne 50 -or $CoachManifest.approach -cne $CoachApproach -or
                $CoachManifest.runs -lt 1 -or $CoachManifest.runs -gt 10 -or @($CoachSummary.models).Count -ne 1 -or
                $CoachSummary.models[0].successful_requests -ne (50 * $CoachManifest.runs) -or
                $CoachSummary.models[0].model -cne 'openai/gpt-oss-120b' -or $CoachSummary.models[0].provider -cne 'groq' -or
                @($CoachManifest.profiles).Count -ne 1 -or $CoachManifest.profiles[0].name -cne 'groq-120b' -or
                $CoachManifest.profiles[0].model -cne 'openai/gpt-oss-120b' -or $CoachManifest.profiles[0].provider -cne 'groq') {
                $CoachPending.Add("$CoachApproach must contain a complete evaluation of all 50 cases for each run")
            } elseif (Test-Path -LiteralPath "$CoachM2/$CoachApproach/records.jsonl") {
                foreach ($CoachIssue in @(Test-CoachLedger -Directory "$CoachM2/$CoachApproach" -Runs $CoachManifest.runs -Approach $CoachApproach)) { $CoachPending.Add($CoachIssue) }
            }
        }
    }
    foreach ($CoachName in @('METHOD.md', 'ANALYSIS.md', 'VERIFICATION.json')) {
        $CoachPath = "$CoachM2/$CoachName"
        if (Test-Path -LiteralPath $CoachPath -PathType Leaf) { $CoachPaths.Add($CoachPath) }
    }
    $CoachRetrieval = 'results/milestone2-retrieval-2026-11-08'
    $CoachRetrievalManifest = $null
    foreach ($CoachName in @('manifest.json', 'records.jsonl', 'summary.json')) {
        $CoachPath = "$CoachRetrieval/$CoachName"
        if (Test-Path -LiteralPath $CoachPath -PathType Leaf) { $CoachPaths.Add($CoachPath) } else { $CoachPending.Add($CoachPath) }
    }
    foreach ($CoachName in @('MODEL_SELECTION.md', 'METHOD.md', 'VERIFICATION.json', 'ANALYSIS.md')) {
        $CoachPath = "$CoachRetrieval/$CoachName"
        if (Test-Path -LiteralPath $CoachPath -PathType Leaf) { $CoachPaths.Add($CoachPath) }
    }
    if ((Test-Path -LiteralPath "$CoachRetrieval/manifest.json") -and (Test-Path -LiteralPath "$CoachRetrieval/summary.json")) {
        $CoachRetrievalManifest = Get-Content -Raw -Encoding UTF8 -LiteralPath "$CoachRetrieval/manifest.json" | ConvertFrom-Json
        $CoachRetrievalSummary = Get-Content -Raw -Encoding UTF8 -LiteralPath "$CoachRetrieval/summary.json" | ConvertFrom-Json
        if (-not $CoachRetrievalSummary.complete -or $CoachRetrievalManifest.case_count -ne 50 -or
            $CoachRetrievalManifest.runs -ne 1 -or $CoachRetrievalManifest.approach -cne 'grounded' -or
            $CoachRetrievalManifest.context_mode -cne 'lexical_top4' -or $CoachRetrievalManifest.top_k -ne 4 -or
            @($CoachRetrievalManifest.profiles).Count -ne 1 -or $CoachRetrievalManifest.profiles[0].name -cne 'groq-120b' -or
            $CoachRetrievalManifest.profiles[0].provider -cne 'groq' -or $CoachRetrievalManifest.profiles[0].model -cne 'openai/gpt-oss-120b' -or
            @($CoachRetrievalSummary.models).Count -ne 1 -or $CoachRetrievalSummary.models[0].successful_requests -ne 50 -or
            $CoachRetrievalSummary.models[0].model -cne 'openai/gpt-oss-120b' -or $CoachRetrievalSummary.models[0].provider -cne 'groq') {
            $CoachPending.Add('Retrieval must contain 50 completed responses using the grounded selected model and lexical top four context')
        } elseif (Test-Path -LiteralPath "$CoachRetrieval/records.jsonl") {
            foreach ($CoachIssue in @(Test-CoachLedger -Directory $CoachRetrieval -Runs 1 -Approach 'grounded' -Retrieval)) { $CoachPending.Add($CoachIssue) }
        }
    }
    if ($CoachPending.Count -eq 0) {
        $CoachComparison = Get-Content -Raw -Encoding UTF8 -LiteralPath "$CoachM2/comparison.json" | ConvertFrom-Json
        if (@($CoachComparison.rows).Count -ne 2 -or $CoachComparison.model -cne 'openai/gpt-oss-120b') {
            $CoachPending.Add('The comparison must contain both approaches using the selected model')
        } else {
            foreach ($CoachRow in $CoachComparison.rows) {
                $CoachManifest = @($CoachManifests | Where-Object { $_.approach -ceq $CoachRow.approach })
                if ($CoachManifest.Count -ne 1 -or $CoachRow.cases_per_run -ne 50 -or
                    $CoachRow.runs -ne $CoachManifest[0].runs -or $CoachRow.completed_requests -ne (50 * $CoachRow.runs) -or
                    $CoachRow.fingerprint -cne $CoachManifest[0].fingerprint) {
                    $CoachPending.Add('Comparison rows do not match the saved complete experiments')
                }
            }
            if (@($CoachComparison.rows | Select-Object -ExpandProperty approach -Unique).Count -ne 2) {
                $CoachPending.Add('Comparison approaches are duplicated')
            }
            if ($CoachManifests[0].runs -ne $CoachManifests[1].runs) {
                $CoachPending.Add('Run counts differ between approaches')
            }
            foreach ($CoachField in @('dataset_fingerprint', 'notes_fingerprint', 'scorer_fingerprint')) {
                if (-not $CoachManifests[0].$CoachField -or $CoachManifests[0].$CoachField -cne $CoachManifests[1].$CoachField) {
                    $CoachPending.Add("Controlled input differs: $CoachField")
                }
            }
            if (($CoachManifests[0].settings | ConvertTo-Json -Depth 20 -Compress) -cne ($CoachManifests[1].settings | ConvertTo-Json -Depth 20 -Compress)) {
                $CoachPending.Add('Generation settings differ between approaches')
            }
            $CoachGroundedManifest = @($CoachManifests | Where-Object { $_.approach -ceq 'grounded' })[0]
            foreach ($CoachField in @('dataset_fingerprint', 'notes_fingerprint', 'scorer_fingerprint', 'system_prompt')) {
                if (-not $CoachRetrievalManifest.$CoachField -or $CoachRetrievalManifest.$CoachField -cne $CoachGroundedManifest.$CoachField) {
                    $CoachPending.Add("Retrieval differs from grounded full context in a controlled field: $CoachField")
                }
            }
            if (($CoachRetrievalManifest.settings | ConvertTo-Json -Depth 20 -Compress) -cne ($CoachGroundedManifest.settings | ConvertTo-Json -Depth 20 -Compress)) {
                $CoachPending.Add('Retrieval generation settings differ from grounded full context')
            }
        }
    }
    [pscustomobject]@{ Ready = $CoachPending.Count -eq 0; Pending = $CoachPending.ToArray(); Paths = $CoachPaths.ToArray() }
}

function Get-CoachRemoteAlpha {
    $CoachTagLines = @(Invoke-CoachGit -GitArguments @('ls-remote', '--tags', 'origin', 'refs/tags/alpha', 'refs/tags/alpha^{}'))
    $CoachPeeled = @($CoachTagLines | Where-Object { $_ -match '\srefs/tags/alpha\^\{\}$' })
    if ($CoachPeeled.Count -gt 0) { return ([string]$CoachPeeled[0] -split '\s+')[0] }
    if ($CoachTagLines.Count -gt 0) { return ([string]$CoachTagLines[0] -split '\s+')[0] }
    return $null
}

$CoachRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$CoachIdentity = 'Charles Ganu <charlesganu2004@gmail.com>'
$CoachMessage = 'Complete Milestone 2 prototype and application distribution'
Push-Location -LiteralPath $CoachRoot
try {
    Get-Command git, gh -ErrorAction Stop | Out-Null
    $CoachGitRoot = (Invoke-CoachGit -GitArguments @('rev-parse', '--show-toplevel') | Out-String).Trim()
    if ([IO.Path]::GetFullPath($CoachGitRoot) -ine $CoachRoot) { throw 'Run this script from its standalone study-coach-rust repository.' }
    if ((Invoke-CoachGit -GitArguments @('branch', '--show-current') | Out-String).Trim() -cne 'main') { throw 'This script expects main and will not switch branches.' }
    $CoachRemote = (Invoke-CoachGit -GitArguments @('remote', 'get-url', 'origin') | Out-String).Trim()
    if ($CoachRemote -notmatch '^(https://github\.com/Charlesganu2004/study-coach(?:\.git)?|git@github\.com:Charlesganu2004/study-coach(?:\.git)?)$') { throw 'Origin is not the expected Charlesganu2004 repository.' }
    $CoachAccounts = @(Invoke-CoachGit -GitArguments @('credential-manager', 'github', 'list') | ForEach-Object { ([string]$_).Trim() })
    if ($CoachAccounts -notcontains 'Charlesganu2004') { throw 'Git Credential Manager must list Charlesganu2004 before committing.' }
    $CoachLogin = (& gh api user --jq .login | Out-String).Trim()
    if ($LASTEXITCODE -ne 0 -or $CoachLogin -cne 'Charlesganu2004') { throw 'GitHub CLI must be authenticated as Charlesganu2004.' }
    foreach ($CoachVariable in @('GIT_AUTHOR_IDENT', 'GIT_COMMITTER_IDENT')) {
        $CoachEffective = (Invoke-CoachGit -GitArguments @('var', $CoachVariable) | Out-String).Trim()
        if (-not $CoachEffective.StartsWith($CoachIdentity + ' ', [StringComparison]::Ordinal)) { throw 'Effective Git author or committer differs from the required Charles Ganu identity.' }
    }
    if (@(Invoke-CoachGit -GitArguments @('ls-files', '--', '.env', '.env.local')).Count -ne 0) { throw 'A credential file is tracked. Stop and inspect locally.' }
    foreach ($CoachCredential in @('.env', '.env.local')) {
        & git check-ignore --quiet -- $CoachCredential
        if ($LASTEXITCODE -ne 0) { throw 'A credential file is not covered by Git ignore rules.' }
    }
    $CoachCandidatePaths = @(Invoke-CoachGit -GitArguments @('ls-files', '--cached', '--others', '--exclude-standard'))
    if (@($CoachCandidatePaths | Where-Object { $_ -match '(^|/)\.env($|\.)|(^|/)(id_rsa|id_ed25519)($|\.)|\.(pem|p12|pfx|key)$' }).Count -gt 0) { throw 'A sensitive filename could enter the commit. Inspect locally before staging.' }
    $CoachEvidence = Get-CoachEvidence
    Write-Host 'Account and local safety checks PASS: expected repository, account, author, committer, and ignored credentials.'
    if (-not $CoachEvidence.Ready) {
        Write-Host 'Submission evidence pending:'
        foreach ($CoachPending in $CoachEvidence.Pending) { Write-Host "  $CoachPending" }
        if ($CheckOnly) { Write-Host 'CheckOnly: no staging, commit, tag, or push performed.'; return }
        throw 'Finish the complete before and after evaluation before creating the submission commit.'
    }
    Write-Host 'Submission evidence checks PASS: complete baseline, grounded, and retrieval experiments on the original 50 cases.'
    if ($CheckOnly) { Write-Host 'CheckOnly: no staging, commit, tag, or push performed.'; return }

    Invoke-CoachGit -GitArguments @('add', '--all')
    foreach ($CoachPath in $CoachEvidence.Paths) { Invoke-CoachGit -GitArguments @('add', '-f', '--', $CoachPath) }
    $CoachAddedPaths = @(Invoke-CoachGit -GitArguments @('diff', '--cached', '--name-only', '--diff-filter=ACMR'))
    if (@($CoachAddedPaths | Where-Object { $_ -match '(^|/)\.env($|\.)|(^|/)(id_rsa|id_ed25519)($|\.)|\.(pem|p12|pfx|key)$' }).Count -gt 0) { throw 'A sensitive filename is staged. Inspect locally before committing.' }
    Invoke-CoachGit -GitArguments @('diff', '--cached', '--check')
    Invoke-CoachGit -GitArguments @('diff', '--cached', '--stat')
    & git diff --cached --quiet
    $CoachDiffStatus = $LASTEXITCODE
    $CoachCreatedCommit = $false
    if ($CoachDiffStatus -eq 1) {
        Invoke-CoachGit -GitArguments @('commit', '-m', $CoachMessage)
        $CoachCreatedCommit = $true
    } elseif ($CoachDiffStatus -eq 0) {
        Write-Host 'No changes staged; inspecting the existing latest commit.'
    } else { throw 'Could not inspect staged changes.' }
    $CoachStoredAuthor = (Invoke-CoachGit -GitArguments @('show', '-s', '--format=%an <%ae>', 'HEAD') | Out-String).Trim()
    $CoachStoredCommitter = (Invoke-CoachGit -GitArguments @('show', '-s', '--format=%cn <%ce>', 'HEAD') | Out-String).Trim()
    $CoachStoredMessage = (Invoke-CoachGit -GitArguments @('show', '-s', '--format=%B', 'HEAD') | Out-String).Trim()
    $CoachTrailers = @($CoachStoredMessage | git interpret-trailers --parse)
    if ($LASTEXITCODE -ne 0) { throw 'Stored trailers could not be inspected.' }
    if ($CoachStoredAuthor -cne $CoachIdentity -or $CoachStoredCommitter -cne $CoachIdentity) { throw 'Stored commit identity is unexpected. No push will be attempted.' }
    if ($CoachTrailers.Count -ne 0 -or $CoachStoredMessage -match '(?im)co-authored-by:|generated with') { throw 'Unexpected attribution or trailers found. Inspect locally before pushing.' }
    if ($CoachCreatedCommit -and $CoachStoredMessage -cne $CoachMessage) { throw 'A hook changed the commit message. Inspect locally before pushing.' }
    Invoke-CoachGit -GitArguments @('log', '-1', '--format=fuller')
    Write-Host 'Stored author, committer, message, and empty trailers verified.'
    $CoachLocalHead = (Invoke-CoachGit -GitArguments @('rev-parse', 'HEAD') | Out-String).Trim()
    if ($TagAlpha) {
        $CoachLocalAlpha = (& git rev-parse --quiet --verify 'refs/tags/alpha^{commit}' | Out-String).Trim()
        if ($LASTEXITCODE -ne 0) { $CoachLocalAlpha = $null }
        if ($CoachLocalAlpha -and $CoachLocalAlpha -cne $CoachLocalHead) { throw 'The local alpha tag points to a different commit. It will not be moved or deleted.' }
        $CoachRemoteAlpha = Get-CoachRemoteAlpha
        if ($CoachRemoteAlpha -and $CoachRemoteAlpha -cne $CoachLocalHead) { throw 'The remote alpha tag points to a different commit. It will not be replaced.' }
        if (-not $CoachLocalAlpha) { Invoke-CoachGit -GitArguments @('tag', 'alpha', 'HEAD') }
        Write-Host 'Local alpha tag matches the inspected commit.'
    }
    if ($Push) {
        if ($TagAlpha) {
            Invoke-CoachGit -GitArguments @('push', '--atomic', '--set-upstream', 'origin', 'main', 'refs/tags/alpha:refs/tags/alpha')
        } else { Invoke-CoachGit -GitArguments @('push', '--set-upstream', 'origin', 'main') }
        $CoachRemoteMain = (Invoke-CoachGit -GitArguments @('ls-remote', 'origin', 'refs/heads/main') | Out-String).Trim()
        if (($CoachRemoteMain -split '\s+')[0] -cne $CoachLocalHead) { throw 'Remote main does not match the inspected local commit.' }
        if ($TagAlpha -and (Get-CoachRemoteAlpha) -cne $CoachLocalHead) { throw 'Remote alpha does not match the inspected local commit.' }
        Write-Host 'Push verified: remote main and any requested alpha tag match the inspected commit.'
    } else { Write-Host 'Commit prepared locally. Push was not requested.' }
} finally { Pop-Location }
