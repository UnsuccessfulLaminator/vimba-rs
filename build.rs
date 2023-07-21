fn main() {
    // The libdir file has simply the directory of the library.
    // It is updated by the shell script in the root directory.
    let libdir = include_str!("libdir");

    println!("cargo:rerun-if-changed=libdir");
    println!("cargo:rustc-link-lib=dylib=VimbaC");
    println!("cargo:rustc-link-search=native={libdir}");
}
