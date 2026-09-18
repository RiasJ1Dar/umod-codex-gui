$vendor = "C:\Users\Гартунг\Desktop\umod-codex-gui\vendor"
$cache = "$env:USERPROFILE\.cargo\registry\cache\index.crates.io-1949cf8c6b5b557f"
if (Test-Path $vendor) { Remove-Item $vendor -Recurse -Force }
New-Item -ItemType Directory -Path $vendor -Force | Out-Null
$crates = Get-ChildItem $cache -Filter "*.crate"
$count = 0
foreach ($c in $crates) {
    $dest = Join-Path $vendor $c.BaseName
    New-Item -ItemType Directory -Path $dest -Force | Out-Null
    tar -xzf $c.FullName -C $dest 2>$null
    $count++
    if ($count % 50 -eq 0) { Write-Host "  $count / $($crates.Count)" }
}
Write-Host "Готово: $count крейтів"
