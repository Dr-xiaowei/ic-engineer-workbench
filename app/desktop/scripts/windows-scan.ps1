param([Parameter(Mandatory=$true)][string]$Path, [Parameter(Mandatory=$true)][string]$Log)
$ErrorActionPreference = 'Stop'
$target = (Resolve-Path -LiteralPath $Path).Path
$started = Get-Date
$status = Get-MpComputerStatus
if (-not $status.AMServiceEnabled -or -not $status.AntivirusEnabled) {
  throw 'Windows Defender is unavailable; refusing to execute unscanned downloads.'
}
if ($status.AntivirusSignatureLastUpdated -lt (Get-Date).AddDays(-7)) { Update-MpSignature }
$status = Get-MpComputerStatus
if ($status.AntivirusSignatureLastUpdated -lt (Get-Date).AddDays(-7)) { throw 'Defender signatures remain stale after update' }
$scanner = Get-ChildItem "$env:ProgramData/Microsoft/Windows Defender/Platform/*/MpCmdRun.exe" -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1 -ExpandProperty FullName
if (-not $scanner) { $scanner = Join-Path $env:ProgramFiles 'Windows Defender/MpCmdRun.exe' }
if (-not (Test-Path -LiteralPath $scanner)) { throw 'Defender command-line scanner unavailable' }
# This scan-only option ignores exclusions and includes archive contents. It
# suppresses remediation only for this scan; system protection stays enabled.
# With remediation disabled, exit 0 means no detection; any error blocks use.
$scanOutput = & $scanner -Scan -ScanType 3 -File $target -DisableRemediation 2>&1
$scanExit = $LASTEXITCODE
$scanOutput | Out-File -LiteralPath "$Log.log" -Encoding utf8
$record = @{ path=$target; engine=$status.AMEngineVersion; signature=$status.AntivirusSignatureVersion; startedAt=$started.ToUniversalTime().ToString('o'); completedAt=(Get-Date).ToUniversalTime().ToString('o'); exitCode=$scanExit; exclusionsIgnored=$true; archiveContentsIncluded=$true; status=$(if ($scanExit -eq 0) { 'passed' } else { 'failed' }) }
$record | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $Log -Encoding utf8
if ($scanExit -ne 0) { throw "Defender scan did not pass (exit $scanExit). Refusing to use the files; inspect $Log.log." }
