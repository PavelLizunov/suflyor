# Install the suflyor Hermes plugin into the local Hermes profile.
# Copies integrations/hermes-plugin/suflyor -> ~/.hermes/plugins/suflyor,
# then prints the exact config.yaml snippet + env vars to finish wiring.
#
# Idempotent: re-running overwrites the plugin files (config is NOT touched —
# you edit ~/.hermes/config.yaml yourself, see the printed snippet).

$ErrorActionPreference = 'Stop'

$src = Join-Path $PSScriptRoot 'suflyor'
$hermesHome = if ($env:HERMES_HOME) { $env:HERMES_HOME } else { Join-Path $HOME '.hermes' }
$dstPlugins = Join-Path $hermesHome 'plugins'
$dst = Join-Path $dstPlugins 'suflyor'

if (-not (Test-Path $src)) { throw "plugin source not found: $src" }
New-Item -ItemType Directory -Force -Path $dst | Out-Null
Copy-Item -Path (Join-Path $src '*') -Destination $dst -Recurse -Force
Write-Host "[ok] plugin installed -> $dst" -ForegroundColor Green

$cfg = Join-Path $hermesHome 'config.yaml'
$envFile = Join-Path $hermesHome '.env'
Write-Host ""
Write-Host "STEP 1 — enable the plugin. Edit $cfg (create if missing) so it contains:" -ForegroundColor Cyan
Write-Host @'

plugins:
  enabled:
    - suflyor

'@ -ForegroundColor Gray
Write-Host "STEP 2 — secrets go in ~/.hermes/.env (Hermes loads it into the plugin env)." -ForegroundColor Cyan
Write-Host "  In suflyor: Настройки -> Hermes -> включи «Мост для Hermes», скопируй токен." -ForegroundColor Gray
Write-Host "  Then append to $envFile :" -ForegroundColor Gray
Write-Host @'

SUFLYOR_BRIDGE_URL=http://127.0.0.1:8654
SUFLYOR_BRIDGE_TOKEN=<paste-token-from-suflyor-settings>

'@ -ForegroundColor Gray
Write-Host "STEP 3 — restart Hermes. Verify:" -ForegroundColor Cyan
Write-Host "  hermes plugins list        # suflyor should be enabled"
Write-Host "  /suflyor  (in chat)        # should print 'suflyor подключён'"
