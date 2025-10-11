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

 echo Please run the following command:
 echo ---
 echo .\target\release\bcc.exe aot ".\examples\%~1.basil" --dep-source local --local-runtime "%RUNTIME%"
 echo ---

.\target\release\bcc.exe aot ".\examples\%~1.basil" --dep-source local --local-runtime "%RUNTIME%"
REM If build failed (non-zero exit code), say so and stop
if errorlevel 1 (
  echo The Build Failed with errorlevel %errorlevel%.  Please examine the output above and rectify any issues. This program runs okay under the basilc interpreter

  exit /b 1
)

REM --- On success, run the produced EXE
".\%~1.exe"

endlocal