fn main() {
    // Compile the KCP C source code using the `cc` crate
    cc::Build::new()
        .file("kcp/ikcp.c")
        .include("kcp")
        // Optimization: use O2 in release, O0 in debug
        .opt_level_str(if cfg!(debug_assertions) { "0" } else { "2" })
        // Suppress common C warnings that don't affect correctness
        .warnings(false)
        // Define standard compilation flags
        .define("NDEBUG", None) // disable C asserts in KCP
        .compile("kcp");

    // Rebuild if C source files change
    println!("cargo:rerun-if-changed=kcp/ikcp.c");
    println!("cargo:rerun-if-changed=kcp/ikcp.h");
}