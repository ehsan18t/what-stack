@echo off
echo what-stack pre-commit quality gate

echo -^> Checking formatting...
cargo fmt --all -- --check
if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%

echo -^> Running cross-target clippy...
where pwsh >nul 2>nul
if %ERRORLEVEL% equ 0 (
  pwsh -NoProfile -File scripts\check-platform-clippy.ps1
) else (
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts\check-platform-clippy.ps1
)
if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%

echo -^> Running tests...

cargo test --locked --lib --tests
if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%
cargo test --locked --doc

if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%

echo All pre-commit gates passed.
exit /b 0
