@echo off
echo what-stack pre-push quality gate

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


echo -^> Compiling benchmarks...
cargo bench --locked --no-run
if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%


echo -^> Building crate...
cargo build --locked
if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%

echo -^> Building docs...
set "RUSTDOCFLAGS=-D warnings -D rustdoc::bare_urls -D rustdoc::invalid_rust_codeblocks -D rustdoc::private_intra_doc_links -D rustdoc::unescaped_backticks"
cargo doc --locked --no-deps
if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%

echo -^> Auditing dependencies...
where cargo-deny >nul 2>nul
if %ERRORLEVEL% equ 0 (
  cargo deny check 2>nul || echo Dependency audit failed locally; CI enforces this gate.
) else (
  echo cargo-deny not installed, skipping local audit.
)

echo All pre-push gates passed.
exit /b 0
