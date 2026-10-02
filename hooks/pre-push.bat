@echo off
REM what-stack - Pre-push quality gate (Windows batch version).
REM The installers copy the POSIX hooks, which Git for Windows runs through sh.
REM This file is for setups that run batch hooks; copy it to .git\hooks\pre-push.

echo what-stack pre-push quality gate

call :ensure_cargo
if %ERRORLEVEL% neq 0 exit /b 1

echo -^> [1/7] Checking formatting...
cargo fmt --all -- --check
if %ERRORLEVEL% neq 0 (
    echo.
    echo FORMATTING FAILED
    echo   Run: cargo fmt
    echo   Then try pushing again.
    exit /b 1
)

echo -^> [2/7] Running cross-target clippy...
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
    echo   then try pushing again.
    exit /b 1
)

echo -^> [3/7] Running tests...
cargo test --locked --lib --tests
if %ERRORLEVEL% neq 0 (
    echo.
    echo TESTS FAILED
    echo   Fix the failing unit or integration tests, then try pushing again.
    exit /b 1
)
cargo test --locked --doc
if %ERRORLEVEL% neq 0 (
    echo.
    echo TESTS FAILED
    echo   Fix the failing doctests, then try pushing again.
    exit /b 1
)

REM Benchmarks only compile here; running them needs Valgrind and
REM gungraun-runner, which Linux CI provides.
echo -^> [4/7] Compiling benchmarks...
cargo bench --locked --no-run
if %ERRORLEVEL% neq 0 (
    echo.
    echo BENCHMARK COMPILE FAILED
    echo   Fix the benchmark build errors, then try pushing again.
    exit /b 1
)

echo -^> [5/7] Building crate...
cargo build --locked
if %ERRORLEVEL% neq 0 (
    echo.
    echo BUILD FAILED
    echo   Fix the build errors, then try pushing again.
    exit /b 1
)

echo -^> [6/7] Building docs...
set "RUSTDOCFLAGS=-D warnings -D rustdoc::bare_urls -D rustdoc::invalid_rust_codeblocks -D rustdoc::private_intra_doc_links -D rustdoc::unescaped_backticks"
cargo doc --locked --no-deps
if %ERRORLEVEL% neq 0 (
    echo.
    echo DOCUMENTATION BUILD FAILED
    echo   Fix the doc errors, then try pushing again.
    exit /b 1
)

echo -^> [7/7] Auditing dependencies...
where cargo-deny >nul 2>nul
if %ERRORLEVEL% equ 0 (
    cargo deny check 2>nul || echo Dependency audit failed locally; CI enforces this gate.
) else (
    echo cargo-deny not installed, skipping local audit.
)

echo All pre-push gates passed.
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
