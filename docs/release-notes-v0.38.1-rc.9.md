# Suflyor v0.38.1-rc.9

This release candidate is v0.38.1-rc.8 plus the local-model work, the Settings redesign and a wording and layout pass over every window (pull requests #267, #276, #277 and #279). Every control keeps its behaviour; screens are reorganised and reworded.

## Local AI

- **Strata engine.** Settings > AI providers can install and run the Strata server with Qwen3.8, or use a Strata folder or server that is already on the PC.
- **Already on this PC.** Running Ollama, LM Studio, Jan, KoboldCpp, GPT4All and llama.cpp servers are found on their usual ports and offered with one click.
- **One model list.** "Now answering" shows which model answers and whether it responds; "Choose a model" lists what is on this PC and what can be downloaded.
- **Model tools.** The local model can search the knowledge base and personal memory (on by default) and the web through DuckDuckGo (off by default).
- **Deep lock** also unloads Strata.

## Settings

- New tab **Screenshots (F8)**; speaker models moved to Components.
- One layout rule on all tabs: action buttons fill the card, buttons in one row share it equally, row labels line up with their control, the page title stays put while the tab scrolls.
- Plain words instead of jargon and `[ok]`/`[err]` prefixes; connection checks name the cause ("No key", "The key was rejected", "The server does not answer").
- Diagnostics no longer reports a ready AI before it is checked.

## Every other window

- One header for help, palette, archive, transcript, the question window, session restore and the setup wizard.
- One set of words: AI, стенограмма, заготовки, панель, скрытность; polite forms in Russian.
- Bar: `STT: … · AI: …` and "Waiting for speech…" in the footer.
- Tiles: no badge that repeats the title, no memory stars on notices and errors, an AI error points to Settings > AI providers.
- Help follows the real bar and the Settings hotkey wording.
- Setup wizard: plain descriptions and check results, failed checks marked in the step list, an honest summary.

## Known, not fixed here

- The bar still has 19 buttons in one row, each 22 px high.
- Notice and error tiles keep the full 460×360 size.
- Some screens show state imprecisely: Audio with no devices found, whether a screenshot-server key is saved, which Components are installed.
- The setup wizard has no repair action on a failed step.

## Installers

- Windows 10/11: `suflyor-slint-setup.exe`.

The Windows installer is unsigned.
