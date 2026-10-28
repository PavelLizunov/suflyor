//! Execution-provider preference for the optional CUDA sidecar build.

pub fn preferred() -> &'static str {
    if cfg!(all(target_os = "linux", feature = "cuda")) {
        "cuda"
    } else {
        "cpu"
    }
}
