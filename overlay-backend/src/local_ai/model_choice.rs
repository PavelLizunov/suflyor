use std::path::{Path, PathBuf};

use super::{
    active_local_model_name, effective_verified_managed_model, HardwareModelProfile, GEMMA26_FILE,
    GEMMA26_SHA256, GEMMA26_SIZE, GEMMA26_URL, GEMMA_FILE, GEMMA_SHA256, GEMMA_SIZE, GEMMA_URL,
    LEGACY_GEMMA_FILE, LEGACY_GEMMA_SHA256, LEGACY_GEMMA_SIZE, LEGACY_GEMMA_URL, LLAMA_PORT,
};

/// True only for Suflyor's bundled loopback llama.cpp endpoint. Port 8080 on a
/// LAN host or another local port is not ours to restart or relabel.
#[must_use]
pub fn is_managed_llama_endpoint(base_url: &str) -> bool {
    let Some(without_scheme) = base_url.trim().strip_prefix("http://") else {
        return false;
    };
    let (authority, path) = without_scheme
        .split_once('/')
        .map_or((without_scheme, ""), |(authority, path)| (authority, path));
    if format!("/{path}").trim_end_matches('/') != "/v1" {
        return false;
    }
    let host_port = if let Some(bracketed) = authority.strip_prefix('[') {
        bracketed.split_once("]:")
    } else {
        authority.rsplit_once(':')
    };
    host_port.is_some_and(|(host, port)| {
        port == LLAMA_PORT
            && matches!(
                host.to_ascii_lowercase().as_str(),
                "127.0.0.1" | "localhost" | "::1"
            )
    })
}

/// User-selected context for Suflyor's managed llama.cpp server.
///
/// Manual presets never exceed the confirmed profile's safe live ceiling.
/// `Auto` stays compact: 16K on known profiles, 8K on unknown hardware.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalContextPreset {
    Auto,
    K8,
    K16,
    K32,
    K64,
    K96,
}

impl LocalContextPreset {
    #[must_use]
    pub fn from_config(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "8k" => Self::K8,
            "16k" => Self::K16,
            "32k" => Self::K32,
            "64k" => Self::K64,
            "96k" => Self::K96,
            _ => Self::Auto,
        }
    }

    #[must_use]
    pub const fn as_config(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::K8 => "8k",
            Self::K16 => "16k",
            Self::K32 => "32k",
            Self::K64 => "64k",
            Self::K96 => "96k",
        }
    }

    #[must_use]
    pub const fn from_index(index: i32) -> Self {
        match index {
            1 => Self::K8,
            2 => Self::K16,
            3 => Self::K32,
            4 => Self::K64,
            5 => Self::K96,
            _ => Self::Auto,
        }
    }

    #[must_use]
    pub const fn index(self) -> i32 {
        match self {
            Self::Auto => 0,
            Self::K8 => 1,
            Self::K16 => 2,
            Self::K32 => 3,
            Self::K64 => 4,
            Self::K96 => 5,
        }
    }

    #[must_use]
    pub fn context_tokens(self, profile: HardwareModelProfile, _prep: bool) -> u32 {
        let safe_live = profile.context_tokens(false);
        match self {
            Self::Auto => 16_384.min(safe_live),
            Self::K8 => 8_192.min(safe_live),
            Self::K16 => 16_384.min(safe_live),
            Self::K32 => 32_768.min(safe_live),
            Self::K64 => 65_536.min(safe_live),
            Self::K96 => 98_304.min(safe_live),
        }
    }

    /// Approximate VRAM change against Auto. A real 26B/F16-KV measurement on
    /// RTX 5060 Ti showed ~675 MiB per 32K context step.
    #[must_use]
    pub fn estimated_vram_delta_mib(self, profile: HardwareModelProfile) -> i32 {
        let auto = Self::Auto.context_tokens(profile, false) as i64;
        let selected = self.context_tokens(profile, false) as i64;
        ((selected - auto) * 675 / 32_768) as i32
    }
}

/// Explicit model selected for Suflyor's managed llama.cpp server.
///
/// `ai_local_model` already stores the active GGUF name, so keeping the third
/// state here avoids a config migration. `ai_local_quality` remains the
/// backwards-compatible 26B flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedModel {
    Legacy4B,
    Fallback12B,
    Primary26B,
}

impl ManagedModel {
    #[must_use]
    pub fn from_config(model_id: &str, quality: bool) -> Self {
        if quality {
            Self::Primary26B
        } else if model_id.eq_ignore_ascii_case(LEGACY_GEMMA_FILE) {
            Self::Legacy4B
        } else {
            Self::Fallback12B
        }
    }

    #[must_use]
    pub const fn from_index(index: i32) -> Self {
        match index {
            0 => Self::Legacy4B,
            2 => Self::Primary26B,
            _ => Self::Fallback12B,
        }
    }

    #[must_use]
    pub const fn index(self) -> i32 {
        match self {
            Self::Legacy4B => 0,
            Self::Fallback12B => 1,
            Self::Primary26B => 2,
        }
    }

