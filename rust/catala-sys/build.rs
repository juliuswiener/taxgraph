// rust/catala-sys/build.rs — compiles the committed, generated Catala C backend
// (`generated/`, refreshed by `gen.sh` / `make catala-c`) plus the hand-written shim
// (`csrc/shim.c`) into a static library and links GMP dynamically (LGPL, REWRITE_PLAN.md §3).
//
// `-std=c89`: GCC 16 rejects `catala_runtime.c` under a newer standard — `gmp.h` only
// declares `gmp_vprintf`/friends when `stdarg.h` is already visible, and the runtime includes
// them the other way round (Audit C Teil 2.3 Schritt B). `clerk` itself builds with c89 for
// the same reason.
use std::path::PathBuf;

fn main() -> std::io::Result<()> {
    let manifest_dir = PathBuf::from(env_or("CARGO_MANIFEST_DIR", "."));
    let generated = manifest_dir.join("generated");

    println!("cargo:rerun-if-changed=generated");
    println!("cargo:rerun-if-changed=csrc/shim.c");

    let mut build = cc::Build::new();
    build
        .include(&generated)
        .std("c89")
        .warnings(false)
        .file("csrc/shim.c");

    for entry in std::fs::read_dir(&generated)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) == Some("c") {
            build.file(path);
        }
    }

    build.compile("catala_taxrules");
    // GMP is LGPL — dynamic link only (REWRITE_PLAN.md §3 "GMP").
    println!("cargo:rustc-link-lib=dylib=gmp");
    Ok(())
}

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}
