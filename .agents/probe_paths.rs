use std::path::PathBuf;
fn main() {
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    println!("APPDATA={}", appdata);
    let base = PathBuf::from(&appdata).join("protonsearch").join("v2");
    println!("base={:?}", base);
    for p in [base.clone(), base.join("cache"), base.join("cache").join("icons")] {
        match std::fs::create_dir_all(&p) { Ok(_) => println!("ok create {:?}", p), Err(e) => println!("ERR create {:?}: {} (raw {})", p, e, e.raw_os_error().unwrap_or(-1)) }
    }
    match std::fs::write(base.join("probe-rust.txt"), b"ok") { Ok(_) => println!("ok write"), Err(e) => println!("ERR write: {} (raw {})", e, e.raw_os_error().unwrap_or(-1)) }
    let _ = std::fs::remove_file(base.join("probe-rust.txt"));
    println!("cwd={:?}", std::env::current_dir());
}
