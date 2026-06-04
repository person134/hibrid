@echo off
setlocal

set BINARY_NAME=hibrid.exe
set SOURCE_PATH=%USERPROFILE%\hibrid\target\release\%BINARY_NAME%
set DEST_DIR=%SystemRoot%\System32

echo Hibrid Installer
echo ================

if not exist "%SOURCE_PATH%" (
    echo Error: Binary not found at %SOURCE_PATH%
    echo Make sure you have run 'cargo build --release' first.
    exit /b 1
)

echo Installing %BINARY_NAME% to %DEST_DIR%...

net session >nul 2>&1
if %errorLevel% neq 0 (
    echo Error: This installer must be run as Administrator.
    echo Right-click install.bat and select "Run as administrator".
    exit /b 1
)

copy /Y "%SOURCE_PATH%" "%DEST_DIR%\%BINARY_NAME%"
if %errorLevel% neq 0 (
    echo Error: Failed to copy binary.
    exit /b 1
)

echo Done! Hibrid has been installed to %DEST_DIR%
echo You can now run 'hibrid' from anywhere.

endlocal
