# msix.ps1 — build SnapShelf, pack per-arch MSIX, and bundle for the Microsoft Store.
#
# Usage:
#   pwsh msix.ps1                      # build all archs SERIALLY + bundle (needs MSVC ARM64 tools)
#   pwsh msix.ps1 -Arch x64            # build + stage + pack a single arch
#   pwsh msix.ps1 -Bundle              # bundle whatever .msix files already sit in msix_drop\<ver>\
#   pwsh msix.ps1 -Arch x64 -SkipBuild # pack from an existing target\<triple>\release build
#   pwsh msix.ps1 -Bundle -CertPath dev.pfx -CertPassword pw   # local sideload test (Store re-signs)
#
# For the Store: upload the UNSIGNED output\snapshelf_<version>.msixbundle — Partner Center signs
# it. The Identity Name/Publisher + PublisherDisplayName in Package.appxmanifest MUST match your
# Partner Center registration, or the upload is rejected.

param (
    [ValidateSet("", "x64", "arm64")]
    [string]$Arch = "",
    [switch]$Bundle,
    [switch]$SkipBuild,
    [switch]$SkipFrontend,   # frontend already built once by the serial driver; skip rebuilding here
    [string]$CertPath = "",
    [string]$CertPassword = "password"
)

$ErrorActionPreference = "Stop"

# Version = single source of truth (package.json), stripped to the X.Y.Z core. The MSIX Identity
# version is this + ".0" (4-part, required by appx).
$pkgVersion = (Get-Content "package.json" -Raw | ConvertFrom-Json).version
$Version    = ([regex]::Match($pkgVersion, '^\d+\.\d+\.\d+')).Value
if (-not $Version) { throw "Bad version in package.json: '$pkgVersion' (expected X.Y.Z)" }

# Arch -> Rust target triple
$Triples = @{
    "x64"   = "x86_64-pc-windows-msvc"
    "arm64" = "aarch64-pc-windows-msvc"
}

$StagingRoot = "winapp-layout"
$DropDir     = "msix_drop\$Version"
$OutputDir   = "output"
$Manifest    = "Package.appxmanifest"
$IconsDir    = "src-tauri\icons"
$BundleOut   = "$OutputDir\snapshelf_$Version.msixbundle"

# MSIX logos the manifest references by relative path (icons\<name>). Tauri-generated Store logos —
# regenerate with `pnpm tauri icon`. Copied into <layout>\icons at pack time by their original names.
$Logos = @("50x50.png", "44x44.png", "150x150.png")

# Emit a manifest whose Identity Version = package.json version (with a .0 revision), so the packaged
# Store version always tracks package.json. Written OUTSIDE the payload layout (passed via --manifest)
# so it is never double-included as a payload file. Returns the generated manifest path.
function New-VersionedManifest {
    param([string]$A)
    if (-not (Test-Path $StagingRoot)) { New-Item -ItemType Directory -Force $StagingRoot | Out-Null }
    [xml]$doc = Get-Content $Manifest -Raw -Encoding UTF8
    $ns = New-Object Xml.XmlNamespaceManager($doc.NameTable)
    $ns.AddNamespace("pkg", "http://schemas.microsoft.com/appx/manifest/foundation/windows10")
    $doc.SelectSingleNode("/pkg:Package/pkg:Identity", $ns).SetAttribute("Version", "$Version.0")
    $out = "$StagingRoot\AppxManifest.$A.xml"
    $doc.Save($out)
    return $out
}

