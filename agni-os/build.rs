fn main() {
    // 1. Force Skylake or newer (AVX2 support) or generic x86-64-v3 for math safety
    println!("cargo:rustc-env=RUSTFLAGS=-C target-cpu=x86-64-v3");

    // 2. Fail build if on 32-bit (Safety math requires 64-bit precision)
    #[cfg(target_pointer_width = "32")]
    compile_error!("AgNiX requires 64-bit architecture for precision safety.");
}
