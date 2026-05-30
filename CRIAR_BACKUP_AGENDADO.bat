@echo off
setlocal
cd /d "%~dp0"

set BACKUP_TIME=23:00
if not "%~1"=="" set BACKUP_TIME=%~1

echo.
echo ============================================
echo  Criando backup automatico do Portex PDV
echo ============================================
echo.
echo Horario diario: %BACKUP_TIME%
echo.

schtasks /Create /TN "Portex PDV Backup" /TR "powershell.exe -NoProfile -ExecutionPolicy Bypass -File %~dp0PORTEX_BACKUP.ps1" /SC DAILY /ST %BACKUP_TIME% /F

echo.
echo Se a mensagem acima indicou SUCCESS/SUCESSO, o backup automatico foi criado.
echo Os backups serao salvos em Downloads\portex-pdv-backups.
echo.
pause
