fn main() {
    println!("cargo:rerun-if-changed=ui/settings.slint");
    println!("cargo:rerun-if-changed=../icons/ProtonSearchTrans.ico");
    println!("cargo:rerun-if-changed=../icons/ProtonSearchTrans.png");
    println!("cargo:rerun-if-changed=../icons/ProtonSearchTrans_small.png");
    println!("cargo:rerun-if-changed=../icons/ProtonSearchTrans_16.png");
    println!("cargo:rerun-if-changed=../icons/ProtonSearchTrans_32.png");

    slint_build::compile("ui/settings.slint").unwrap();

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../icons/ProtonSearchTrans.ico");
        res.set_language(0x0409); // U.S. English
        res.set("FileDescription", "ProtonSearch Launcher");
        res.set("ProductName", "ProtonSearch");
        res.set("OriginalFilename", "protonsearch.exe");
        res.set("CompanyName", "Pranshul Soni");
        res.set("LegalCopyright", "Copyright (c) 2026 Pranshul Soni");
        res.compile().unwrap();
    }
}
