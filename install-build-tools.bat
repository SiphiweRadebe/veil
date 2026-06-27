@echo off
REM Quick installer for Visual Studio Build Tools
REM This enables Rust compilation on Windows

echo.
echo ============================================
echo Veil Build Tools Installer
echo ============================================
echo.
echo This will download and install Visual Studio Build Tools
echo Required for compiling Rust on Windows
echo.
pause

REM Check if Build Tools already installed
where link.exe >nul 2>&1
if %errorlevel% equ 0 (
    echo Build Tools already detected. Skipping installation.
    goto build
)

REM Download Build Tools
echo Downloading Visual Studio Build Tools...
powershell -Command ^
  "Invoke-WebRequest -Uri 'https://aka.ms/vs/17/release/vs_buildtools.exe' -OutFile '%TEMP%\vs_buildtools.exe'" ^
  || goto error

REM Install Build Tools
echo Installing Build Tools (this may take 10-20 minutes)...
"%TEMP%\vs_buildtools.exe" ^
  --norestart ^
  --quiet ^
  --wait ^
  --add Microsoft.VisualStudio.Workload.VCTools ^
  --add Microsoft.VisualStudio.Component.Windows10SDK.19041 ^
  || goto error

echo Build Tools installed successfully!

REM Try to build
:build
echo.
echo Attempting to build veil...
cargo build --release
if %errorlevel% equ 0 (
    echo.
    echo ============================================
    echo Build successful!
    echo Executable: target\release\veil.exe
    echo ============================================
    pause
    exit /b 0
)

:error
echo.
echo Installation failed or build encountered an error.
echo Please visit: https://visualstudio.microsoft.com/downloads/
echo And install "Build Tools for Visual Studio 2022"
echo Select "Desktop development with C++" workload
pause
exit /b 1
