fn main() {
    println!("cargo:rustc-link-search=native=/Applications/DevEco-Studio.app/Contents/sdk/default/hms/native/sysroot/usr/lib/aarch64-linux-ohos");
    println!("cargo:rustc-link-lib=ace_napi.z");
    println!("cargo:rustc-link-lib=hilog_ndk.z");
    println!("cargo:rustc-link-lib=rcp_c");
    println!("cargo:rustc-link-lib=net_websocket");
}
