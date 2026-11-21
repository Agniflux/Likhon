//! Embeds the keyboard's icon and version information (likhon-tsf.rc), and exports the four COM entry points as
//! private (likhon_tsf.def), as regsvr32 and COM expect.
fn main() {
    embed_resource::compile("likhon-tsf.rc", embed_resource::NONE).manifest_optional().unwrap();
    let def = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("likhon_tsf.def");
    println!("cargo:rustc-cdylib-link-arg=/DEF:{}", def.display());
    println!("cargo:rerun-if-changed=likhon-tsf.rc");
    println!("cargo:rerun-if-changed=likhon_tsf.def");
}
