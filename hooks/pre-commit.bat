@echo off
REM what-stack - Pre-commit quality gate (Windows batch version).
REM The installers copy the POSIX hooks, which Git for Windows runs through sh.
REM This file is for setups that run batch hooks; copy it to .git\hooks\pre-commit.

echo what-stack pre-commit quality gate

call :ensure_cargo
if %ERRORLEVEL% neq 0 exit /b 1

echo -^> [1/3] Checking formatting...
cargo fmt --all -- --check
if %ERRORLEVEL% neq 0 (
    echo.
    echo FORMATTING FAILED
    echo   Run: cargo fmt
    echo   Then try committing again.
    exit /b 1
)

echo -^> [2/3] Running cross-target clippy...
where pwsh >nul 2>nul
if %ERRORLEVEL% equ 0 (
    pwsh -NoProfile -File scripts\check-platform-clippy.ps1
) else (
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\check-platform-clippy.ps1
)
if %ERRORLEVEL% neq 0 (
    echo.
    echo CLIPPY FAILED
    echo   Fix the lint errors above or install the missing rustup targets,
    echo   then try committing again.
    exit /b 1
)

echo -^> [3/3] Running tests...
cargo test --locked --lib --tests
if %ERRORLEVEL% neq 0 (
    echo.
    echo TESTS FAILED
    echo   Fix the failing unit or integration tests, then try committing again.
    exit /b 1
)
cargo test --locked --doc
if %ERRORLEVEL% neq 0 (
    echo.
    echo TESTS FAILED
    echo   Fix the failing doctests, then try committing again.
    exit /b 1
)

echo All pre-commit gates passed.
exit /b 0

REM Git can run hooks with a reduced PATH that lacks the rustup shims.
:ensure_cargo
where cargo >nul 2>nul
if %ERRORLEVEL% equ 0 exit /b 0

if exist "%USERPROFILE%\.cargo\bin\cargo.exe" (
    set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
)

where cargo >nul 2>nul
if %ERRORLEVEL% equ 0 exit /b 0

echo.
echo RUST TOOLCHAIN NOT FOUND
echo   cargo is not available in the git hook environment.
echo   Install Rust or add %%USERPROFILE%%\.cargo\bin to PATH, then try again.
exit /b 1
