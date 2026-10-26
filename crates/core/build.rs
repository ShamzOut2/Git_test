fn main() {
    // rust-embed requires the folder to exist; an empty one serves a "UI not built" hint.
    let dist = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ui/dist");
    let _ = std::fs::create_dir_all(&dist);
    println!("cargo:rerun-if-changed=../../ui/dist");
}
