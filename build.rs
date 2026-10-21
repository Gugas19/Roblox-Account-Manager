fn main() {
    tauri_build::build();
    // test binaries that reach the app's dialogs (the live check of the Roblox requests) need the same Windows common
    // controls as the app, or Windows refuses to start them (STATUS_ENTRYPOINT_NOT_FOUND)
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg-tests=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'");
    }
}
