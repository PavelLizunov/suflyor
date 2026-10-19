# Suflyor v0.38.1-rc.12

This release candidate is v0.38.1-rc.11 plus Settings that say what to do next when speech recognition, Components or screenshots are not ready (pull request #290).

## Speech recognition (STT)

- **GigaAM download on Windows.** The STT tab now offers the GigaAM download on Windows too. Once the model is installed the button reads «Check files»: it checks the files against the pinned checksum and restores what is missing or damaged.
- **A check that says what to do.** «Check STT connection» with GigaAM tells "this folder has no model" from "the model is there but did not start" and names the button to press next. A working model reads "The model loaded; recognition is ready".
- **The folder you see is the folder used.** When the saved GigaAM folder has no model but the managed install is complete, the app uses the managed folder, the one Settings already shows. A lost `vocab.txt` beside a complete model is restored.

## Components

- **In use now.** A new card at the top names what answers, screenshots and speech use (provider and model, never an address or key), whether each is set up, and a «Set up» button that opens its tab.
- **Not needed.** Local parts that your current choice does not use read «not needed» with the reason, for example the local engine while answers go through OpenAI API.

## Screenshots (F8)

- **Same as the answer model.** This choice now works when answers go through OpenAI API or Anthropic API. It uses the key, address and model from AI providers, so nothing is entered twice.
- Screenshots that were never set up follow a newly chosen OpenAI or Anthropic answer provider.
- The provider list shows the choice that was saved.
- «Test screenshot model» names the failure, for example "The key was rejected".

## Other changes

- The generic connection failure text now asks to check the address, key and model.
- The supply-chain check ignores three new "unmaintained" notices for the Linux-only D-Bus crates under Slint (RUSTSEC-2026-0370, 0371, 0372); they are not built into the Windows app.

## Known, not fixed here

The items listed for v0.38.1-rc.9 remain: the bar has 19 buttons in one row, notice and error tiles keep the full size, some screens show state imprecisely, the setup wizard has no repair action.

## Installers

- Windows 10/11: `suflyor-slint-setup.exe`.

The Windows installer is unsigned.
