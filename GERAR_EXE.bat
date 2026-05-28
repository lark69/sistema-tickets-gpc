@echo off
setlocal
cd /d "%~dp0"

echo.
echo ============================================
echo  Gerando instalador do Sistema de Tickets GPC
echo ============================================
echo.

npm run build

echo.
echo Se a compilacao terminou sem erro, veja o instalador em:
echo src-tauri\target\release\bundle\nsis
echo.
pause
