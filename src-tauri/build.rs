use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let source_cli_path = manifest_dir
        .join("target")
        .join("release")
        .join("codex_switch.exe");

    if source_cli_path.exists() {
        println!(
            "cargo:rustc-env=CODEX_SWITCH_RELEASE_EXE={}",
            source_cli_path.display()
        );
    }

    build_macos_native_tray();

    tauri_build::build()
}

#[cfg(target_os = "macos")]
fn build_macos_native_tray() {
    swift_rs::SwiftLinker::new("12.0")
        .with_package("CodexSwitchNativeTray", "macos-native-tray")
        .link();
}

#[cfg(not(target_os = "macos"))]
fn build_macos_native_tray() {}