function Pack-Arch {
    param([string]$A)

    $triple     = $Triples[$A]
    # Default cargo/tauri target dir — serial builds, so no per-arch isolation needed.
    $releaseDir = "src-tauri\target\$triple\release"
    $exe        = "$releaseDir\snap-shelf.exe"
    $layout     = "$StagingRoot\$A"

    if (-not $SkipBuild) {
        if (-not $SkipFrontend) {
            Write-Host "==> Building frontend..."
            pnpm build
            if ($LASTEXITCODE -ne 0) { throw "frontend build failed" }
        }

        Write-Host "==> Building $A host ($triple)..."
        # --no-bundle: we pack the MSIX ourselves below; Tauri's NSIS bundler is not used here.
        pnpm tauri build --target $triple --no-bundle
        if ($LASTEXITCODE -ne 0) { throw "tauri build failed for $A" }
    }

    if (-not (Test-Path $exe)) { throw "Missing build output: $exe" }

    Write-Host "==> Staging $A into $layout..."
    if (Test-Path $layout) { Remove-Item -Recurse -Force $layout }
    New-Item -ItemType Directory -Force $layout | Out-Null

    Copy-Item $exe -Destination $layout
    # Copy any sibling DLLs Tauri staged next to the exe (none today; future-proof, no per-file naming).
    Get-ChildItem $releaseDir -Filter *.dll -ErrorAction SilentlyContinue | Copy-Item -Destination $layout
    # MSIX logos must physically live in the layout — the manifest names them by relative path
    # (icons\<name>). Copy the tauri-generated Store logos in by their original names.
    $layoutIcons = "$layout\icons"
    New-Item -ItemType Directory -Force $layoutIcons | Out-Null
    foreach ($logo in $Logos) {
        $src = Join-Path $IconsDir $logo
        if (-not (Test-Path $src)) { throw "Missing logo: $src (run 'pnpm tauri icon')" }
        Copy-Item $src -Destination $layoutIcons
    }

    $manifestPath = New-VersionedManifest $A

    Write-Host "==> Packing $A MSIX (v$Version.0)..."
    # Trailing backslash = output directory; winapp auto-names <Identity>_<ver>_<arch>.msix.
    winapp package $layout --manifest $manifestPath --output "$DropDir\"
    if ($LASTEXITCODE -ne 0) { throw "winapp package failed for $A" }
}

function New-Bundle {
    if (! (Test-Path $OutputDir)) { New-Item -ItemType Directory $OutputDir | Out-Null }
    if (Test-Path $BundleOut) { Remove-Item -Force $BundleOut }

    $payloads = Get-ChildItem "$DropDir\*.msix"
    if (-not $payloads) { throw "No .msix payloads in $DropDir to bundle" }

    Write-Host "==> Bundling MSIXBundle..."
    winapp tool makeappx bundle /d $DropDir /p $BundleOut
    if ($LASTEXITCODE -ne 0) { throw "makeappx bundle failed" }

    if ($CertPath -and (Test-Path $CertPath)) {
        Write-Host "==> Signing bundle (local sideload test only — the Store re-signs)..."
        winapp sign $BundleOut $CertPath --password $CertPassword
        if ($LASTEXITCODE -ne 0) { throw "winapp sign failed" }
    }

    Write-Host "Done -> $BundleOut"
}

# --- main ---------------------------------------------------------------------

if (! (Test-Path $DropDir)) { New-Item -ItemType Directory $DropDir | Out-Null }

if ($Bundle -and -not $Arch) {
    # Bundle-only: bundle existing msix_drop\<ver>\*.appx.
    New-Bundle
}
elseif ($Arch) {
    # Single arch. No bundling here.
    Pack-Arch $Arch
}
else {
    # Local convenience: build every arch SERIALLY, then bundle. Cleans the drop first.
    Remove-Item -Recurse -Force $DropDir -ErrorAction SilentlyContinue
    New-Item -ItemType Directory $DropDir | Out-Null

    # Frontend ONCE up front (arch-independent); each arch pack then runs with -SkipFrontend.
    Write-Host "==> Building frontend (once)..."
    pnpm build
    if ($LASTEXITCODE -ne 0) { throw "frontend build failed" }

    $SkipFrontend = $true
    foreach ($a in $Triples.Keys) {
        Pack-Arch $a
    }

    New-Bundle
}
