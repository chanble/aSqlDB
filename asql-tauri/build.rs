use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    if env::var("PROFILE").unwrap_or_default() == "release" {
        build_sidecar();
    }
    tauri_build::build();
}

fn build_sidecar() {
    let workspace_root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .expect("asql-tauri must be inside a workspace")
        .to_path_buf();

    let target_triple = env::var("TARGET").expect("TARGET env var must be set");
    let binary_name = format!("asql-web{}", std::env::consts::EXE_SUFFIX);

    let out_dir = workspace_root.join("target").join("release");
    let source = out_dir.join(&binary_name);

    println!("cargo:rerun-if-changed=../asql-web/src");
    println!("cargo:rerun-if-changed=../asql-web/Cargo.toml");

    let status = Command::new("cargo")
        .arg("build")
        .arg("--bin")
        .arg("asql-web")
        .arg("--release")
        .current_dir(&workspace_root)
        .status()
        .expect("failed to execute cargo build for asql-web sidecar");

    if !status.success() {
        panic!("asql-web sidecar build failed");
    }

    if !source.exists() {
        panic!("asql-web sidecar binary not found at {}", source.display());
    }

    let binaries_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("binaries");
    std::fs::create_dir_all(&binaries_dir).expect("failed to create binaries directory");

    let sidecar_name = format!("asql-web-{target_triple}{}", std::env::consts::EXE_SUFFIX);
    let destination = binaries_dir.join(&sidecar_name);

    std::fs::copy(&source, &destination).expect("failed to copy sidecar binary");

    println!("cargo:rustc-env=ASQL_WEB_SIDECAR_PATH={}", destination.display());
}
