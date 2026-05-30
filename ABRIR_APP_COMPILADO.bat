@echo off
setlocal
cd /d "%~dp0"

set APP_EXE=src-tauri\target\release\portex_pdv.exe

if exist "%APP_EXE%" (
  start "" "%APP_EXE%"
  exit /b 0
)

echo.
echo O aplicativo compilado ainda nao existe.
echo Primeiro execute GERAR_EXE.bat ou rode npm run build.
echo.
pause
