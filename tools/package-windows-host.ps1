param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path $RepositoryRoot).Path
$output = Join-Path $root 'target/m5/windows-host'
$zip = Join-Path $root 'target/m5/sico-desktop-host-windows-x86_64.zip'
$previous = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
Push-Location $root
try {
  & $CargoPath build --offline --locked --release -p sico-desktop-host
  if ($LASTEXITCODE -ne 0) { throw 'Windows Desktop Host release build failed' }
  if (Test-Path -LiteralPath $output) { Remove-Item -LiteralPath $output -Recurse -Force }
  New-Item -ItemType Directory -Path $output | Out-Null
  Copy-Item -LiteralPath (Join-Path $root 'target/release/sico-desktop-host.exe') -Destination $output
  Copy-Item -LiteralPath (Join-Path $root 'assets/desktop/sico-desktop-host.ico') -Destination $output
  Copy-Item -LiteralPath (Join-Path $root 'assets/desktop/README.txt') -Destination $output
  if (Test-Path -LiteralPath $zip) { Remove-Item -LiteralPath $zip -Force }
  Compress-Archive -Path (Join-Path $output '*') -DestinationPath $zip
  Write-Output "WINDOWS_HOST_PACKAGE_OK files=3 zip=$zip"
} finally {
  Pop-Location
  $env:RUSTUP_TOOLCHAIN = $previous
}
