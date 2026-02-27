fn main() {
    println!("cargo:rerun-if-changed=native/g711_ref.c");
    cc::Build::new()
        .file("native/g711_ref.c")
        .compile("g711_ref");
}
