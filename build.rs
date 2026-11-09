fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        // cargo-quad-apk uses ld directly; keep Oboe's C++ runtime in the APK's
        // native library rather than relying on a system libc++ on the device.
        println!("cargo:rustc-link-lib=static:+whole-archive=c++_static");
        println!("cargo:rustc-link-lib=static:+whole-archive=c++abi");
    }
}
