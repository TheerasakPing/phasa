fn main() {
    #[cfg(target_os = "macos")]
    {
        cc::Build::new()
            .file("src/ui/dashboard.m")
            .flag("-fobjc-arc")
            .compile("phasa_ui");

        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=Foundation");
    }
}
