@echo off
setlocal
"%~dp0study-coach-app.exe" %*
if errorlevel 1 (
  echo Study Coach stopped. Read the message above for the next step.
  pause
)
