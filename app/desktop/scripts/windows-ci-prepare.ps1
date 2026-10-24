$ErrorActionPreference = 'Stop'
$root = (Resolve-Path "$PSScriptRoot/../../..").Path
$work = Join-Path $root 'artifacts/windows-validation/ci'
New-Item -ItemType Directory -Path $work -Force | Out-Null
$downloads = Join-Path $work 'downloads'
New-Item -ItemType Directory -Path $downloads | Out-Null
$url = 'https://registry.npmjs.org/pnpm/-/pnpm-11.19.0.tgz'
$archive = Join-Path $downloads 'pnpm.tgz'
Invoke-WebRequest -Uri $url -OutFile $archive
$hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
if ($hash -ne 'b9e49603540d04107b98e93917a30e6114970d403c23e40309a44ea9c2bca7fd') { throw 'pnpm checksum mismatch' }
& "$PSScriptRoot/windows-scan.ps1" -Path $downloads -Log "$work/pnpm-archive-scan.json"
tar -xzf $archive -C $downloads
if ($LASTEXITCODE) { throw 'pnpm extraction failed' }
& "$PSScriptRoot/windows-scan.ps1" -Path "$downloads/package" -Log "$work/pnpm-source-scan.json"
$pnpm = Join-Path $downloads 'package/bin/pnpm.cjs'
if (-not (Test-Path $pnpm)) { $pnpm = Join-Path $downloads 'package/bin/pnpm.mjs' }
$env:PNPM_MANAGE_PACKAGE_MANAGER_VERSIONS = 'false'
Set-Location "$root/app/desktop"
node $pnpm install --frozen-lockfile --ignore-scripts --config.manage-package-manager-versions=false
if ($LASTEXITCODE) { throw 'Locked dependency installation failed' }
cargo fetch --locked --target x86_64-pc-windows-msvc --manifest-path src-tauri/Cargo.toml
if ($LASTEXITCODE) { throw 'Locked Rust dependency fetch failed' }
& "$PSScriptRoot/windows-scan.ps1" -Path node_modules -Log "$work/frontend-dependencies-scan.json"
& "$PSScriptRoot/windows-scan.ps1" -Path "$env:USERPROFILE/.cargo/registry" -Log "$work/rust-dependencies-scan.json"
# Wrapper is task-local; no global pnpm install or system configuration changes.
$bin = Join-Path $work 'bin'
New-Item -ItemType Directory -Path $bin | Out-Null
"@echo off`r`nnode `"$pnpm`" %*" | Set-Content "$bin/pnpm.cmd" -Encoding ascii
$bin | Out-File -FilePath $env:GITHUB_PATH -Encoding utf8 -Append
@{ pnpmUrl=$url; pnpmSha256=$hash; node=(& node --version); rust=(& rustc --version); image=$env:ImageVersion; runner=$env:RUNNER_OS } | ConvertTo-Json | Set-Content "$work/toolchain.json" -Encoding utf8
python "$PSScriptRoot/prepare-windows-fixtures.py" --output "$work/fixtures"
if ($LASTEXITCODE) { throw 'Fixture generation failed' }
