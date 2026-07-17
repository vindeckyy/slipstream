@echo off
rem slipstream plugin/script runner launcher - the action the SlipstreamScripting scheduled task runs.
rem
rem OPT-IN: the installer registers that task DISABLED (the runner is inert until you add automation).
rem Enable it once you have scripts/plugins:  Enable-ScheduledTask -TaskName SlipstreamScripting
rem
rem Lays out next to the installed payload: {app}\scripting\scripting-run.cmd + runner-cli.js and
rem {app}\bun\bun.exe (so %~dp0 = {app}\scripting\). The runner discovers the operator's units under
rem %ProgramData%\slipstream\{scripts,plugins}; a plugin's connect() auto-wires to the host's mgmt
rem token + identity cert in %ProgramData%\slipstream\ (written by the host's `serve`). No env editing.
setlocal EnableExtensions

set "BUN=%~dp0..\bun\bun.exe"
set "RUNNER=%~dp0runner-cli.js"

if not exist "%RUNNER%" (
  echo [slipstream-scripting] runner bundle missing at "%RUNNER%".
  exit /b 1
)
if not exist "%BUN%" (
  echo [slipstream-scripting] bundled bun missing at "%BUN%".
  exit /b 1
)

rem The runner import()s the operator's .ts plugin files, so it runs on the bundled bun. SIGTERM (task
rem End) interrupts the whole unit tree structurally so plugin finalizers run before exit.
"%BUN%" "%RUNNER%"
