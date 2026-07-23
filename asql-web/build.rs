use std::path::Path;
use std::process::Command;

fn npm() -> &'static str {
    if cfg!(windows) { "npm.cmd" } else { "npm" }
}

fn main() {
    // Only build frontend in release mode (debug uses `npm run dev`)
    let profile = std::env::var("PROFILE").unwrap_or_default();
    if profile != "release" {
        return;
    }

    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_dir = Path::new(&manifest);
    let frontend_dir = manifest_dir.join("frontend");
    let dist_dir = frontend_dir.join("dist");

    println!("cargo:warning=Building frontend...");

    // Install dependencies if needed
    if !frontend_dir.join("node_modules").exists() {
        let status = Command::new(npm())
            .arg("ci")
            .current_dir(&frontend_dir)
            .status()
            .expect("failed to run npm ci");
        if !status.success() {
            panic!("npm ci failed");
        }
    }

    // Build frontend
    let status = Command::new(npm())
        .arg("run")
        .arg("build")
        .current_dir(&frontend_dir)
        .status()
        .expect("failed to run npm build");
    if !status.success() {
        panic!("npm build failed");
    }

    // Copy to target/release/frontend/dist so the binary can find it
    let target_dir = manifest_dir
        .parent()
        .unwrap()
        .join("target")
        .join("release");
    let dest = target_dir.join("frontend").join("dist");

    if dest.exists() {
        std::fs::remove_dir_all(&dest).expect("failed to remove old frontend dist in target");
    }
    std::fs::create_dir_all(dest.parent().unwrap()).expect("failed to create target frontend dir");
    copy_dir(&dist_dir, &dest);

    println!(
        "cargo:warning=Frontend built and copied to {}",
        dest.display()
    );
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
