use std::fs;
use std::env;
use winres::WindowsResource;
fn main() {
    // Get the current build date and time
    let build_date = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();

    // Write the build date to a file
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = format!("{}/build_date.txt", out_dir);
    fs::write(&dest_path, &build_date).unwrap();

    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        // Add a version resource to the executable
        let mut res = WindowsResource::new();
        if !cfg!(windows) && env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
            let prefix = match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
                Ok("x86") => "i686-w64-mingw32",
                _ => "x86_64-w64-mingw32",
            };
            res.set_windres_path(&format!("{}-windres", prefix));
            res.set_ar_path(&format!("{}-ar", prefix));
        }
        res.set_icon("assets/Bootstrapper.ico");
        res.set_language(0x0409); // US English
        res.set("FileVersion", env!("CARGO_PKG_VERSION"));
        res.set("FileDescription", "SYNTAX Windows Bootstrapper");
        res.set("ProductName", "SYNTAX Bootstrapper");
        res.set("ProductVersion", env!("CARGO_PKG_VERSION"));
        res.set("InternalName", "SYNTAX Bootstrapper");
        res.set("OriginalFilename", "SyntaxPlayerLauncher.exe");
        res.set("CompanyName", "SYNTAX Corporation");
        res.set("LegalCopyright", "Copyright (c) 2023");
        res.compile().unwrap();
        if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
            println!("cargo:rustc-link-arg-bins={}/resource.o", out_dir);
        }
    }

    println!("cargo:rerun-if-changed=build.rs");
}