$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true') { throw 'Disposable GitHub runner required' }
$root = (Resolve-Path "$PSScriptRoot/../../..").Path
$evidence = Join-Path $root 'artifacts/windows-validation/ui'
New-Item -ItemType Directory -Path $evidence -Force | Out-Null
$fixtures = Join-Path $root 'artifacts/windows-validation/ci/fixtures'
function Install-Preview([string]$Version, [string]$Mode, [string]$Destination) {
  $candidate = Join-Path $root "artifacts/windows-preview/$Version/x64-installed"
  $manifest = Get-Content -LiteralPath "$candidate/manifest.json" -Raw | ConvertFrom-Json
  $entry = @($manifest.files | Where-Object { $_.path.StartsWith("$Mode/") })
  if ($entry.Count -ne 1) { throw 'Expected exactly one installer for the selected mode' }
  $installer = Join-Path $candidate $entry[0].path
  if ((Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash.ToLowerInvariant() -ne $entry[0].sha256) { throw 'Installer hash changed after scanning' }
  $process = Start-Process -FilePath $installer -ArgumentList @('/S', "/D=$Destination") -PassThru -Wait
  if ($process.ExitCode -ne 0) { throw "Installer failed: $($process.ExitCode)" }
  if (-not (Test-Path "$Destination/ic-engineer-workbench.exe")) { throw 'Installed application missing' }
}
function Check-Preview([string]$Mode, [string]$Phase, [string]$Version, [string]$Destination) {
  node "$PSScriptRoot/windows-native-ui.mjs" "$Destination/ic-engineer-workbench.exe" $Mode $Phase $Version $fixtures $evidence
  if ($LASTEXITCODE) { throw "Native UI failed: $Mode $Phase" }
}
$steps = @()
try {
  foreach ($mode in @('normal', 'demo')) {
    $destination = Join-Path $env:RUNNER_TEMP "icwb-installed-$mode"
    if (Test-Path $destination) { throw 'Refusing an existing install directory' }
    Install-Preview '1.1.0-rc.1' $mode $destination
    Check-Preview $mode 'initial' '1.1.0-rc.1' $destination
    $steps += "$mode installed and native functions checked"
    Install-Preview '1.1.0-rc.2' $mode $destination
    Check-Preview $mode 'upgrade' '1.1.0-rc.2' $destination
    $steps += "$mode rc.1 to rc.2 retains data"
    $uninstaller = Join-Path $destination 'uninstall.exe'
    if (-not (Test-Path $uninstaller)) { throw 'Uninstaller missing' }
    $process = Start-Process -FilePath $uninstaller -ArgumentList @('/S', "_?=$destination") -PassThru -Wait
    if ($process.ExitCode -ne 0) { throw "Uninstall failed: $($process.ExitCode)" }
    if (Test-Path "$destination/ic-engineer-workbench.exe") { throw 'Uninstaller did not remove the executable' }
    Install-Preview '1.1.0-rc.2' $mode $destination
    Check-Preview $mode 'reinstall' '1.1.0-rc.2' $destination
    $steps += "$mode uninstall/reinstall retains data"
  }
  @{ status='passed'; scope='Windows Server current-user installers under runner administrator, UAC disabled'; steps=$steps; remaining=@('Win10 workstation','standard user and UAC','interactive file dialogs and DPI','older released Windows binary unavailable') } | ConvertTo-Json -Depth 5 | Set-Content "$evidence/lifecycle.json" -Encoding utf8
} catch {
  @{ status='failed'; steps=$steps; error=$_.ToString() } | ConvertTo-Json -Depth 5 | Set-Content "$evidence/lifecycle.json" -Encoding utf8
  throw
}
