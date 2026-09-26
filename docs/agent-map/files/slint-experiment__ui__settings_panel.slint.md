---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_7abb3fee5ac1"
source_path: "slint-experiment/ui/settings_panel.slint"
batch_id: "B05"
total_lines: 4170
symbols_count: 378
review_state: validated
---

# File Map: `slint-experiment/ui/settings_panel.slint`

- **Batch:** B05
- **Physical Lines:** 4170
- **Coverage:** 4170/4170 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| slint_component | `SettingsWindow` | L62 | - |

## Symbols & Routines (378)

| Kind | Name | Line | Signature |
|---|---|---|---|
| ui_component | `SettingsWindow` | L62 | `-` |
| slint_property | `widgets-light` | L69 | `bool` |
| slint_property | `active-tab` | L91 | `int` |
| slint_property | `tile-monitors` | L96 | `[string]` |
| slint_property | `tile-monitor-index` | L97 | `int` |
| slint_property | `memory-candidates` | L101 | `[MemoryRow]` |
| slint_property | `memory-items` | L102 | `[MemoryRow]` |
| slint_property | `component-rows` | L104 | `[ComponentRow]` |
| slint_property | `component-busy-index` | L107 | `int` |
| slint_property | `component-busy-phase` | L108 | `int` |
| slint_property | `component-busy-label` | L109 | `string` |
| slint_property | `memory-status` | L112 | `string` |
| slint_property | `memory-add-text` | L121 | `string` |
| slint_property | `memory-editing-id` | L126 | `int` |
| slint_property | `memory-edit-text` | L127 | `string` |
| slint_property | `always-on-top-toggle` | L130 | `bool` |
| slint_property | `stealth-toggle` | L131 | `bool` |
| slint_property | `stealth-effective` | L138 | `bool` |
| slint_property | `ai-bearer-status` | L145 | `string` |
| slint_property | `ai-bearer-input` | L146 | `string` |
| slint_property | `groq-api-key-status` | L147 | `string` |
| slint_property | `groq-api-key-input` | L148 | `string` |
| slint_property | `ai-base-url-input` | L149 | `string` |
| slint_property | `openai-key-status` | L150 | `string` |
| slint_property | `openai-key-input` | L151 | `string` |
| slint_property | `openai-base-url-input` | L152 | `string` |
| slint_property | `openai-model-input` | L153 | `string` |
| slint_property | `anthropic-key-status` | L154 | `string` |
| slint_property | `anthropic-key-input` | L155 | `string` |
| slint_property | `anthropic-base-url-input` | L156 | `string` |
| slint_property | `anthropic-model-input` | L157 | `string` |
| slint_property | `codex-auth-status` | L158 | `string` |
| slint_property | `codex-auth-busy` | L159 | `bool` |
| slint_property | `codex-models-busy` | L160 | `bool` |
| slint_property | `codex-login-url` | L161 | `string` |
| slint_property | `codex-user-code` | L162 | `string` |
| slint_property | `codex-copy-status` | L163 | `string` |
| slint_property | `codex-model-labels` | L164 | `[string]` |
| slint_property | `codex-model-ids` | L165 | `[string]` |
| slint_property | `codex-model-index` | L166 | `int` |
| slint_property | `codex-reasoning-labels` | L167 | `[string]` |
| slint_property | `codex-reasoning-ids` | L168 | `[string]` |
| slint_property | `codex-reasoning-index` | L169 | `int` |
| slint_property | `codex-vision-model-labels` | L170 | `[string]` |
| slint_property | `codex-vision-model-ids` | L171 | `[string]` |
| slint_property | `codex-vision-model-index` | L172 | `int` |
| slint_property | `codex-rate-status` | L173 | `string` |
| slint_property | `hermes-bridge-enabled` | L177 | `bool` |
| slint_property | `hermes-bridge-port` | L178 | `string` |
| slint_property | `hermes-bridge-token` | L179 | `string` |
| slint_property | `hermes-bridge-host` | L180 | `string` |
| slint_property | `hermes-bridge-remote` | L181 | `bool` |
| slint_property | `hermes-bridge-status` | L182 | `string` |
| slint_property | `hermes-plugin-install-status` | L188 | `string` |
| slint_property | `hermes-api-url` | L191 | `string` |
| slint_property | `hermes-api-key` | L192 | `string` |
| slint_property | `hermes-api-test-result` | L193 | `string` |
| slint_property | `hermes-api-setup-status` | L195 | `string` |
| slint_property | `hermes-profile-seed` | L197 | `string` |
| slint_property | `hermes-profile-status` | L198 | `string` |
| slint_property | `ai-models` | L205 | `[string]` |
| slint_property | `ai-model-index` | L206 | `int` |
| slint_property | `ai-prompt-cache` | L208 | `bool` |
| slint_property | `ai-provider-index` | L210 | `int` |
| slint_property | `mlx-text-model-index` | L211 | `int` |
| slint_property | `mlx-text-busy` | L213 | `bool` |
| slint_property | `mlx-text-checking` | L214 | `bool` |
| slint_property | `mlx-text-installed` | L215 | `bool` |
| slint_property | `mlx-text-active` | L216 | `bool` |
| slint_property | `mlx-text-failed` | L217 | `bool` |
| slint_property | `mlx-text-cancelled` | L218 | `bool` |
| slint_property | `mlx-text-progress` | L219 | `float` |
| slint_property | `mlx-text-done` | L220 | `string` |
| slint_property | `mlx-text-total` | L221 | `string` |
| slint_property | `mlx-vision-model-index` | L225 | `int` |
| slint_property | `mlx-vision-busy` | L227 | `bool` |
| slint_property | `mlx-vision-checking` | L228 | `bool` |
| slint_property | `mlx-vision-installed` | L229 | `bool` |
| slint_property | `mlx-vision-active` | L230 | `bool` |
| slint_property | `mlx-vision-failed` | L231 | `bool` |
| slint_property | `mlx-vision-cancelled` | L232 | `bool` |
| slint_property | `mlx-vision-progress` | L233 | `float` |
| slint_property | `mlx-vision-done` | L234 | `string` |
| slint_property | `mlx-vision-total` | L235 | `string` |
| slint_property | `vision-provider-index` | L240 | `int` |
| slint_property | `vision-same-available` | L241 | `bool` |
| slint_property | `vision-phonetics` | L243 | `bool` |
| slint_property | `vision-test-practice` | L245 | `bool` |
| slint_property | `vision-base-url-input` | L246 | `string` |
| slint_property | `vision-bearer-input` | L247 | `string` |
| slint_property | `vision-model-input` | L248 | `string` |
| slint_property | `vision-local-base-url-input` | L249 | `string` |
| slint_property | `vision-local-bearer-input` | L250 | `string` |
| slint_property | `vision-local-model-input` | L251 | `string` |
| slint_property | `vision-test-result` | L252 | `string` |
| slint_property | `tts-voice-names` | L265 | `[string]` |
| slint_property | `tts-voice-index` | L266 | `int` |
| slint_property | `tts-rate-index` | L267 | `int` |
| slint_property | `tts-available` | L268 | `bool` |
| slint_property | `tts-test-status` | L269 | `string` |
| slint_property | `tts-installing` | L270 | `bool` |
| slint_property | `tts-install-phase` | L271 | `int` |
| slint_property | `tts-install-label` | L272 | `string` |
| slint_property | `tts-engine-names` | L279 | `[string]` |
| slint_property | `tts-engine-index` | L280 | `int` |
| slint_property | `tera-model-status` | L282 | `string` |
| slint_property | `tera-installing` | L283 | `bool` |
| slint_property | `tera-install-phase` | L284 | `int` |
| slint_property | `tera-install-label` | L285 | `string` |
| slint_property | `ocr-installed` | L291 | `bool` |
| slint_property | `ocr-installing` | L292 | `bool` |
| slint_property | `ocr-install-phase` | L293 | `int` |
| slint_property | `diar-models-installed` | L297 | `bool` |
| slint_property | `diar-installing` | L298 | `bool` |
| slint_property | `diar-install-status` | L299 | `string` |
| slint_property | `ai-local-base-url-input` | L301 | `string` |
| slint_property | `ai-local-models` | L304 | `[string]` |
| slint_property | `ai-local-model-index` | L305 | `int` |
| slint_property | `managed-local-server` | L306 | `bool` |
| slint_property | `local-model-resource-warning` | L307 | `string` |
| slint_property | `ai-local-vision` | L308 | `bool` |
| slint_property | `ai-local-vision-available` | L309 | `bool` |
| slint_property | `ai-local-thinking` | L312 | `bool` |
| slint_property | `ai-local-quality` | L315 | `bool` |
| slint_property | `ai-local-model-profile-index` | L316 | `int` |
| slint_property | `legacy-model-present` | L317 | `bool` |
| slint_property | `fallback-model-present` | L318 | `bool` |
| slint_property | `quality-model-present` | L319 | `bool` |
| slint_property | `quality-selection-allowed` | L323 | `bool` |
| slint_property | `quality-downloading` | L324 | `bool` |
| slint_property | `model-switching` | L325 | `bool` |
| slint_property | `quality-progress` | L326 | `float` |
| slint_property | `quality-status` | L327 | `string` |
| slint_property | `ai-local-custom-active` | L328 | `bool` |
| slint_property | `ai-local-custom-model-name` | L329 | `string` |
| slint_property | `ai-local-context-index` | L331 | `int` |
| slint_property | `ai-local-context-preview-index` | L332 | `int` |
| slint_property | `ai-local-hardware-profile-index` | L333 | `int` |
| slint_property | `ai-local-context-max-k` | L334 | `int` |
| slint_property | `ai-local-context-auto-k` | L335 | `int` |
| slint_property | `ai-local-context-vram-hint` | L336 | `string` |
| slint_property | `quality-vision-present` | L339 | `bool` |
| slint_property | `quality-vision-supported` | L340 | `bool` |
| slint_property | `vision12b-downloading` | L341 | `bool` |
| slint_property | `vision12b-status` | L342 | `string` |
| slint_property | `engine-build` | L345 | `string` |
| slint_property | `engine-updating` | L346 | `bool` |
| slint_property | `engine-update-status` | L347 | `string` |
| slint_property | `ai-local-test-result` | L348 | `string` |
| slint_property | `ai-local-bearer-input` | L349 | `string` |
| slint_property | `local-ai-installing` | L352 | `bool` |
| slint_property | `local-ai-status` | L353 | `string` |
| slint_property | `local-ai-progress` | L354 | `float` |
| slint_property | `local-ai-gpu` | L355 | `string` |
| slint_property | `local-ai-on-gpu` | L356 | `bool` |
| slint_property | `tile-body-opacity` | L363 | `float` |
| slint_property | `mic-devices` | L369 | `[string]` |
| slint_property | `mic-device-index` | L370 | `int` |
| slint_property | `mic-test-result` | L371 | `string` |
| slint_property | `audio-default-label` | L375 | `string` |
| slint_property | `system-devices` | L376 | `[string]` |
| slint_property | `system-device-index` | L377 | `int` |
| slint_property | `audio-devices-loading` | L378 | `bool` |
| slint_property | `audio-devices-failed` | L379 | `bool` |
| slint_property | `mic-device-missing` | L380 | `bool` |
| slint_property | `system-device-missing` | L381 | `bool` |
| slint_property | `mic-devices-empty` | L382 | `bool` |
| slint_property | `system-devices-empty` | L383 | `bool` |
| slint_property | `audio-save-state` | L385 | `int` |
| slint_property | `ai-bridge-test-result` | L440 | `string` |
| slint_property | `stt-test-result` | L441 | `string` |
| slint_property | `stt-provider-index` | L443 | `int` |
| slint_property | `stt-language-index` | L445 | `int` |
| slint_property | `stt-cloud-model-index` | L447 | `int` |
| slint_property | `stt-gigaam-dir-input` | L448 | `string` |
| slint_property | `stt-gigaam-installed` | L449 | `bool` |
| slint_property | `stt-gigaam-installing` | L450 | `bool` |
| slint_property | `stt-gigaam-install-failed` | L451 | `bool` |
| slint_property | `stt-gigaam-install-cancelled` | L452 | `bool` |
| slint_property | `stt-gigaam-install-progress` | L453 | `float` |
| slint_property | `stt-gigaam-install-done` | L454 | `string` |
| slint_property | `stt-gigaam-install-total` | L455 | `string` |
| slint_property | `stt-gigaam-gpu` | L459 | `bool` |
| slint_property | `stt-whisper-url-input` | L460 | `string` |
| slint_property | `stt-whisper-bearer-input` | L461 | `string` |
| slint_property | `stt-whisper-model-input` | L462 | `string` |
| slint_property | `profile-io-result` | L474 | `string` |
| slint_property | `server-preview-ready` | L486 | `bool` |
| slint_property | `server-preview-cloud` | L490 | `string` |
| slint_property | `server-preview-local` | L491 | `string` |
| slint_property | `server-preview-vision` | L492 | `string` |
| slint_property | `server-preview-stt` | L493 | `string` |
| slint_property | `server-preview-gigaam` | L495 | `string` |
| slint_property | `diag-summary` | L499 | `string` |
| slint_property | `diag-ai-level` | L500 | `int` |
| slint_property | `diag-ai-detail` | L501 | `string` |
| slint_property | `diag-stt-level` | L502 | `int` |
| slint_property | `diag-stt-detail` | L503 | `string` |
| slint_property | `diag-mic-level` | L504 | `int` |
| slint_property | `diag-mic-detail` | L505 | `string` |
| slint_property | `diag-sys-level` | L506 | `int` |
| slint_property | `diag-sys-detail` | L507 | `string` |
| slint_property | `diag-stealth-on` | L508 | `bool` |
| slint_property | `diag-vision-level` | L510 | `int` |
| slint_property | `diag-vision-detail` | L511 | `string` |
| slint_property | `diag-hotkeys-level` | L514 | `int` |
| slint_property | `diag-hotkeys-detail` | L515 | `string` |
| slint_property | `diag-hotkeys-failed` | L518 | `string` |
| slint_property | `diag-copied` | L521 | `bool` |
| slint_property | `diag-logs-path` | L524 | `string` |
| slint_property | `db-repair-status` | L529 | `string` |
| slint_property | `db-clear-armed` | L534 | `string` |
| slint_property | `meeting-context-input` | L538 | `string` |
| slint_property | `meeting-context-result` | L539 | `string` |
| slint_property | `profile-names` | L543 | `[string]` |
| slint_property | `active-profile-index` | L544 | `int` |
| slint_property | `profile-name-input` | L545 | `string` |
| slint_property | `coaching-debrief` | L551 | `bool` |
| slint_property | `coaching-live-tiles` | L553 | `bool` |
| slint_property | `record-audio` | L556 | `bool` |
| slint_property | `retention-mode` | L561 | `int` |
| slint_property | `retention-value` | L562 | `string` |
| slint_property | `journal-keep-value` | L565 | `string` |
| slint_property | `journal-mb-value` | L566 | `string` |
| slint_property | `auto-tiles-enabled` | L571 | `bool` |
| slint_property | `suppress-tiles` | L575 | `bool` |
| slint_property | `trigger-keywords-input` | L577 | `string` |
| slint_property | `context-processing` | L583 | `bool` |
| slint_property | `context-dictating` | L589 | `bool` |
| slint_property | `ui-language-index` | L596 | `int` |
| slint_property | `color-scheme-index` | L603 | `int` |
| slint_property | `app-version` | L610 | `string` |
| slint_property | `update-status` | L611 | `string` |
| slint_property | `update-available` | L612 | `bool` |
| slint_property | `update-checking` | L613 | `bool` |
| slint_property | `update-download-url` | L616 | `string` |
| slint_callback | `component-install` | L111 | `-` |
| slint_callback | `memory-approve` | L113 | `-` |
| slint_callback | `memory-reject` | L114 | `-` |
| slint_callback | `memory-delete-item` | L115 | `-` |
| slint_callback | `memory-restore-item` | L116 | `-` |
| slint_callback | `memory-extract` | L117 | `-` |
| slint_callback | `memory-add-fact` | L122 | `-` |
| slint_callback | `memory-edit-save` | L128 | `-` |
| slint_callback | `hermes-bridge-toggled` | L183 | `-` |
| slint_callback | `hermes-bridge-port-save` | L184 | `-` |
| slint_callback | `hermes-bridge-host-save` | L185 | `-` |
| slint_callback | `hermes-bridge-regen-token` | L186 | `-` |
| slint_callback | `hermes-plugin-install` | L189 | `-` |
| slint_callback | `hermes-api-setup` | L196 | `-` |
| slint_callback | `hermes-api-url-save` | L199 | `-` |
| slint_callback | `hermes-api-key-save` | L200 | `-` |
| slint_callback | `hermes-api-test` | L201 | `-` |
| slint_callback | `hermes-prepare-profile` | L202 | `-` |
| slint_callback | `mlx-text-model-changed` | L212 | `-` |
| slint_callback | `mlx-text-download` | L222 | `-` |
| slint_callback | `mlx-text-cancel` | L223 | `-` |
| slint_callback | `mlx-text-enable` | L224 | `-` |
| slint_callback | `mlx-vision-model-changed` | L226 | `-` |
| slint_callback | `mlx-vision-download` | L236 | `-` |
| slint_callback | `mlx-vision-cancel` | L237 | `-` |
| slint_callback | `mlx-vision-enable` | L238 | `-` |
| slint_callback | `vision-provider-changed` | L253 | `-` |
| slint_callback | `vision-phonetics-changed` | L254 | `-` |
| slint_callback | `vision-test-practice-changed` | L255 | `-` |
| slint_callback | `vision-base-url-save` | L256 | `-` |
| slint_callback | `vision-bearer-save` | L257 | `-` |
| slint_callback | `vision-model-save` | L258 | `-` |
| slint_callback | `vision-local-base-url-save` | L259 | `-` |
| slint_callback | `vision-local-bearer-save` | L260 | `-` |
| slint_callback | `vision-local-model-save` | L261 | `-` |
| slint_callback | `vision-test-clicked` | L262 | `-` |
| slint_callback | `tts-voice-changed` | L273 | `-` |
| slint_callback | `tts-rate-changed` | L274 | `-` |
| slint_callback | `tts-test-clicked` | L275 | `-` |
| slint_callback | `tts-install-clicked` | L276 | `-` |
| slint_callback | `tts-engine-changed` | L281 | `-` |
| slint_callback | `tera-install-clicked` | L286 | `-` |
| slint_callback | `tera-install-cancel-clicked` | L287 | `-` |
| slint_callback | `ocr-install-clicked` | L294 | `-` |
| slint_callback | `diar-install-clicked` | L300 | `-` |
| slint_callback | `install-local-ai-clicked` | L357 | `-` |
| slint_callback | `cancel-local-ai-clicked` | L358 | `-` |
| slint_callback | `tile-body-opacity-changed` | L364 | `-` |
| slint_callback | `mic-device-selected` | L372 | `-` |
| slint_callback | `mic-test-clicked` | L373 | `-` |
| slint_callback | `audio-devices-refresh` | L386 | `-` |
| slint_callback | `mic-device-index-selected` | L387 | `-` |
| slint_callback | `system-device-selected` | L388 | `-` |
| slint_callback | `always-on-top-changed` | L389 | `-` |
| slint_callback | `stealth-changed` | L391 | `-` |
| slint_callback | `tile-monitor-changed` | L392 | `-` |
| slint_callback | `open-wizard-clicked` | L393 | `-` |
| slint_callback | `close-clicked` | L394 | `-` |
| slint_callback | `tab-selected` | L395 | `-` |
| slint_callback | `drag-start-requested` | L397 | `-` |
| slint_callback | `drag-moved` | L398 | `-` |
| slint_callback | `ai-bearer-save` | L399 | `-` |
| slint_callback | `groq-api-key-save` | L400 | `-` |
| slint_callback | `ai-base-url-save` | L401 | `-` |
| slint_callback | `openai-key-save` | L402 | `-` |
| slint_callback | `openai-base-url-save` | L403 | `-` |
| slint_callback | `openai-model-save` | L404 | `-` |
| slint_callback | `anthropic-key-save` | L405 | `-` |
| slint_callback | `anthropic-base-url-save` | L406 | `-` |
| slint_callback | `anthropic-model-save` | L407 | `-` |
| slint_callback | `codex-connect-clicked` | L408 | `-` |
| slint_callback | `codex-disconnect-clicked` | L409 | `-` |
| slint_callback | `codex-open-signin-clicked` | L410 | `-` |
| slint_callback | `codex-copy-code-clicked` | L411 | `-` |
| slint_callback | `codex-model-selected` | L412 | `-` |
| slint_callback | `codex-reasoning-selected` | L413 | `-` |
| slint_callback | `codex-vision-model-selected` | L414 | `-` |
| slint_callback | `codex-models-refresh` | L415 | `-` |
| slint_callback | `ai-model-selected` | L416 | `-` |
| slint_callback | `ai-models-refresh` | L417 | `-` |
| slint_callback | `ai-prompt-cache-changed` | L418 | `-` |
| slint_callback | `ai-provider-changed` | L419 | `-` |
| slint_callback | `ai-local-base-url-save` | L420 | `-` |
| slint_callback | `ai-local-bearer-save` | L421 | `-` |
| slint_callback | `ai-local-model-selected` | L422 | `-` |
| slint_callback | `ai-local-models-refresh` | L423 | `-` |
| slint_callback | `ai-local-vision-changed` | L424 | `-` |
| slint_callback | `ai-local-thinking-changed` | L425 | `-` |
| slint_callback | `model-profile-changed` | L430 | `-` |
| slint_callback | `ai-local-context-preview-changed` | L431 | `-` |
| slint_callback | `ai-local-context-changed` | L432 | `-` |
| slint_callback | `download-model-clicked` | L433 | `-` |
| slint_callback | `choose-custom-gguf-clicked` | L434 | `-` |
| slint_callback | `download-vision12b-clicked` | L436 | `-` |
| slint_callback | `update-engine-clicked` | L437 | `-` |
| slint_callback | `ai-local-test-clicked` | L438 | `-` |
| slint_callback | `stt-gigaam-install` | L456 | `-` |
| slint_callback | `stt-gigaam-install-cancel` | L457 | `-` |
| slint_callback | `ai-bridge-test-clicked` | L463 | `-` |
| slint_callback | `stt-test-clicked` | L464 | `-` |
| slint_callback | `stt-provider-changed` | L465 | `-` |
| slint_callback | `stt-cloud-model-changed` | L466 | `-` |
| slint_callback | `stt-language-changed` | L467 | `-` |
| slint_callback | `stt-gigaam-dir-save` | L468 | `-` |
| slint_callback | `stt-gigaam-gpu-changed` | L469 | `-` |
| slint_callback | `stt-whisper-url-save` | L470 | `-` |
| slint_callback | `stt-whisper-bearer-save` | L471 | `-` |
| slint_callback | `stt-whisper-model-save` | L472 | `-` |
| slint_callback | `export-profile-clicked` | L475 | `-` |
| slint_callback | `import-profile-clicked` | L476 | `-` |
| slint_callback | `import-server-settings-clicked` | L478 | `-` |
| slint_callback | `export-server-settings-clicked` | L481 | `-` |
| slint_callback | `apply-server-settings-clicked` | L482 | `-` |
| slint_callback | `cancel-server-settings-clicked` | L483 | `-` |
| slint_callback | `diagnostics-check-all-clicked` | L525 | `-` |
| slint_callback | `diagnostics-copy-report-clicked` | L526 | `-` |
| slint_callback | `diagnostics-collect-logs-clicked` | L527 | `-` |
| slint_callback | `db-repair-clicked` | L530 | `-` |
| slint_callback | `db-clear-queue-clicked` | L535 | `-` |
| slint_callback | `db-clear-memory-clicked` | L536 | `-` |
| slint_callback | `meeting-context-save` | L540 | `-` |
| slint_callback | `profile-selected` | L546 | `-` |
| slint_callback | `profile-add` | L547 | `-` |
| slint_callback | `profile-rename` | L548 | `-` |
| slint_callback | `profile-delete` | L549 | `-` |
| slint_callback | `coaching-debrief-changed` | L552 | `-` |
| slint_callback | `coaching-live-tiles-changed` | L554 | `-` |
| slint_callback | `record-audio-changed` | L557 | `-` |
| slint_callback | `retention-changed` | L563 | `-` |
| slint_callback | `journal-retention-changed` | L567 | `-` |
| slint_callback | `open-recordings-clicked` | L568 | `-` |
| slint_callback | `open-data-folder-clicked` | L569 | `-` |
| slint_callback | `open-groq-keys-clicked` | L570 | `-` |
| slint_callback | `auto-tiles-enabled-changed` | L572 | `-` |
| slint_callback | `suppress-tiles-changed` | L576 | `-` |
| slint_callback | `trigger-keywords-save` | L578 | `-` |
| slint_callback | `context-process-clicked` | L584 | `-` |
| slint_callback | `context-dictate-clicked` | L590 | `-` |
| slint_callback | `language-selected` | L597 | `-` |
| slint_callback | `color-scheme-selected` | L604 | `-` |
| slint_callback | `check-updates-clicked` | L617 | `-` |
| slint_callback | `install-update-clicked` | L618 | `-` |

