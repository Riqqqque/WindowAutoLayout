param(
    [switch]$SkipBuild,
    [switch]$NoLaunchSmoke,
    [string]$InstallerPath
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$bundleRoot = Join-Path $repoRoot "src-tauri\target\release\bundle"
$productName = "WindowAutoLayout"

function Get-UninstallEntry {
    $roots = @(
        "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*",
        "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*",
        "HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*"
    )

    foreach ($root in $roots) {
        Get-ItemProperty $root -ErrorAction SilentlyContinue |
            Where-Object { $_.DisplayName -eq $productName } |
            Select-Object -First 1
    }
}

function Get-InstalledExe {
    param($Entry)

    function Normalize-InstallPath {
        param([string]$Path)
        if (-not $Path) {
            return $null
        }
        ($Path.Trim() -replace '^"', '') -replace '"$', ''
    }

    $candidates = @()
    if ($Entry.InstallLocation) {
        $installLocation = Normalize-InstallPath $Entry.InstallLocation
        $candidates += Join-Path $installLocation "$productName.exe"
        $candidates += Join-Path $installLocation "windowautolayout.exe"
    }
    if ($Entry.DisplayIcon) {
        $displayIcon = Normalize-InstallPath (($Entry.DisplayIcon -split ",")[0])
        $candidates += $displayIcon
    }
    $candidates += Join-Path $env:LOCALAPPDATA "Programs\$productName\$productName.exe"
    $candidates += Join-Path $env:LOCALAPPDATA "$productName\$productName.exe"
    $candidates += Join-Path $env:ProgramFiles "$productName\$productName.exe"
    $candidates += Join-Path ${env:ProgramFiles(x86)} "$productName\$productName.exe"

    $candidates |
        Where-Object { $_ -and (Test-Path -LiteralPath $_) } |
        Select-Object -First 1
}

Push-Location $repoRoot
try {
    if (-not $SkipBuild) {
        npm run desktop:build
    }

    if ($InstallerPath) {
        $installer = Get-Item -LiteralPath $InstallerPath
    } else {
        $installer = Get-ChildItem -LiteralPath $bundleRoot -Recurse -File |
            Where-Object { $_.Name -like "WindowAutoLayout*_setup.exe" -or $_.Name -like "WindowAutoLayout*setup.exe" -or $_.Extension -eq ".msi" } |
            Sort-Object LastWriteTime -Descending |
            Select-Object -First 1
    }

    if (-not $installer) {
        throw "No WindowAutoLayout installer was found under $bundleRoot"
    }

    Get-Process -Name "WindowAutoLayout", "windowautolayout" -ErrorAction SilentlyContinue |
        Stop-Process -Force -ErrorAction SilentlyContinue

    $hash = Get-FileHash -Algorithm SHA256 -LiteralPath $installer.FullName
    if ($installer.Extension -eq ".msi") {
        $process = Start-Process -FilePath "msiexec.exe" -ArgumentList @("/i", $installer.FullName, "/qn", "/norestart") -Wait -PassThru
    } else {
        $process = Start-Process -FilePath $installer.FullName -ArgumentList "/S" -Wait -PassThru
    }

    if ($process.ExitCode -ne 0) {
        throw "Installer exited with code $($process.ExitCode)"
    }

    Start-Sleep -Seconds 2
    $entry = Get-UninstallEntry | Select-Object -First 1
    if (-not $entry) {
        throw "Install completed, but no WindowAutoLayout uninstall entry was found"
    }

    $exe = Get-InstalledExe -Entry $entry
    if (-not $exe) {
        throw "Install completed, but the installed WindowAutoLayout exe was not found"
    }

    $stayedAlive = $null
    if (-not $NoLaunchSmoke) {
        $app = Start-Process -FilePath $exe -PassThru -WindowStyle Hidden
        Start-Sleep -Seconds 5
        $stayedAlive = -not $app.HasExited
        if ($stayedAlive) {
            $app.CloseMainWindow() | Out-Null
            Start-Sleep -Seconds 2
            if (-not $app.HasExited) {
                Stop-Process -Id $app.Id -Force
            }
        }
        if (-not $stayedAlive) {
            throw "Installed app exited during launch smoke"
        }
    }

    [pscustomobject]@{
        Product = $productName
        Version = $entry.DisplayVersion
        Installer = $installer.FullName
        InstallerSha256 = $hash.Hash
        InstalledExe = $exe
        LaunchSmokeStayedAlive = $stayedAlive
    } | ConvertTo-Json
}
finally {
    Pop-Location
}