    #[must_use]
    pub const fn file_name(self) -> &'static str {
        match self {
            Self::Legacy4B => LEGACY_GEMMA_FILE,
            Self::Fallback12B => GEMMA_FILE,
            Self::Primary26B => GEMMA26_FILE,
        }
    }

    #[must_use]
    pub const fn is_quality(self) -> bool {
        matches!(self, Self::Primary26B)
    }

    /// The immutable download coordinates + integrity pins for this model. A
    /// model button downloads EXACTLY this spec on any hardware (never /main,
    /// never hardware-redirected) and verifies it byte-for-byte before load.
    #[must_use]
    pub const fn spec(self) -> ModelSpec {
        match self {
            Self::Legacy4B => ModelSpec {
                url: LEGACY_GEMMA_URL,
                file: LEGACY_GEMMA_FILE,
                size: LEGACY_GEMMA_SIZE,
                sha256: LEGACY_GEMMA_SHA256,
                label: "Gemma 4B",
            },
            Self::Fallback12B => ModelSpec {
                url: GEMMA_URL,
                file: GEMMA_FILE,
                size: GEMMA_SIZE,
                sha256: GEMMA_SHA256,
                label: "Gemma 12B QAT",
            },
            Self::Primary26B => ModelSpec {
                url: GEMMA26_URL,
                file: GEMMA26_FILE,
                size: GEMMA26_SIZE,
                sha256: GEMMA26_SHA256,
                label: "Gemma 26B-A4B",
            },
        }
    }
}

/// Immutable download coordinates + integrity pins for one bundled model. Every
/// field is a compile-time constant pinned from a fixed Hugging Face revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelSpec {
    pub url: &'static str,
    pub file: &'static str,
    pub size: u64,
    pub sha256: &'static str,
    /// Short progress label, e.g. "Gemma 4B".
    pub label: &'static str,
}

/// Approximate total VRAM requirement before launch. The 26B baseline and
/// context slope come from the owner's RTX 5060 Ti measurements; 4B/12B are
/// conservative rounded profiles and the UI labels the result as an estimate.
#[must_use]
pub fn estimated_total_vram_mib(
    model: ManagedModel,
    context: LocalContextPreset,
    profile: HardwareModelProfile,
) -> u32 {
    let auto_mib: u32 = match model {
        ManagedModel::Legacy4B => 6_144,
        ManagedModel::Fallback12B => 9_728,
        ManagedModel::Primary26B => 12_238,
    };
    auto_mib
        .saturating_add_signed(context.estimated_vram_delta_mib(profile))
        .max(1)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedLlamaChoice {
    pub model: ManagedModel,
    pub context: LocalContextPreset,
    pub(super) custom_gguf: Option<PathBuf>,
}

impl ManagedLlamaChoice {
    #[must_use]
    pub const fn new(prefer_quality: bool, context: LocalContextPreset) -> Self {
        Self {
            model: if prefer_quality {
                ManagedModel::Primary26B
            } else {
                ManagedModel::Fallback12B
            },
            context,
            custom_gguf: None,
        }
    }

    #[must_use]
    pub const fn for_model(model: ManagedModel, context: LocalContextPreset) -> Self {
        Self {
            model,
            context,
            custom_gguf: None,
        }
    }

    #[must_use]
    pub fn for_custom(path: PathBuf, context: LocalContextPreset) -> Self {
        Self {
            model: ManagedModel::Fallback12B,
            context,
            custom_gguf: Some(path),
        }
    }

    #[must_use]
    pub fn from_config(
        model_id: &str,
        quality: bool,
        custom_gguf: &str,
        context: LocalContextPreset,
    ) -> Self {
        if custom_gguf.trim().is_empty() {
            Self::for_model(ManagedModel::from_config(model_id, quality), context)
        } else {
            Self::for_custom(PathBuf::from(custom_gguf), context)
        }
    }

    #[must_use]
    pub fn with_context(&self, context: LocalContextPreset) -> Self {
        Self {
            model: self.model,
            context,
            custom_gguf: self.custom_gguf.clone(),
        }
    }

    #[must_use]
    pub fn custom_gguf(&self) -> Option<&Path> {
        self.custom_gguf.as_deref()
    }

    #[must_use]
    pub fn is_custom(&self) -> bool {
        self.custom_gguf.is_some()
    }
}

/// Accept only an absolute, non-empty `.gguf` file with the standard magic.
#[must_use]
pub fn valid_custom_gguf_path(value: &str) -> Option<PathBuf> {
    use std::io::Read;

    let path = PathBuf::from(value.trim());
    if !path.is_absolute()
        || !path.is_file()
        || !path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("gguf"))
    {
        return None;
    }
    let mut magic = [0_u8; 4];
    let mut file = std::fs::File::open(&path).ok()?;
    file.read_exact(&mut magic).ok()?;
    (magic == *b"GGUF").then_some(path)
}

#[must_use]
pub fn custom_gguf_display_name(value: &str) -> String {
    valid_custom_gguf_path(value)
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_default()
}

pub(super) fn valid_custom_choice_path(choice: &ManagedLlamaChoice) -> Option<PathBuf> {
    choice
        .custom_gguf()
        .and_then(|path| valid_custom_gguf_path(&path.to_string_lossy()))
}

pub(super) fn custom_choice_alias(choice: &ManagedLlamaChoice) -> Option<String> {
    valid_custom_choice_path(choice).and_then(|path| {
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
    })
}

pub(super) fn effective_llama_choice(
    root: &Path,
    choice: &ManagedLlamaChoice,
) -> ManagedLlamaChoice {
    if let Some(path) = valid_custom_choice_path(choice) {
        return ManagedLlamaChoice::for_custom(path, choice.context);
    }
    let model = effective_verified_managed_model(root, choice.model);
    ManagedLlamaChoice::for_model(model, choice.context)
}

pub(super) fn llama_choice_name(root: &Path, choice: &ManagedLlamaChoice) -> String {
    custom_choice_alias(choice).unwrap_or_else(|| active_local_model_name(root, choice.model))
}
