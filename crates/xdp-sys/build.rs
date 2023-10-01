use std::env;
use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = env::var("OUT_DIR")?;
    let out_path = PathBuf::from(out_dir).join("bindings.rs");

    println!("cargo:rustc-link-search=include/linux-6.5.4/");
    println!("cargo:rerun-if-changed=include/linux-6.5.4/wrapper.h");

    bindgen::Builder::default()
        .header("include/linux-6.5.4/wrapper.h")
        .generate()?
        .write_to_file(out_path)?;

    Ok(())
}
