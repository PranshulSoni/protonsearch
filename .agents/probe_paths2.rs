use std::path::PathBuf;
fn try_write(label: &str, p: PathBuf) {
    let w = p.join("probe.txt");
    match std::fs::write(&w, b"ok") {
        Ok(_) => { println!("OK   {} -> {:?}", label, w); let _ = std::fs::remove_file(&w); }
        Err(e) => println!("ERR  {} -> {:?}: {} (raw {})", label, w, e, e.raw_os_error().unwrap_or(-1)),
    }
}
fn main() {
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    let base = PathBuf::from(&appdata).join("protonsearch");
    try_write("appdata/protonsearch", base.clone());
    try_write("appdata/protonsearch/v2", base.join("v2"));
    try_write("appdata/protonsearch/v2/cache", base.join("v2").join("cache"));
    try_write("appdata root", PathBuf::from(&appdata));
    try_write("localappdata", PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default()));
    try_write("temp", std::env::temp_dir());
    try_write("repo target", PathBuf::from("target"));
    try_write("user profile", PathBuf::from(std::env::var("USERPROFILE").unwrap_or_default()));
    println!("token integrity: {:?}", std::env::var("USERNAME"));
}
