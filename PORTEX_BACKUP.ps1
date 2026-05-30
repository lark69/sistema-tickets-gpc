$ErrorActionPreference = "Stop"

$appDataDir = Join-Path $env:APPDATA "com.portex.pdv"
$databasePath = Join-Path $appDataDir "portex_pdv.sqlite"
$backupDir = Join-Path $env:USERPROFILE "Downloads\portex-pdv-backups"

if (-not (Test-Path -LiteralPath $databasePath)) {
  throw "Banco de dados nao encontrado em: $databasePath"
}

New-Item -ItemType Directory -Force -Path $backupDir | Out-Null

$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$destination = Join-Path $backupDir "portex-pdv-backup-$timestamp.sqlite"

Copy-Item -LiteralPath $databasePath -Destination $destination -Force
Write-Host "Backup salvo em: $destination"
