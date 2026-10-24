// Original Windhawk mod by caliberda
// Ported to standalone Rust by Vigi Riyantanu

use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=app.manifest");
    println!("cargo:rerun-if-changed=app.rc");
    println!("cargo:rerun-if-changed=app.ico");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();

    if target_os == "windows" && target_env == "msvc" {
        let manifest = Path::new("app.manifest").canonicalize().unwrap();
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());

        // Compile icon resource so the executable has the angel icon in Windows Explorer
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let res_path = Path::new(&out_dir).join("app.res");

        let rc_paths = [
            "C:\\Program Files (x86)\\Windows Kits\\10\\bin\\10.0.26100.0\\x64\\rc.exe",
            "C:\\Program Files (x86)\\Windows Kits\\10\\bin\\10.0.22621.0\\x64\\rc.exe",
            "C:\\Program Files (x86)\\Windows Kits\\10\\bin\\10.0.19041.0\\x64\\rc.exe",
        ];

        let mut compiled = false;
        for path in rc_paths {
            if Path::new(path).exists() {
                let status = Command::new(path)
                    .args(["/fo", res_path.to_str().unwrap(), "app.rc"])
                    .status();
                if let Ok(s) = status {
                    if s.success() {
                        println!("cargo:rustc-link-arg={}", res_path.display());
                        compiled = true;
                        break;
                    }
                }
            }
        }

        if !compiled && Path::new("app.res").exists() {
            let res_canonical = Path::new("app.res").canonicalize().unwrap();
            println!("cargo:rustc-link-arg={}", res_canonical.display());
        }
    }
}
