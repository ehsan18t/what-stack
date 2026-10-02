@echo off
REM what-stack - Conventional Commits validation (Windows batch version).
REM The installers copy the POSIX hook, which Git for Windows runs through sh.
REM This file is for setups that run batch hooks; copy it to .git\hooks\commit-msg.
setlocal EnableExtensions
set "commit_msg_file=%~1"

REM scripts\Test-CommitMessage.ps1 does the matching; batch has no regex.
where pwsh >nul 2>nul
if %ERRORLEVEL% equ 0 (
  pwsh -NoProfile -File scripts\Test-CommitMessage.ps1 "%commit_msg_file%"
) else (
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts\Test-CommitMessage.ps1 "%commit_msg_file%"
)
set "result=%ERRORLEVEL%"

if "%result%"=="0" exit /b 0
if "%result%"=="2" goto too_short
if "%result%"=="3" goto too_long
if "%result%"=="4" goto ends_with_period

set /p subject=<"%commit_msg_file%"
echo.
echo COMMIT MESSAGE REJECTED
echo   Your message: "%subject%"
echo   Expected: ^<type^>(^<scope^>): ^<description^>
echo   Types: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert, enforce
echo   The description starts with a lowercase letter; the scope is optional.
exit /b 1

:too_short
echo.
echo COMMIT MESSAGE REJECTED
echo   The commit description must be at least 5 characters.
exit /b 1

:too_long
echo.
echo COMMIT MESSAGE REJECTED
echo   The commit description must be at most 200 characters.
exit /b 1

:ends_with_period
echo.
echo COMMIT MESSAGE REJECTED
echo   The commit subject must not end with a period.
exit /b 1
