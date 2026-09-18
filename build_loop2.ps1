$ErrorActionPreference = "Stop"
Set-Location "C:\Users\Гартунг\Desktop\umod-codex-gui"

$maxIter = 300
for ($i = 1; $i -le $maxIter; $i++) {
    $output = cargo build --release --offline 2>&1 | Out-String

    if ($output -match "Finished") {
        Write-Host "=== ZBIRKA Zavershena! ===" -ForegroundColor Green
        break
    }

    $crate = $null
    $ver = ""

    # Помилка 1: no matching package named `XXX` found
    if ($output -match "no matching package named\s+.([a-zA-Z0-9_-]+).") {
        $crate = $matches[1]
        Write-Host "[$i] Brakuje: $crate (latest)" -ForegroundColor Yellow
    }
    # Помилка 2: failed to select a version for `XXX = "^ver"`
    elseif ($output -match "failed to select a version for the requirement .([a-zA-Z0-9_-]+).*=\s*.?\^?([0-9][0-9.]+)") {
        $crate = $matches[1]
        $ver = $matches[2]
        Write-Host "[$i] Brakuje: $crate v$ver" -ForegroundColor Yellow
    }

    if ($crate) {
        $pyScript = @"
import urllib.request, os, tarfile, io, json, time, sys

CACHE = "/mnt/c/Users/Гартунг/.cargo/registry/cache/index.crates.io-1949cf8c6b5b557f"
VENDOR = "/mnt/c/Users/Гартунг/Desktop/umod-codex-gui/vendor"
HEADERS = {"User-Agent": "cargo-fetch-py"}

name = "$crate"
req_version = "$ver"

for attempt in range(6):
    try:
        url = f"https://crates.io/api/v1/crates/{name}"
        req = urllib.request.Request(url, headers=HEADERS)
        r = urllib.request.urlopen(req, timeout=15)
        d = json.load(r)
        versions = [v["num"] for v in d.get("versions", []) if not v["yanked"]]
        break
    except urllib.error.HTTPError as e:
        if e.code == 404:
            print(f"FAIL: {name} not found (404)")
            sys.exit(1)
        if e.code == 429:
            wait = 5 * (attempt + 1)
            print(f"  429, waiting {wait}s")
            time.sleep(wait)
            continue
        print(f"FAIL: HTTP {e.code}")
        sys.exit(1)
else:
    sys.exit(1)

if req_version:
    stable = [v for v in versions if "-" not in v]
    parts = req_version.split(".")
    if len(parts) >= 2:
        prefix = ".".join(parts[:2]) + "."
    else:
        prefix = req_version + "."
    matches = [v for v in stable if v.startswith(prefix)]
    version = matches[0] if matches else None
    if not version and req_version in versions:
        version = req_version
else:
    stable = [v for v in versions if "-" not in v]
    version = stable[0] if stable else (versions[0] if versions else None)

if not version:
    print(f"FAIL: no version for {name}")
    sys.exit(1)

crate_file = f"{name}-{version}.crate"
vdir = os.path.join(VENDOR, f"{name}-{version}")

if os.path.isdir(vdir) and os.path.isfile(os.path.join(vdir, "Cargo.toml")):
    print(f"  {crate_file} already in vendor")
    sys.exit(0)

print(f"  downloading {crate_file}...")
for attempt in range(4):
    try:
        url = f"https://static.crates.io/crates/{name}/{crate_file}"
        req = urllib.request.Request(url, headers=HEADERS)
        data = urllib.request.urlopen(req, timeout=30).read()
        break
    except urllib.error.HTTPError as e:
        if e.code == 429:
            time.sleep(5 * (attempt + 1))
            continue
        print(f"FAIL: download HTTP {e.code}")
        sys.exit(1)

with open(os.path.join(CACHE, crate_file), "wb") as f:
    f.write(data)
os.makedirs(vdir, exist_ok=True)
with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as tar:
    tar.extractall(vdir)
inner = os.path.join(vdir, f"{name}-{version}")
if os.path.isdir(inner):
    for item in os.listdir(inner):
        os.rename(os.path.join(inner, item), os.path.join(vdir, item))
    os.rmdir(inner)
with open(os.path.join(vdir, ".cargo-checksum.json"), "w") as f:
    f.write('{"files":{}}')
print(f"  OK {crate_file}")
"@
        $pyFile = "$env:TEMP\fetch_crate2.py"
        Set-Content -Path $pyFile -Value $pyScript -Encoding utf8
        $result = bash -c "python3 /mnt/c/Users/Гартунг/AppData/Local/Temp/fetch_crate2.py 2>&1"
        Write-Host $result
        if ($result -match "FAIL") { break }
        Start-Sleep -Milliseconds 300
        continue
    }

    if ($output -match "error\[") {
        Write-Host "[$i] Kompilyatsiyna pomylka:" -ForegroundColor Red
        Write-Host $output.Substring(0, [Math]::Min(3000, $output.Length))
        break
    }

    Write-Host "[$i] Insha pomylka:" -ForegroundColor Red
    Write-Host $output.Substring(0, [Math]::Min(2000, $output.Length))
    break
}
Write-Host "=== Done ($i iter) ==="
