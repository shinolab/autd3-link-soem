#[cfg(feature = "local")]
fn main() -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    let target = std::env::var("TARGET").unwrap();

    let os = if cfg!(target_os = "windows") {
        "win32"
    } else if cfg!(target_os = "macos") {
        "macosx"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        panic!("Unsupported OS");
    };

    let mut build = cc::Build::new();
    build.cpp(false);

    glob::glob("3rdparty/SOEM/soem/*.c")?.try_fold(&mut build, |build, entry| {
        let build = build.file(entry?);
        Result::<_, glob::GlobError>::Ok(build)
    })?;
    glob::glob(&format!("3rdparty/SOEM/osal/{}/*.c", os))?.try_fold(
        &mut build,
        |build, entry| {
            let build = build.file(entry?);
            Result::<_, glob::GlobError>::Ok(build)
        },
    )?;
    glob::glob(&format!("3rdparty/SOEM/oshw/{}/*.c", os))?.try_fold(
        &mut build,
        |build, entry| {
            let build = build.file(entry?);
            Result::<_, glob::GlobError>::Ok(build)
        },
    )?;
    build
        .include("3rdparty/SOEM/soem")
        .include("3rdparty/SOEM/osal")
        .include(format!("3rdparty/SOEM/osal/{}", os))
        .include(format!("3rdparty/SOEM/oshw/{}", os));
    #[cfg(target_os = "windows")]
    {
        build
            .include("3rdparty/SOEM/oshw/win32/wpcap/Include")
            .include("3rdparty/SOEM/oshw/win32/wpcap/Include/pcap")
            .flag("/DWIN32");
        if target.contains("arm") || target.contains("aarch64") {
            build.target("aarch64-pc-windows-msvc");
        }
    }
    build.compile("soem");

    #[cfg(target_os = "windows")]
    {
        let home_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();

        println!("cargo:rustc-link-lib=winmm");
        println!("cargo:rustc-link-lib=ws2_32");
        if target.contains("arm") || target.contains("aarch64") {
            println!("cargo:rustc-link-search={home_dir}\\Lib\\ARM64");
        } else {
            println!("cargo:rustc-link-search={home_dir}\\Lib\\x64");
        }
        println!("cargo:rustc-link-lib=wpcap");
    }
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=pthread");
        println!("cargo:rustc-link-lib=pcap");
    }
    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-lib=pthread");
        println!("cargo:rustc-link-lib=rt");
    }

    Ok(())
}

#[cfg(not(feature = "local"))]
fn main() {}
