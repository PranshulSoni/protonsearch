fn main() {
    if cfg!(target_os = "windows") {
        copy_directml();
    }
}

#[cfg(target_os = "windows")]
fn copy_directml() {
    let local_app_data = match std::env::var("LOCALAPPDATA") {
        Ok(v) => v,
        Err(_) => return,
    };

    let ort_cache = std::path::Path::new(&local_app_data)
        .join("ort.pyke.io")
        .join("dfbin")
        .join("x86_64-pc-windows-msvc");

    let dll = find_directml(&ort_cache);
    if dll.is_none() {
        println!("cargo:warning=Could not find DirectML.dll in ort cache at {}", ort_cache.display());
        return;
    }
    let dll = dll.unwrap();

    // OUT_DIR is target/debug/build/opensearch-os-xxx/out — go up 3 to reach target/debug/
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let target_dir = std::path::Path::new(&out_dir)
        .ancestors()
        .nth(3)
        .unwrap()
        .to_path_buf();

    for dir in [
        target_dir.clone(),
        target_dir.join("deps"),
        target_dir.join("examples"),
    ] {
        let dst = dir.join("DirectML.dll");
        // Remove symlink or stale copy
        if dst.exists() || dst.is_symlink() {
            let _ = std::fs::remove_file(&dst);
        }
        if !dst.exists() {
            if let Err(e) = std::fs::copy(&dll, &dst) {
                println!("cargo:warning=Failed to copy DirectML.dll to {}: {e}", dst.display());
            }
        }
    }

    println!("cargo:rerun-if-changed=build.rs");
}

fn find_directml(base: &std::path::Path) -> Option<std::path::PathBuf> {
    let entries = std::fs::read_dir(base).ok()?;
    for entry in entries.flatten() {
        let candidate = entry.path().join("DirectML.dll");
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}
