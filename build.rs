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

        println!("cargo:rustc-link-lib=winmm");
        println!("cargo:rustc-link-lib=ws2_32");
        println!("cargo:rustc-link-search={home_dir}/3rdparty/SOEM/oshw/win32/wpcap/Lib/x64");
        println!("cargo:rustc-link-lib=wpcap");
    }
    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-lib=pthread");
        println!("cargo:rustc-link-lib=rt");
    }
}