## Key Behaviors & Concurrency

- UI Callback `component-install` declared at L111
- UI Callback `memory-approve` declared at L113
- UI Callback `memory-reject` declared at L114
- UI Callback `memory-delete-item` declared at L115
- UI Callback `memory-restore-item` declared at L116
- UI Callback `memory-extract` declared at L117
- UI Callback `memory-add-fact` declared at L122
- UI Callback `memory-edit-save` declared at L128
- UI Callback `hermes-bridge-toggled` declared at L183
- UI Callback `hermes-bridge-port-save` declared at L184
- UI Callback `hermes-bridge-host-save` declared at L185
- UI Callback `hermes-bridge-regen-token` declared at L186
- UI Callback `hermes-plugin-install` declared at L189
- UI Callback `hermes-api-setup` declared at L196
- UI Callback `hermes-api-url-save` declared at L199
- UI Callback `hermes-api-key-save` declared at L200
- UI Callback `hermes-api-test` declared at L201
- UI Callback `hermes-prepare-profile` declared at L202
- UI Callback `mlx-text-model-changed` declared at L212
- UI Callback `mlx-text-download` declared at L222
- UI Callback `mlx-text-cancel` declared at L223
- UI Callback `mlx-text-enable` declared at L224
- UI Callback `mlx-vision-model-changed` declared at L226
- UI Callback `mlx-vision-download` declared at L236
- UI Callback `mlx-vision-cancel` declared at L237
- UI Callback `mlx-vision-enable` declared at L238
- UI Callback `vision-provider-changed` declared at L253
- UI Callback `vision-phonetics-changed` declared at L254
- UI Callback `vision-test-practice-changed` declared at L255
- UI Callback `vision-base-url-save` declared at L256
- UI Callback `vision-bearer-save` declared at L257
- UI Callback `vision-model-save` declared at L258
- UI Callback `vision-local-base-url-save` declared at L259
- UI Callback `vision-local-bearer-save` declared at L260
- UI Callback `vision-local-model-save` declared at L261
- UI Callback `vision-test-clicked` declared at L262
- UI Callback `tts-voice-changed` declared at L273
- UI Callback `tts-rate-changed` declared at L274
- UI Callback `tts-test-clicked` declared at L275
- UI Callback `tts-install-clicked` declared at L276
- UI Callback `tts-engine-changed` declared at L281
- UI Callback `tera-install-clicked` declared at L286
- UI Callback `tera-install-cancel-clicked` declared at L287
- UI Callback `ocr-install-clicked` declared at L294
- UI Callback `diar-install-clicked` declared at L300
- UI Callback `install-local-ai-clicked` declared at L357
- UI Callback `cancel-local-ai-clicked` declared at L358
- UI Callback `tile-body-opacity-changed` declared at L364
- UI Callback `mic-device-selected` declared at L372
- UI Callback `mic-test-clicked` declared at L373
- UI Callback `audio-devices-refresh` declared at L386
- UI Callback `mic-device-index-selected` declared at L387
- UI Callback `system-device-selected` declared at L388
- UI Callback `always-on-top-changed` declared at L389
- UI Callback `stealth-changed` declared at L391
- UI Callback `tile-monitor-changed` declared at L392
- UI Callback `open-wizard-clicked` declared at L393
- UI Callback `close-clicked` declared at L394
- UI Callback `tab-selected` declared at L395
- UI Callback `drag-start-requested` declared at L397
- UI Callback `drag-moved` declared at L398
- UI Callback `ai-bearer-save` declared at L399
- UI Callback `groq-api-key-save` declared at L400
- UI Callback `ai-base-url-save` declared at L401
- UI Callback `openai-key-save` declared at L402
- UI Callback `openai-base-url-save` declared at L403
- UI Callback `openai-model-save` declared at L404
- UI Callback `anthropic-key-save` declared at L405
- UI Callback `anthropic-base-url-save` declared at L406
- UI Callback `anthropic-model-save` declared at L407
- UI Callback `codex-connect-clicked` declared at L408
- UI Callback `codex-disconnect-clicked` declared at L409
- UI Callback `codex-open-signin-clicked` declared at L410
- UI Callback `codex-copy-code-clicked` declared at L411
- UI Callback `codex-model-selected` declared at L412
- UI Callback `codex-reasoning-selected` declared at L413
- UI Callback `codex-vision-model-selected` declared at L414
- UI Callback `codex-models-refresh` declared at L415
- UI Callback `ai-model-selected` declared at L416
- UI Callback `ai-models-refresh` declared at L417
- UI Callback `ai-prompt-cache-changed` declared at L418
- UI Callback `ai-provider-changed` declared at L419
- UI Callback `ai-local-base-url-save` declared at L420
- UI Callback `ai-local-bearer-save` declared at L421
- UI Callback `ai-local-model-selected` declared at L422
- UI Callback `ai-local-models-refresh` declared at L423
- UI Callback `ai-local-vision-changed` declared at L424
- UI Callback `ai-local-thinking-changed` declared at L425
- UI Callback `model-profile-changed` declared at L430
- UI Callback `ai-local-context-preview-changed` declared at L431
- UI Callback `ai-local-context-changed` declared at L432
- UI Callback `download-model-clicked` declared at L433
- UI Callback `choose-custom-gguf-clicked` declared at L434
- UI Callback `download-vision12b-clicked` declared at L436
- UI Callback `update-engine-clicked` declared at L437
- UI Callback `ai-local-test-clicked` declared at L438
- UI Callback `stt-gigaam-install` declared at L456
- UI Callback `stt-gigaam-install-cancel` declared at L457
- UI Callback `ai-bridge-test-clicked` declared at L463
- UI Callback `stt-test-clicked` declared at L464
- UI Callback `stt-provider-changed` declared at L465
- UI Callback `stt-cloud-model-changed` declared at L466
- UI Callback `stt-language-changed` declared at L467
- UI Callback `stt-gigaam-dir-save` declared at L468
- UI Callback `stt-gigaam-gpu-changed` declared at L469
- UI Callback `stt-whisper-url-save` declared at L470
- UI Callback `stt-whisper-bearer-save` declared at L471
- UI Callback `stt-whisper-model-save` declared at L472
- UI Callback `export-profile-clicked` declared at L475
- UI Callback `import-profile-clicked` declared at L476
- UI Callback `import-server-settings-clicked` declared at L478
- UI Callback `export-server-settings-clicked` declared at L481
- UI Callback `apply-server-settings-clicked` declared at L482
- UI Callback `cancel-server-settings-clicked` declared at L483
- UI Callback `diagnostics-check-all-clicked` declared at L525
- UI Callback `diagnostics-copy-report-clicked` declared at L526
- UI Callback `diagnostics-collect-logs-clicked` declared at L527
- UI Callback `db-repair-clicked` declared at L530
- UI Callback `db-clear-queue-clicked` declared at L535
- UI Callback `db-clear-memory-clicked` declared at L536
- UI Callback `meeting-context-save` declared at L540
- UI Callback `profile-selected` declared at L546
- UI Callback `profile-add` declared at L547
- UI Callback `profile-rename` declared at L548
- UI Callback `profile-delete` declared at L549
- UI Callback `coaching-debrief-changed` declared at L552
- UI Callback `coaching-live-tiles-changed` declared at L554
- UI Callback `record-audio-changed` declared at L557
- UI Callback `retention-changed` declared at L563
- UI Callback `journal-retention-changed` declared at L567
- UI Callback `open-recordings-clicked` declared at L568
- UI Callback `open-data-folder-clicked` declared at L569
- UI Callback `open-groq-keys-clicked` declared at L570
- UI Callback `auto-tiles-enabled-changed` declared at L572
- UI Callback `suppress-tiles-changed` declared at L576
- UI Callback `trigger-keywords-save` declared at L578
- UI Callback `context-process-clicked` declared at L584
- UI Callback `context-dictate-clicked` declared at L590
- UI Callback `language-selected` declared at L597
- UI Callback `color-scheme-selected` declared at L604
- UI Callback `check-updates-clicked` declared at L617
- UI Callback `install-update-clicked` declared at L618
