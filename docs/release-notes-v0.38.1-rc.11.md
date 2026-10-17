# Suflyor v0.38.1-rc.11

This release candidate is v0.38.1-rc.10 plus answers with context and term definitions on calls (pull request #287) and a batch of dependency and file-permission updates (pull request #285).

## Answers on calls

- **Answers with context.** A fast answer is one line to say, then three to five points: what it is, how it works, how the two sides differ, a typical cause or example. "502 vs 503" went from 31 to about 130 words; the first words still show in about 0.6 s with Strata.
- **The whole question.** An auto tile shows everything the other person said, wrapped in a box under the header (the end of a long line, where the question is). The header keeps only the tile number.
- **Terms under the answer.** The answer names the terms a listener may not know; they are buttons under it. A click opens a short definition under the answer, with terms of its own, so you can go deeper: 502 Bad Gateway › nginx › web server. The path goes back; an opened definition shows at once the next time. Nothing is asked of the model until you click.
- **No reasoning in fast answers.** Fast answers turn a local model's reasoning off even when "Local model reasoning" is on, so the answer starts at once.
- **Answer language.** With the answer language on auto, an English question gets an English answer.

Measured with Strata (Qwen3.8 Flash-Next IQ3_S): the answer's first words 0.6 s after the question reaches the model, the whole answer about 4 s; a definition 1.2 s to the first words, about 2.5 s in full. Speech recognition adds its own time before that.

## Other changes

- Dependencies: wasapi 0.26, transcribe-rs 0.3.12, tokio, serde_json and build tools.
- On macOS the overlay log, the exported log, the single-instance lock and the session-names file are readable by their owner only.
- The exported log also masks values labelled `groq_api_key` or `password` and Hugging Face tokens.

## Not changed

- F9 and typed questions keep their own prompt, without terms.
- Strata answers one request at a time: a definition asked while an answer is being written waits for it.

## Known, not fixed here

The items listed for v0.38.1-rc.9 remain: the bar has 19 buttons in one row, notice and error tiles keep the full size, some screens show state imprecisely, the setup wizard has no repair action.

## Installers

- Windows 10/11: `suflyor-slint-setup.exe`.

The Windows installer is unsigned.
