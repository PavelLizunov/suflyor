[CmdletBinding()]
param(
    [ValidateRange(1, 8)]
    [int]$MaxParallel = 3,
    [string]$OutputDirectory = (Join-Path ([IO.Path]::GetTempPath()) (
        "suflyor-qwen-audit-{0}" -f (Get-Date -Format "yyyyMMdd-HHmmss")
    ))
)

$ErrorActionPreference = "Stop"
$repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$qwen = (Get-Command qwen -ErrorAction Stop).Source
$model = "qwen3.8-max-preview"

$common = @'
Проведи read-only аудит текущего checkout Suflyor. Сначала полностью прочитай
AGENTS.md, затем проверяй только назначенный ниже домен. Соседние файлы читай
только для трассировки границ и вызывающих функций.

Жёсткие ограничения:
- ничего не редактируй и не создавай;
- не запускай сборку, тесты, приложение или сетевые запросы;
- не читай .git, .claude, .codex, .agents и любые файлы вне checkout;
- не читай живой %APPDATA%\suflyor\config.json и другие секреты;
- не ищи стиль, микрооптимизации и гипотетические улучшения;
- не повторяй проблему без точной цепочки отказа в текущем коде;
- уложись максимум в 45 вызовов read-инструментов;
- максимум 8 findings, сначала самые серьёзные.

Верни только JSON без markdown-ограждения:
{
  "verdict": "PASS|WARN|FAIL",
  "findings": [{
    "severity": "critical|high|medium|low",
    "location": "path:line",
    "failure_chain": "как реально возникает отказ",
    "evidence": "код, тест или точная цепочка вызовов",
    "minimal_fix": "минимальное исправление корня"
  }],
  "verified": ["что проверено положительно и чем подтверждено"],
  "coverage_gaps": ["что нельзя доказать статически"]
}
PASS допустим только с положительными доказательствами.
'@

