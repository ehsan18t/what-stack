@echo off
setlocal EnableExtensions
set "commit_msg_file=%~1"

where pwsh >nul 2>nul
if %ERRORLEVEL% equ 0 (
  pwsh -NoProfile -File scripts\Test-CommitMessage.ps1 "%commit_msg_file%"
) else (
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts\Test-CommitMessage.ps1 "%commit_msg_file%"
)

if %ERRORLEVEL% neq 0 (
  echo COMMIT MESSAGE REJECTED
  exit /b 1
)

exit /b 0
