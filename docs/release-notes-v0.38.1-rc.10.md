# Suflyor v0.38.1-rc.10

This release candidate is v0.38.1-rc.9 plus faster automatic answers on calls (pull request #283).

## Fast answers on calls

- **Auto tiles stream.** The tile opens as soon as a question is heard and the answer is written into it as the model produces it. Before, the tile appeared only after the whole answer, 6–10 s after the question with a local Strata model.
- **Fast answers for calls and interviews** (Settings, under the auto-tile detector, on by default): a one-line answer first, then two or three points. Turn it off to get the previous longer answers.
- **Local servers keep the prompt ready.** Suflyor recognises Strata, llama.cpp and Ollama. With Strata the call prompt stays in the server's cache during a session, so the model starts answering without reading it again; Ollama is asked to keep the model loaded for 30 minutes.
- If the server closes the answer before any text, the auto tile retries twice and then closes instead of staying empty.

Measured with Strata (Qwen3.8 Flash-Next IQ3_S): first words about 0.6 s after the question reaches the model, the whole answer about 2 s. Speech recognition adds its own time before that.

## Not changed

- F9 and typed questions keep their own prompt, knowledge base and tools.
- Strata answers one request at a time: while a long F9 answer is being written, an auto tile waits for it.
- llama.cpp and Ollama are covered by tests only; the live check used Strata.

## Known, not fixed here

The items listed for v0.38.1-rc.9 remain: the bar has 19 buttons in one row, notice and error tiles keep the full size, some screens show state imprecisely, the setup wizard has no repair action.

## Installers

- Windows 10/11: `suflyor-slint-setup.exe`.

The Windows installer is unsigned.
