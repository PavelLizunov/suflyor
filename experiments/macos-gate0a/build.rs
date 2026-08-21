fn main() -> Result<(), Box<dyn std::error::Error>> {
    let debug_info = std::env::var("PROFILE").is_ok_and(|profile| profile != "release")
        || std::env::var("SLINT_EMIT_DEBUG_INFO").is_ok_and(|value| value == "1");
    let config = slint_build::CompilerConfiguration::new().with_debug_info(debug_info);
    slint_build::compile_with_config("ui/gate0a.slint", config)?;

    #[cfg(target_os = "macos")]
    {
        cc::Build::new()
            .file("src/appkit.m")
            .flag("-fobjc-arc")
            .compile("suflyor_gate0a_appkit");
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rerun-if-changed=src/appkit.m");
    }

    Ok(())
}
