fn main() {
    println!("cargo:rerun-if-changed=ui/settings.slint");
    println!("cargo:rerun-if-changed=ui/app.slint");
    println!("cargo:rerun-if-changed=../assets/brand/runtime/protonsearch.ico");
    println!("cargo:rerun-if-changed=../assets/brand/runtime/protonsearch-1024.png");
    println!("cargo:rerun-if-changed=../assets/brand/runtime/protonsearch-128.png");
    println!("cargo:rerun-if-changed=../assets/brand/runtime/protonsearch-16.png");
    println!("cargo:rerun-if-changed=../assets/brand/runtime/protonsearch-32.png");

    slint_build::compile("ui/app.slint").unwrap();

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../assets/brand/runtime/protonsearch.ico");
        res.set_language(0x0409); // U.S. English
        res.set("FileDescription", "ProtonSearch Launcher");
        res.set("ProductName", "ProtonSearch");
        res.set("OriginalFilename", "ProtonSearch.exe");
        res.set("CompanyName", "Pranshul Soni");
        res.set("LegalCopyright", "Copyright (c) 2026 Pranshul Soni");
        res.compile().unwrap();
    }
}
