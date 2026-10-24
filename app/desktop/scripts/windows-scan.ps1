param([Parameter(Mandatory=$true)][string]$Path, [Parameter(Mandatory=$true)][string]$Log)
$ErrorActionPreference = 'Stop'
$target = (Resolve-Path -LiteralPath $Path).Path
$started = Get-Date
$status = Get-MpComputerStatus
if (-not $status.AMServiceEnabled -or -not $status.AntivirusEnabled) {
  throw 'Windows Defender is unavailable; refusing to execute unscanned downloads.'
}
if ($status.AntivirusSignatureLastUpdated -lt (Get-Date).AddDays(-7)) { Update-MpSignature }
# A custom scan does not disable protection, add exclusions, or change system trust.
Start-MpScan -ScanType CustomScan -ScanPath $target
$detections = @(Get-MpThreatDetection | Where-Object { $_.InitialDetectionTime -ge $started })
$record = @{ path=$target; engine=$status.AMEngineVersion; signature=$status.AntivirusSignatureVersion; startedAt=$started.ToUniversalTime().ToString('o'); completedAt=(Get-Date).ToUniversalTime().ToString('o'); detections=$detections.Count }
$record | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $Log -Encoding utf8
if ($detections.Count) { throw 'Defender detected a threat. Downloads remain quarantined; inspect the runner security log.' }