$audits = @(
    [pscustomobject]@{
        Id = "01-config-settings-update"
        Domain = @'
Конфиг, Settings shell, диагностика и обновления.
Scope: overlay-backend/src/config*, components.rs, paths.rs, download.rs,
update.rs; settings_controller.rs, settings_import_export.rs,
settings_updates.rs, diagnostics.rs, wizard.rs, logging/http_log.
Проверь legacy/malformed config, reset transient-status, утечки секретов в
export/report/log collection, SHA/trusted URL, UI-thread blocking и тесты
config/guard/update.
'@
    },
    [pscustomobject]@{
        Id = "02-audio-stt-recording"
        Domain = @'
Аудиозахват, STT и запись.
Scope: audio.rs, recorder.rs, stt.rs, session_audio.rs, STT/audio-части
slint_session.rs и settings_stt.rs.
Проверь WASAPI lifecycle, mic guard, resampling/VAD, provider dispatch,
retry/cancel, generic errors, start/stop cleanup, retention и соответствующие
тесты.
'@
    },
    [pscustomobject]@{
        Id = "03-ai-bridge-local-hermes"
        Domain = @'
AI, bridge, local AI и Hermes.
Scope: ai.rs, bridge.rs, local_ai.rs и local_ai/tests.rs, AI-части runtime.rs,
health.rs, events.rs, hermes_install.rs, settings_ai.rs, settings_local_ai.rs,
settings_hermes.rs.
Проверь маршрутизацию через ai_endpoint, cancellation/races stream, prompt
bounds, cost accounting, process/install transactionality, archive traversal,
generic UI errors и существующие AI/runtime/local-AI/Hermes tests.
'@
    },
    [pscustomobject]@{
        Id = "04-ask-tile-conversation"
        Domain = @'
Ask/tile/conversation.
Scope: все slint-experiment/src/bin/overlay_host/tile_*.rs, ask-dispatch в
overlay_host.rs, markdown.rs и slint_events.rs.
Проверь F3/F6/F9/Shift+F9 route matrix, PTT watchdog, wrong-tile generation
guard, follow-up/regenerate/escalate, soft cost cap, copy semantics, markdown
safety и связанные tests.
'@
    },
    [pscustomobject]@{
        Id = "05-win32-windows-hotkeys"
        Domain = @'
Win32, окна и глобальный ввод.
Scope: win32.rs, window_lifecycle.rs, hotkeys.rs, kbd_shortcuts.rs, оконная
часть aux_windows.rs и bootstrap overlay_host.rs.
Проверь UI-thread ownership, stealth до первого кадра, registry cleanup,
focus/toggle, timers, hotkey conflicts, work-area/multi-monitor placement,
Alt-Tab/taskbar leakage, Win32 tests и необходимые manual-smoke gaps.
'@
    },
    [pscustomobject]@{
        Id = "06-vision-ocr-tts"
        Domain = @'
Vision, OCR и read-aloud.
Scope: vision.rs, ocr*.rs, tts*.rs, capture.rs, vision_capture.rs,
settings_vision.rs, settings_voice.rs и suflyor-tts/{engine,playback,main}.rs
без diarization.
Проверь F8/Shift/Ctrl routes, image bounds/data URL, OCR verbatim,
neural-vs-SAPI normalization, sidecar protocol/cancel/pause/speed, safe install
и vision/OCR/TTS tests.
'@
    },
    [pscustomobject]@{
        Id = "07-journal-archive-recovery"
        Domain = @'
Журнал, каталог, архив и recovery.
Scope: journal.rs, persistence/*, session_admin.rs, session_names.rs,
summary_source.rs, recovery.rs, transcript_player.rs и archive/replay
orchestration.
Проверь JSONL source-of-truth vs SQLite projection, migrations, corrupt
fallback, FTS escaping RU/EN, delete/privacy, active-session reindex, retention,
crash recovery, audio seek/speed, archive_cycle и persistence tests.
'@
    },
    [pscustomobject]@{
        Id = "08-diarization-retranscribe"
        Domain = @'
Diarization и re-transcribe.
Scope: diarize.rs, diar_install.rs, re_transcribe.rs, suflyor-tts/src/diar.rs
и speaker/transcript integration.
Проверь sidecar JSON contract, model validation, auto speaker sweep, post-merge
sorted/non-overlap, max-overlap alignment, phantom filtering, atomic
persistence/error recovery и оба набора diarization tests.
'@
    },
    [pscustomobject]@{
        Id = "09-kb-memory-context"
        Domain = @'
KB, память, профили и контекст.
Scope: kb.rs, memory/*, conspect.rs, config/snippets.rs, settings_memory.rs;
только context-builder вызовы в runtime/tile.
Проверь query bounds/ranking, profile isolation, candidate
dedup/approve/reject/delete, prompt-budget math, normalization, summary
sanitization, graceful DB failure, UI-thread blocking и memory/KB/conspect
tests.
'@
    },
    [pscustomobject]@{
        Id = "10-slint-ui"
        Domain = @'
Декларативный Slint UI.
Scope: все slint-experiment/ui/*.slint, theme/metrics/controls, SVG icons,
ru/LC_MESSAGES/slint-replay.po; Rust только для property/callback contract.
Проверь все поверхности и 17 Settings-вкладок, clipping/wrap,
focus/accessibility, темы, ASCII/no-tofu, @tr+PO, reset properties, callback
parity, i18n_guard/icon_guard/settings_reset_guard и manual evidence gaps.
'@
    },
    [pscustomobject]@{
        Id = "11-build-package-docs"
        Domain = @'
Сборка, упаковка и эксплуатационная документация.
Scope: три Cargo.toml, committed lock-файлы, build.rs, scripts/ci.ps1,
NSIS/release scripts, hooks, README.md, docs/architecture.md и release
checklists.
Проверь покрытие всех трёх standalone crates gate-скриптом, отдельную упаковку
sidecar, синхронность версий, installer asset contract, запреты
tag/release/master push и устаревшие утверждения в документации.
'@
    }
)

$excludedTools = @(
    "edit", "write_file", "run_shell_command", "notebook_edit",
    "read_mcp_resource", "tool_search", "send_message",
    "cron_create", "cron_list", "cron_delete", "task_stop", "skill",
    "todo_write", "record_artifact", "loop_wakeup",
    "enter_worktree", "exit_worktree", "ask_user_question", "monitor",
    "agent", "create_sub_session", "list_agents", "web_fetch", "web_search",
    "computer_use__bring_to_front", "computer_use__check_for_update",
    "computer_use__check_permissions", "computer_use__click",
    "computer_use__double_click", "computer_use__drag",
    "computer_use__end_session", "computer_use__get_accessibility_tree",
    "computer_use__get_agent_cursor_state", "computer_use__get_config",
    "computer_use__get_cursor_position", "computer_use__get_recording_state",
    "computer_use__get_screen_size", "computer_use__get_window_state",
    "computer_use__hotkey", "computer_use__kill_app",
    "computer_use__launch_app", "computer_use__list_apps",
    "computer_use__list_windows", "computer_use__move_cursor",
    "computer_use__page", "computer_use__press_key",
    "computer_use__replay_trajectory", "computer_use__right_click",
    "computer_use__scroll", "computer_use__set_agent_cursor_enabled",
    "computer_use__set_agent_cursor_motion", "computer_use__set_agent_cursor_style",
    "computer_use__set_config", "computer_use__set_value",
    "computer_use__start_recording", "computer_use__start_session",
    "computer_use__stop_recording", "computer_use__type_text",
    "computer_use__zoom"
) -join ","

New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$outputRoot = (Resolve-Path $OutputDirectory).Path
$pending = [Collections.Generic.Queue[object]]::new()
foreach ($audit in $audits) {
    $pending.Enqueue($audit)
}
$running = @()
$completed = @()

while ($pending.Count -gt 0 -or $running.Count -gt 0) {
    while ($pending.Count -gt 0 -and $running.Count -lt $MaxParallel) {
        $audit = $pending.Dequeue()
        $promptPath = Join-Path $outputRoot "$($audit.Id).prompt.txt"
        $resultPath = Join-Path $outputRoot "$($audit.Id).result.txt"
        $errorPath = Join-Path $outputRoot "$($audit.Id).stderr.txt"
        [IO.File]::WriteAllText($promptPath, "$common`r`nНазначенный домен:`r`n$($audit.Domain)",
            [Text.UTF8Encoding]::new($false))

        $repoQuoted = $repo.Replace("'", "''")
        $qwenQuoted = $qwen.Replace("'", "''")
        $promptQuoted = $promptPath.Replace("'", "''")
        $excludedQuoted = $excludedTools.Replace("'", "''")
        $child = @"
Set-Location -LiteralPath '$repoQuoted'
`$prompt = Get-Content -Raw -LiteralPath '$promptQuoted'
`$qwenArgs = @(
    '--safe-mode', '--approval-mode', 'plan', '--no-chat-recording',
    '--max-session-turns', '60', '--max-wall-time', '20m',
    '--exclude-tools', '$excludedQuoted', '-m', '$model', '-o', 'text',
    `$prompt
)
& '$qwenQuoted' @qwenArgs
exit `$LASTEXITCODE
"@
        $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($child))
        $process = Start-Process -FilePath "powershell.exe" -WindowStyle Hidden -PassThru `
            -ArgumentList @("-NoProfile", "-EncodedCommand", $encoded) `
            -RedirectStandardOutput $resultPath -RedirectStandardError $errorPath
        $running += [pscustomobject]@{
            Audit = $audit
            Process = $process
            ResultPath = $resultPath
            ErrorPath = $errorPath
        }
        Write-Host "[started] $($audit.Id)"
    }

    Start-Sleep -Seconds 2
    $nextRunning = @()
    foreach ($job in $running) {
        if ($job.Process.HasExited) {
            $job.Process.WaitForExit()
            $job.Process.Refresh()
            $exitCode = $job.Process.ExitCode
            if ($null -eq $exitCode) {
                $exitCode = if ((Get-Item $job.ResultPath).Length -gt 0) { 0 } else { 1 }
            }
            $completed += [pscustomobject]@{
                id = $job.Audit.Id
                exit_code = $exitCode
                result = $job.ResultPath
                stderr = $job.ErrorPath
            }
            Write-Host "[done:$exitCode] $($job.Audit.Id)"
        } else {
            $nextRunning += $job
        }
    }
    $running = $nextRunning
}

$manifestPath = Join-Path $outputRoot "manifest.json"
[IO.File]::WriteAllText(
    $manifestPath,
    ($completed | ConvertTo-Json -Depth 4),
    [Text.UTF8Encoding]::new($false)
)
Write-Host "Results: $outputRoot"

if ($completed.Where({ $_.exit_code -ne 0 }).Count -gt 0) {
    exit 1
}
