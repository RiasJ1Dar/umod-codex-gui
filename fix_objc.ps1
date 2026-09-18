$vdir = "C:\Users\Гартунг\Desktop\umod-codex-gui\vendor\objc-sys-0.3.5"
$inner = Join-Path $vdir "objc-sys-0.3.5"
if (Test-Path (Join-Path $inner "src")) {
    Move-Item (Join-Path $inner "src") (Join-Path $vdir "src") -Force
}
if (Test-Path $inner) { Remove-Item $inner -Recurse -Force }
$cs = Join-Path $vdir ".cargo-checksum.json"
if (-not (Test-Path $cs)) { '{"files":{}}' | Out-File -FilePath $cs -Encoding ascii -NoNewline }
Write-Host "Cargo.toml: $(Test-Path (Join-Path $vdir 'Cargo.toml')) checksum: $(Test-Path $cs)"
