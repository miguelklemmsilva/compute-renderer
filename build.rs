use std::fs;
use std::path::Path;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("assets");

    // Copy assets folder to the output directory
    fs::create_dir_all(&dest_path).unwrap();
    fs::copy("assets/african_head.obj", dest_path.join("african_head.obj")).unwrap();
    fs::copy("assets/african_head_diffuse.tga", dest_path.join("african_head_diffuse.tga")).unwrap();

    println!("cargo:rerun-if-changed=assets/");
}