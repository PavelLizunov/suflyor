---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_fa9750e4bc3f"
source_path: "overlay-backend/src/local_ai/model_choice.rs"
batch_id: "B06"
total_lines: 370
symbols_count: 27
review_state: validated
---

# File Map: `overlay-backend/src/local_ai/model_choice.rs`

- **Batch:** B06
- **Physical Lines:** 370
- **Coverage:** 370/370 lines (100%)

## Types & Structures (4)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `LocalContextPreset` | L41 | pub |
| enum | `ManagedModel` | L128 | pub |
| struct | `ModelSpec` | L212 | pub |
| struct | `ManagedLlamaChoice` | L241 | pub |

## Symbols & Routines (27)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `is_managed_llama_endpoint` | L12 | `fn is_managed_llama_endpoint(base_url: &str) -> bool` |
| function | `from_config` | L52 | `fn from_config(value: &str) -> Self` |
| function | `as_config` | L64 | `fn as_config(self) -> &'static str` |
| function | `from_index` | L76 | `fn from_index(index: i32) -> Self` |
| function | `index` | L88 | `fn index(self) -> i32` |
| function | `context_tokens` | L100 | `fn context_tokens(self, profile: HardwareModelProfile, _prep: bool) -> u32` |
| function | `estimated_vram_delta_mib` | L115 | `fn estimated_vram_delta_mib(self, profile: HardwareModelProfile) -> i32` |
| function | `from_config` | L136 | `fn from_config(model_id: &str, quality: bool) -> Self` |
| function | `from_index` | L147 | `fn from_index(index: i32) -> Self` |
| function | `index` | L156 | `fn index(self) -> i32` |
| function | `file_name` | L165 | `fn file_name(self) -> &'static str` |
| function | `is_quality` | L174 | `fn is_quality(self) -> bool` |
| function | `spec` | L182 | `fn spec(self) -> ModelSpec` |
| function | `estimated_total_vram_mib` | L225 | `fn estimated_total_vram_mib(model: ManagedModel, context: LocalContextPreset, profile: HardwareModelProfile,) -> u32` |
| function | `new` | L249 | `fn new(prefer_quality: bool, context: LocalContextPreset) -> Self` |
| function | `for_model` | L262 | `fn for_model(model: ManagedModel, context: LocalContextPreset) -> Self` |
| function | `for_custom` | L271 | `fn for_custom(path: PathBuf, context: LocalContextPreset) -> Self` |
| function | `from_config` | L280 | `fn from_config(model_id: &str, quality: bool, custom_gguf: &str, context: LocalContextPreset,) -> Self` |
| function | `with_context` | L294 | `fn with_context(&self, context: LocalContextPreset) -> Self` |
| function | `custom_gguf` | L303 | `fn custom_gguf(&self) -> Option<&Path>` |
| function | `is_custom` | L308 | `fn is_custom(&self) -> bool` |
| function | `valid_custom_gguf_path` | L315 | `fn valid_custom_gguf_path(value: &str) -> Option<PathBuf>` |
| function | `custom_gguf_display_name` | L335 | `fn custom_gguf_display_name(value: &str) -> String` |
| function | `valid_custom_choice_path` | L344 | `fn valid_custom_choice_path(choice: &ManagedLlamaChoice) -> Option<PathBuf>` |
| function | `custom_choice_alias` | L350 | `fn custom_choice_alias(choice: &ManagedLlamaChoice) -> Option<String>` |
| function | `effective_llama_choice` | L357 | `fn effective_llama_choice(root: &Path, choice: &ManagedLlamaChoice,) -> ManagedLlamaChoice` |
| function | `llama_choice_name` | L368 | `fn llama_choice_name(root: &Path, choice: &ManagedLlamaChoice) -> String` |
