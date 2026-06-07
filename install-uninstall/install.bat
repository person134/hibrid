@echo off
setlocal enabledelayedexpansion

set BINARY_NAME=hibrid.exe
set SCRIPT_DIR=%~dp0
set PROJECT_DIR=%SCRIPT_DIR%..
set SOURCE_PATH=%PROJECT_DIR%\target\release\%BINARY_NAME%
set DEST_DIR=%SystemRoot%\System32
set DEST_PATH=%DEST_DIR%\%BINARY_NAME%

echo === Hibrid Installer / Uninstaller (Windows) ===
echo.

:choose_action
echo Select action:
echo   1) Install
echo   2) Uninstall
set /p ACTION="Choice [1/2]: "
echo.

if "%ACTION%"=="1" goto install
if "%ACTION%"=="2" goto uninstall
echo Invalid choice
goto choose_action

:install
if exist "%DEST_PATH%" (
    echo Hibrid is already installed at %DEST_PATH%
    exit /b 0
)

where cargo >nul 2>nul
if %errorLevel% neq 0 (
    echo Rust/Cargo not found. Installing Rust via winget...
    where winget >nul 2>nul
    if !errorLevel! neq 0 (
        echo Error: winget not found. Install Rust from https://rustup.rs first.
        exit /b 1
    )
    winget install Rustlang.Rustup
    echo Please restart your terminal after Rust installs, then re-run this script.
    exit /b 0
)

echo Building %BINARY_NAME%...
if not exist "%PROJECT_DIR%\Cargo.toml" (
    echo Error: Cargo.toml not found at %PROJECT_DIR%\Cargo.toml
    exit /b 1
)

pushd "%PROJECT_DIR%"
call cargo build --release
popd

if not exist "%SOURCE_PATH%" (
    echo Error: Build failed - binary not found at %SOURCE_PATH%
    exit /b 1
)

echo Installing to %DEST_PATH%...
net session >nul 2>nul
if %errorLevel% neq 0 (
    echo Error: This installer must be run as Administrator.
    echo Right-click install.bat and select "Run as administrator".
    exit /b 1
)

copy /Y "%SOURCE_PATH%" "%DEST_PATH%"
echo Done! Hibrid has been installed to %DEST_PATH%
echo You can now run 'hibrid' from anywhere.
exit /b 0

:uninstall
if not exist "%DEST_PATH%" (
    echo Hibrid is not installed.
    exit /b 0
)

net session >nul 2>nul
if %errorLevel% neq 0 (
    echo Error: This uninstaller must be run as Administrator.
    echo Right-click install.bat and select "Run as administrator".
    exit /b 1
)

echo Removing %DEST_PATH%...
del /F "%DEST_PATH%"
echo Done! Hibrid has been uninstalled.
endlocal
