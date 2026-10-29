# AI connection settings

Choose **Model on this PC** to use a local model. An explicit successful Strata
or built-in model activation selects that provider; downloading a model or
repairing its stored metadata does not change the answer provider. An explicit
cloud escalation remains cloud for that tile's follow-ups and regeneration.
Screenshots use their separately configured Vision route.

## Custom Chat Completions server

Enter the API base URL and exact model ID. Model discovery is optional; a server
without `GET /models` can still answer requests. A pasted URL ending in
`/chat/completions` is reduced to its base. Custom prefixes are preserved,
including APIs mounted at the origin. Include `/v1` when your server requires it.
URLs containing embedded credentials, query parameters or fragments are rejected.

Local **Connect** saves and tests the visible URL, model and nonempty token.
The compatible cloud connection test likewise saves its visible fields before
testing. An empty token input retains the saved token; the local **Clear saved
token** action removes it. The model list retains the actual saved selection,
even when the server does not list that ID. Old asynchronous results cannot
replace a newer URL, token or model selection.

## Reuse an OpenCode or Pi connection

In AI settings, press **Find connections**, select a profile, then press
**Use connection**. This explicitly reads the clients' global configuration and
copies the selected API credential into Suflyor's existing server settings.
It does not run either agent. Applying a profile does not test an inference
request; its status says so.

Supported profiles are Pi's explicit `openai-completions` model entries and
OpenCode providers using `@ai-sdk/openai-compatible`. Loopback endpoints select
the local provider; other endpoints select the compatible cloud bridge. Stored
API keys, literal keys and supported environment references are accepted.
Pi's stored login takes precedence over its configured key. OAuth subscriptions,
credential commands, file interpolation, custom headers and protocol overrides
are not imported. Other provider APIs must use Suflyor's corresponding provider
or be configured manually; subscription tokens are not ordinary API keys.

Discovery uses `PI_CODING_AGENT_DIR` or `~/.pi/agent` for Pi, and
`OPENCODE_CONFIG` or the global OpenCode configuration directory for OpenCode.
The OpenCode directory follows `XDG_CONFIG_HOME`, and its auth location follows
`XDG_DATA_HOME`; defaults are `~/.config/opencode` and
`~/.local/share/opencode/auth.json`. Without an explicit path, `opencode.jsonc`
takes precedence over `opencode.json`. Project configuration merging and
extension-defined providers are not implemented. Each input is limited to 1 MiB
and discovery returns at most 128 profiles. Credential values never enter the
selection list.

Synthetic Linux tests cover resolver separation, actual completion and streaming
request URL/model/bearer, URL normalization, catalog selection and profile parsing.
They do not reproduce the reported Windows incident or establish live GUI
acceptance, account authentication or real-model inference.
