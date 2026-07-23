use std::env;
use std::path::{Path, PathBuf};
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

    // Also copy frontend dist so the sidecar can find it at runtime
    let frontend_src = out_dir.join("frontend").join("dist");
    if frontend_src.exists() {
        let frontend_dst = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("frontend_dist");
        if frontend_dst.exists() {
            std::fs::remove_dir_all(&frontend_dst).expect("failed to remove old frontend dist");
        }
        copy_dir(&frontend_src, &frontend_dst);
        println!("cargo:warning=Frontend dist copied to {}", frontend_dst.display());
    } else {
        println!("cargo:warning=Frontend dist not found at {}", frontend_src.display());
    }

    println!("cargo:rustc-env=ASQL_WEB_SIDECAR_PATH={}", destination.display());
}

fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).expect("failed to create destination dir");
    for entry in std::fs::read_dir(src).expect("failed to read source dir") {
        let entry = entry.expect("failed to read entry");
        let file_type = entry.file_type().expect("failed to read file type");
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir(&src_path, &dst_path);
        } else {
            std::fs::copy(&src_path, &dst_path).expect("failed to copy file");
        }
    }
}
