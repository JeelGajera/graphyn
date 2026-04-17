$ErrorActionPreference = 'Stop'

$GITHUB_REPO = "JeelGajera/graphyn"
$TARGET = "x86_64-pc-windows-msvc"
$ASSET_NAME = "graphyn-${TARGET}.zip"
$DOWNLOAD_URL = "https://github.com/${GITHUB_REPO}/releases/latest/download/${ASSET_NAME}"

$INSTALL_DIR = [System.IO.Path]::Combine($env:LOCALAPPDATA, "Programs", "graphyn")
$BIN_DIR = [System.IO.Path]::Combine($INSTALL_DIR, "bin")

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "⚡ Installing Graphyn Code Intelligence Engine" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Detected Platform: Windows (x86_64)"
Write-Host "Target: $TARGET"

# 1. Create temp directory
$TMP_DIR = [System.IO.Path]::Combine([System.IO.Path]::GetTempPath(), [System.Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Force -Path $TMP_DIR | Out-Null

try {
    # 2. Download
    Write-Host "Downloading latest release..."
    $ZIP_PATH = [System.IO.Path]::Combine($TMP_DIR, $ASSET_NAME)
    Invoke-WebRequest -Uri $DOWNLOAD_URL -OutFile $ZIP_PATH -UseBasicParsing

    # 3. Extract
    Write-Host "Extracting binary..."
    Expand-Archive -Path $ZIP_PATH -DestinationPath $TMP_DIR -Force

    # 4. Install
    Write-Host "Installing to $BIN_DIR..."
    New-Item -ItemType Directory -Force -Path $BIN_DIR | Out-Null
    
    $EXISTING_EXE = [System.IO.Path]::Combine($BIN_DIR, "graphyn.exe")
    if (Test-Path $EXISTING_EXE) {
        try {
            $OLD_VERSION = & $EXISTING_EXE --version
            Write-Host "Updating existing installation ($OLD_VERSION)..."
        } catch {
            Write-Host "Updating existing installation..."
        }
    }

    Move-Item -Path [System.IO.Path]::Combine($TMP_DIR, "graphyn.exe") -Destination $EXISTING_EXE -Force

    # 5. Check PATH Setup
    $NEW_VERSION = & $EXISTING_EXE --version
    Write-Host "`n✅ Successfully installed $NEW_VERSION!`n" -ForegroundColor Green

    $USER_PATH = [Environment]::GetEnvironmentVariable("PATH", "User")
    if ($USER_PATH -notlike "*$BIN_DIR*") {
        Write-Host "Adding $BIN_DIR to your User PATH..."
        $NEW_PATH = "$USER_PATH;$BIN_DIR"
        [Environment]::SetEnvironmentVariable("PATH", $NEW_PATH, "User")
        $env:PATH = "$env:PATH;$BIN_DIR"
        Write-Host "PATH updated successfully. You may need to restart your terminal." -ForegroundColor Yellow
    } else {
        Write-Host "Graphyn is ready to use! Run 'graphyn --help' to get started." -ForegroundColor Green
    }
    
} finally {
    # Cleanup
    if (Test-Path $TMP_DIR) {
        Remove-Item -Path $TMP_DIR -Recurse -Force
    }
}
Write-Host "==========================================" -ForegroundColor Cyan
