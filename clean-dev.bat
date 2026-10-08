@echo off
setlocal
cd /d "%~dp0"

rem Usage: clean-dev.bat [/y] [/all]
rem   /y    skip confirmation
rem   /all  also run "cargo clean" (removes the target directory)
set "ASSUME_YES="
set "CLEAN_ALL="
:parse
if "%~1"=="" goto parsed
if /i "%~1"=="/y" set "ASSUME_YES=1"
if /i "%~1"=="/all" set "CLEAN_ALL=1"
shift
goto parse
:parsed

rem A running server holds data\app.db open
powershell -NoProfile -Command "if (Get-NetTCPConnection -LocalPort 3000 -State Listen -ErrorAction SilentlyContinue) { exit 1 }"
if errorlevel 1 (
    echo ERROR: something is listening on port 3000. Stop the dev server first.
    exit /b 1
)

echo This will delete: .env and data\ (database)
if defined CLEAN_ALL echo Also: target\ (cargo clean)
if defined ASSUME_YES goto confirmed
set /p "CONFIRM=Continue? [y/N] "
if /i not "%CONFIRM%"=="y" (
    echo Aborted.
    exit /b 0
)
:confirmed

if exist ".env" del /q ".env" && echo Removed .env
if exist "data" rmdir /s /q "data" && echo Removed data\

if defined CLEAN_ALL (
    where cargo >nul 2>nul
    if errorlevel 1 (
        echo ERROR: cargo not found on PATH; skipping cargo clean.
        exit /b 1
    )
    cargo clean
)

echo Done.
