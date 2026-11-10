use std::{env, path::PathBuf, process::Command};
fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let arch = if env::var("CARGO_CFG_TARGET_ARCH").unwrap() == "aarch64" {
            "arm64-apple-macos13.0"
        } else {
            "x86_64-apple-macos13.0"
        };
        let out = PathBuf::from(env::var("OUT_DIR").unwrap());
        let object = out.join("Mac.o");
        let status = Command::new("xcrun")
            .args([
                "swiftc",
                "-parse-as-library",
                "-emit-object",
                "-O",
                "-target",
                arch,
                "macos/Mac.swift",
                "-o",
            ])
            .arg(&object)
            .status()
            .expect("Swift compiler required");
        assert!(status.success(), "Swift integration did not compile");
        assert!(Command::new("ar")
            .arg("crs")
            .arg(out.join("libfilepilot_macos.a"))
            .arg(&object)
            .status()
            .unwrap()
            .success());
        println!("cargo:rustc-link-search=native={}", out.display());
        println!("cargo:rustc-link-lib=static=filepilot_macos");
        let info = Command::new("xcrun")
            .args(["swiftc", "-print-target-info"])
            .output()
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&info.stdout).unwrap();
        for path in json["paths"]["runtimeLibraryPaths"].as_array().unwrap() {
            println!("cargo:rustc-link-search=native={}", path.as_str().unwrap());
        }
        println!("cargo:rustc-link-lib=dylib=swiftCore");
        for name in ["Foundation", "AppKit", "ServiceManagement"] {
            println!("cargo:rustc-link-lib=framework={name}");
        }
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    }
    println!("cargo:rerun-if-changed=macos/Mac.swift");
    tauri_build::build();
}
