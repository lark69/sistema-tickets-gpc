@echo off
setlocal
cd /d "%~dp0"

echo.
echo ============================================
echo  Abrindo Sistema de Tickets GPC em modo dev
echo ============================================
echo.

npm run dev

pause
