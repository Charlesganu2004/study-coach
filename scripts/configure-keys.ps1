# Run interactively yourself. Keys are never echoed or passed as command arguments.
$ErrorActionPreference = 'Stop'
$CoachKeyFile = Join-Path (Split-Path -Parent $PSScriptRoot) '.env'
function Assert-CoachKeyFileEmpty {
    if (-not (Test-Path -LiteralPath $CoachKeyFile)) { return }
    foreach ($CoachLine in [System.IO.File]::ReadAllLines($CoachKeyFile)) {
        $CoachLine = $CoachLine.Trim().TrimStart([char]0xfeff)
        if ([string]::IsNullOrWhiteSpace($CoachLine) -or $CoachLine.StartsWith('#')) { continue }
        $CoachAssignment = $CoachLine.Split('=', 2)
        if ($CoachAssignment.Count -ne 2 -or $CoachAssignment[0].Trim() -notin @('GROQ_API_KEY', 'NVIDIA_API_KEY')) {
            throw '.env contains unsupported content. Edit it locally; no file was written.'
        }
        $CoachExistingValue = $CoachAssignment[1].Trim()
        if ($CoachExistingValue -eq '""' -or $CoachExistingValue -eq "''") { $CoachExistingValue = '' }
        if (-not [string]::IsNullOrWhiteSpace($CoachExistingValue)) {
            throw '.env already contains a credential. Edit it locally; no file was written.'
        }
    }
}
Assert-CoachKeyFileEmpty
$CoachGroqSecure = Read-Host 'Groq API key from your free-plan account' -AsSecureString
$CoachNvidiaSecure = Read-Host 'NVIDIA API key with free endpoint access' -AsSecureString
$CoachGroqKey = [System.Net.NetworkCredential]::new('', $CoachGroqSecure).Password
$CoachNvidiaKey = [System.Net.NetworkCredential]::new('', $CoachNvidiaSecure).Password
if ([string]::IsNullOrWhiteSpace($CoachGroqKey) -or [string]::IsNullOrWhiteSpace($CoachNvidiaKey)) {
    throw 'Both keys are required; no file was written.'
}
if ($CoachGroqKey -match '\s' -or $CoachNvidiaKey -match '\s') {
    throw 'API keys must not contain whitespace; no file was written.'
}
$CoachUtf8 = [System.Text.UTF8Encoding]::new($false)
Assert-CoachKeyFileEmpty
[System.IO.File]::WriteAllText($CoachKeyFile, "GROQ_API_KEY=$CoachGroqKey`nNVIDIA_API_KEY=$CoachNvidiaKey`n", $CoachUtf8)
$CoachGroqKey = $null
$CoachNvidiaKey = $null
Write-Host 'Keys saved locally in ignored .env. Run scripts/coach.ps1 doctor next.'
