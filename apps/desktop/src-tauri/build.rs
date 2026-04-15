use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    prepare_sidecar_binary();
    tauri_build::build();
}

fn prepare_sidecar_binary() {
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be available"),
    );
    let workspace_root = workspace_root_from_manifest(&manifest_dir);
    let target_triple = env::var("TARGET").expect("TARGET should be available");
    let profile = env::var("PROFILE").expect("PROFILE should be available");
    let target_dir = resolve_target_dir(&workspace_root);

    let extension = if target_triple.contains("windows") {
        ".exe"
    } else {
        ""
    };

    let source_binary_path = target_dir
        .join(&target_triple)
        .join(&profile)
        .join(format!("runtime-sidecar{extension}"));
    let destination_binary_path = manifest_dir
        .join("binaries")
        .join(format!("runtime-sidecar-{target_triple}{extension}"));

    fs::create_dir_all(
        destination_binary_path
            .parent()
            .expect("sidecar destination should have a parent directory"),
    )
    .expect("failed to create sidecar destination directory");

    if source_binary_path.exists() {
        fs::copy(&source_binary_path, &destination_binary_path)
            .expect("failed to copy sidecar binary");
    } else if profile == "release" {
        panic!(
            "runtime-sidecar binary missing for release build, expected at: {}",
            source_binary_path.display()
        );
    } else if !destination_binary_path.exists() {
        // Keep check/test/dev flows unblocked; release build requires real sidecar artifact.
        fs::write(&destination_binary_path, b"")
            .expect("failed to create placeholder sidecar binary for non-release profile");
    }

    println!("cargo:rerun-if-changed={}", workspace_root.join("crates/runtime-sidecar/src/main.rs").display());
    println!("cargo:rerun-if-changed={}", workspace_root.join("crates/runtime-sidecar/Cargo.toml").display());
}

fn workspace_root_from_manifest(manifest_dir: &Path) -> PathBuf {
    manifest_dir
        .join("..")
        .join("..")
        .join("..")
        .canonicalize()
        .expect("failed to resolve workspace root from src-tauri manifest directory")
}

fn resolve_target_dir(workspace_root: &Path) -> PathBuf {
    match env::var_os("CARGO_TARGET_DIR") {
        Some(path) => {
            let path = PathBuf::from(path);
            if path.is_absolute() {
                path
            } else {
                workspace_root.join(path)
            }
        }
        None => workspace_root.join("target"),
    }
}
