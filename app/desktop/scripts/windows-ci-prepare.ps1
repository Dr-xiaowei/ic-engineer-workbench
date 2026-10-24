$ErrorActionPreference = 'Stop'
$root = (Resolve-Path "$PSScriptRoot/../../..").Path
$clang = & clang-cl --version
if ($LASTEXITCODE) { throw 'Preinstalled LLVM clang-cl unavailable' }
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
# Preseed the exact tools used by tauri-cli 2.11.4 so its bundler never needs
# to execute a newly downloaded NSIS binary before our scanner has seen it.
$nsisArchive = Join-Path $downloads 'nsis-3.11.zip'
$plugin = Join-Path $downloads 'nsis_tauri_utils.dll'
Invoke-WebRequest 'https://github.com/tauri-apps/binary-releases/releases/download/nsis-3.11/nsis-3.11.zip' -OutFile $nsisArchive
Invoke-WebRequest 'https://github.com/tauri-apps/nsis-tauri-utils/releases/download/nsis_tauri_utils-v0.5.3/nsis_tauri_utils.dll' -OutFile $plugin
if ((Get-FileHash $nsisArchive -Algorithm SHA256).Hash -ne 'C7D27F780DDB6CFFB4730138CD1591E841F4B7EDB155856901CDF5F214394FA1') { throw 'NSIS checksum mismatch' }
if ((Get-FileHash $plugin -Algorithm SHA256).Hash -ne '5BA143B5DB4A87D32D6E7802E033330AAE56CBCEABE0D1E3BA41948385AD4709') { throw 'NSIS plugin checksum mismatch' }
& "$PSScriptRoot/windows-scan.ps1" -Path $downloads -Log "$work/nsis-download-scan.json"
$tools = Join-Path $env:LOCALAPPDATA 'tauri'
New-Item -ItemType Directory -Path $tools -Force | Out-Null
Expand-Archive -LiteralPath $nsisArchive -DestinationPath $tools
Move-Item (Join-Path $tools 'nsis-3.11') (Join-Path $tools 'NSIS')
$plugins = Join-Path $tools 'NSIS/Plugins/x86-unicode/additional'
New-Item -ItemType Directory -Path $plugins -Force | Out-Null
Copy-Item $plugin $plugins
& "$PSScriptRoot/windows-scan.ps1" -Path "$tools/NSIS" -Log "$work/nsis-expanded-scan.json"
# Wrapper is task-local; no global pnpm install or system configuration changes.
$bin = Join-Path $work 'bin'
New-Item -ItemType Directory -Path $bin | Out-Null
"@echo off`r`nnode `"$pnpm`" %*" | Set-Content "$bin/pnpm.cmd" -Encoding ascii
$bin | Out-File -FilePath $env:GITHUB_PATH -Encoding utf8 -Append
@{ pnpmUrl=$url; pnpmSha256=$hash; node=(& node --version); clang=$clang; rust=(& rustc --version); image=$env:ImageVersion; runner=$env:RUNNER_OS } | ConvertTo-Json | Set-Content "$work/toolchain.json" -Encoding utf8
python "$PSScriptRoot/prepare-windows-fixtures.py" --output "$work/fixtures"
if ($LASTEXITCODE) { throw 'Fixture generation failed' }
