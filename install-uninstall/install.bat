@echo off
setlocal

set NAME=hibrid.exe
set DEST=%SystemRoot%\System32\

if "%1"=="-u" goto uninstall
if "%1"=="--uninstall" goto uninstall

cargo build --release
if %errorlevel% neq 0 exit /b %errorlevel%
copy /Y "target\release\%NAME%" "%DEST%"
echo Installed %NAME% to %DEST%
exit /b 0

:uninstall
del /F "%DEST%%NAME%" 2>nul
echo Uninstalled %NAME%
