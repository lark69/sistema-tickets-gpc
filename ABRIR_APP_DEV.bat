@echo off
setlocal
cd /d "%~dp0"

echo.
echo ============================================
echo  Abrindo Portex PDV em modo dev
echo ============================================
echo.

npm run dev

pause
