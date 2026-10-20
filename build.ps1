$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $PSScriptRoot

cargo build --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

cargo test --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$dist = Join-Path $PSScriptRoot 'dist'
New-Item -ItemType Directory -Force -Path $dist | Out-Null
Copy-Item (Join-Path $PSScriptRoot 'target\release\PCLPatch.exe') (Join-Path $dist 'PCLPatch.exe') -Force

$versionLine = Select-String -Path (Join-Path $PSScriptRoot 'Cargo.toml') -Pattern '^version = "(.+)"' | Select-Object -First 1
$version = $versionLine.Matches[0].Groups[1].Value

$stage = Join-Path $env:TEMP ('PCLPatch-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $stage | Out-Null
Copy-Item (Join-Path $dist 'PCLPatch.exe') $stage -Force
Copy-Item (Join-Path $PSScriptRoot 'README.md') $stage -Force
Copy-Item (Join-Path $PSScriptRoot 'LICENSE') $stage -Force

$zip = Join-Path $dist ('PCLPatch-' + $version + '-x64.zip')
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $zip -Force
Remove-Item $stage -Recurse -Force

Write-Host ('build ok -> ' + (Join-Path $dist 'PCLPatch.exe'))
Write-Host ('pack  ok -> ' + $zip)
