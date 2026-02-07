fn main() {
    // OpenSSL の静的リンクに必要な Windows システムライブラリ
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-lib=crypt32");
        println!("cargo:rustc-link-lib=advapi32");
        println!("cargo:rustc-link-lib=user32");
    }
}
