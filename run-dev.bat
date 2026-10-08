@echo off
setlocal
cd /d "%~dp0"

rem Usage: run-dev.bat [admin-email]
set "ADMIN_EMAIL=%~1"
if "%ADMIN_EMAIL%"=="" set "ADMIN_EMAIL=admin@example.com"

where cargo >nul 2>nul
if errorlevel 1 (
    echo ERROR: cargo not found on PATH. Install Rust from https://rustup.rs
    exit /b 1
)

if not exist ".env" (
    copy ".env.example" ".env" >nul
    echo Created .env from .env.example - change JWT_SECRET before any non-local use.
)

if not exist "data\app.db" (
    if not exist "data" mkdir "data"
    echo Initializing database with admin %ADMIN_EMAIL% ...
    cargo run -- init-db --admin-email %ADMIN_EMAIL%
    if errorlevel 1 (
        echo ERROR: init-db failed.
        exit /b 1
    )
)

echo.
echo Starting server: http://127.0.0.1:3000/login.html  (Ctrl+C to stop)
cargo run -- serve
