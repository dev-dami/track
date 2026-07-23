fn main() {
    // Prefer static LLVM linking so release binaries don't require system LLVM libraries
    println!("cargo:rustc-env=LLVM_SYS_221_PREFER_STATIC=1");
    println!("cargo:rerun-if-changed=build.rs");
}
