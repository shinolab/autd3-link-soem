#[cfg(feature = "local")]
fn main() {
    println!("cargo:rerun-if-changed=3rdparty/SOEM");

    let dst = cmake::build("3rdparty/SOEM");
    println!(
        "cargo:rustc-link-search=native={}",
        dst.join("lib").display()
    );
    println!("cargo:rustc-link-lib=static=soem");

    #[cfg(target_os = "windows")]
    {
        let home_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let target = std::env::var("TARGET").unwrap();

        println!("cargo:rustc-link-lib=winmm");
        println!("cargo:rustc-link-lib=ws2_32");
        if target.contains("arm") || target.contains("aarch64") {
            println!("cargo:rustc-link-search={home_dir}\\Lib\\ARM64");
        } else {
            println!("cargo:rustc-link-search={home_dir}\\Lib\\x64");
        }
        println!("cargo:rustc-link-lib=wpcap");
    }
    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-lib=pthread");
        println!("cargo:rustc-link-lib=rt");
    }
}

#[cfg(not(feature = "local"))]
fn main() {}
