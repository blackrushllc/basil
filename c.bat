@echo off
setlocal

REM --- Config (adjust path if needed)
set "RUNTIME=E:\Projects\Yore\basil"

REM --- Require an argument
if "%~1"=="" (
  echo Usage: %~n0 programNameWithoutExtension
  exit /b 2
)

REM --- Build with bcc
.\target\release\bcc.exe aot ".\examples\%~1.basil" --dep-source local --local-runtime "%RUNTIME%"
REM If build failed (non-zero exit code), say so and stop
if errorlevel 1 (
  echo bcc did not work
  exit /b 1
)

REM --- On success, run the produced EXE
".\%~1.exe"

endlocal