fn main() {
    let sdk_path = std::env::var("HARMONY_SDK_HOME")
        .unwrap_or_else(|_| "/Applications/DevEco-Studio.app/Contents/sdk/default".to_string());
    println!("cargo:rustc-link-search=native={}/hms/native/sysroot/usr/lib/aarch64-linux-ohos", sdk_path);
    println!("cargo:rustc-link-lib=ace_napi.z");
    println!("cargo:rustc-link-lib=hilog_ndk.z");
    println!("cargo:rustc-link-lib=rcp_c");
    println!("cargo:rustc-link-lib=net_websocket");
}
