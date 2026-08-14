# suflyor-teratts — third-party model notice and release gate

## RELEASE GATE (read before distributing this feature)

The TeraTTSv2 model assets (ONNX graphs, voice styles, tokenizer table,
RUAccent-derived dictionaries and neural graphs) come from the public
Hugging Face repository `TeraSpace/TeraTTSv2` (pinned revision
`f05ea799094571a3553904a555df3834fb0b963b`).

**Upstream license status as of 2026-08-10: NONE published.** The repository
exposes no LICENSE file and no `license` metadata. The model author is
reported by the owner to have stated "MIT" in a Telegram conversation; that
statement is **not** a verified public grant. It is sufficient to develop
against the published artifacts, and **not sufficient** to claim a verified
right to redistribute them.

DO NOT publish a suflyor release that ships, mirrors, or claims rights over
TeraTTSv2 weights or assets until an archived written grant from the author
is on file that explicitly covers ALL of:

1. the TeraTTSv2 code (graph contracts and reference pipeline),
2. the model weights (text encoder, duration predictor, samplers, vocoder),
3. the voice style assets (`styles/*`),
4. the RUAccent-derived dictionaries and neural assets bundled in the release,
5. commercial redistribution, including redistribution inside the suflyor
   installer or update channels.

Until that grant exists, suflyor must only point users at the official
upstream URL and download the assets on demand to the user's own machine
(`%APPDATA%\suflyor\tts\teratts-v2-<revision>`), which is what the current
implementation does. Never package model weights in the NSIS installer.

## Upstream attribution

TeraTTSv2 upstream: https://huggingface.co/TeraSpace/TeraTTSv2
No copyright holder or year is asserted here because upstream publishes none;
do not invent one.

## Bundled RUAccent-derived assets (verbatim upstream notice)

The release downloads `RUACCENT_NOTICE.txt` alongside the model. Its content,
preserved verbatim from upstream revision
`f05ea799094571a3553904a555df3834fb0b963b`:

> TeraTTS includes a local adaptation of RUAccent and its ONNX model assets.
>
> RUAccent is Copyright 2026 Denis Petrov and licensed under the MIT License:
>
> Permission is hereby granted, free of charge, to any person obtaining a copy
> of this software and associated documentation files (the "Software"), to deal
> in the Software without restriction, including without limitation the rights
> to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
> copies of the Software, and to permit persons to whom the Software is
> furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in all
> copies or substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
> IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
> FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
> AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
> LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
> OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
> SOFTWARE.

RUAccent upstream: https://github.com/Den4ikAI/ruaccent

## suflyor code in this crate

The `suflyor-teratts` sidecar source itself is part of suflyor and licensed
like the rest of the project (GPL-3.0-or-later, see `Cargo.toml` and the
repository `LICENSE`). This notice concerns the third-party MODEL ASSETS only.
