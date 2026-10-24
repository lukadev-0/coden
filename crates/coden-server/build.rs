use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());

    println!("cargo::rerun-if-env-changed=COPYRIGHT_HTML");
    println!("cargo::rustc-check-cfg=cfg(copyright_page)");

    if let Some(copyright_html) = std::env::var_os("COPYRIGHT_HTML")
        && !copyright_html.is_empty()
    {
        std::fs::copy(copyright_html, out_dir.join("copyright.html")).unwrap();
        println!("cargo::rustc-cfg=copyright_page");
    }
}
