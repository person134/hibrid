@echo off
setlocal

set BINARY_NAME=hibrid.exe
set DEST_PATH=%SystemRoot%\System32\%BINARY_NAME%

echo Hibrid Uninstaller
echo ==================

if not exist "%DEST_PATH%" (
    echo Error: Hibrid is not installed at %DEST_PATH%
    exit /b 1
)

net session >nul 2>&1
if %errorLevel% neq 0 (
    echo Error: This uninstaller must be run as Administrator.
    echo Right-click uninstall.bat and select "Run as administrator".
    exit /b 1
)

echo Removing %DEST_PATH%...

del /F "%DEST_PATH%"
if %errorLevel% neq 0 (
    echo Error: Failed to remove binary.
    exit /b 1
)

echo Done! Hibrid has been uninstalled.

endlocal
