//! Put the two big fonts into `OUT_DIR`, where `lib.rs` `include_bytes!`es them. They are
//! too large for this crate's crates.io upload (10 MB), so they come from build-dependency
//! crates published next to it: `uzor-fonts-emoji` (Noto Color Emoji) and
//! `uzor-fonts-cjk-a` + `uzor-fonts-cjk-b` (Noto Sans CJK SC in two halves). No network:
//! builds work offline, behind proxies and on docs.rs.

use std::path::Path;

fn put(out: &Path, name: &str, parts: &[&[u8]], len: usize) {
    let total: usize = parts.iter().map(|p| p.len()).sum();
    assert_eq!(total, len, "uzor-fonts build.rs: {name} is {total} bytes, expected {len}");
    let dest = out.join(name);
    // Rewrite only on change: a fresh mtime would rebuild `uzor-fonts` for nothing.
    let bytes = parts.concat();
    if std::fs::read(&dest).is_ok_and(|have| have == bytes) {
        return;
    }
    std::fs::write(&dest, &bytes)
        .unwrap_or_else(|e| panic!("uzor-fonts build.rs: writing {}: {e}", dest.display()));
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let out = std::env::var("OUT_DIR").expect("OUT_DIR must be set by cargo");
    let out = Path::new(&out);
    put(out, "NotoColorEmoji.ttf", &[uzor_fonts_emoji::NOTO_COLOR_EMOJI], 10_673_480);
    put(out, "NotoSansCJKsc-Regular.otf", &[uzor_fonts_cjk_a::PART, uzor_fonts_cjk_b::PART], 16_437_364);
}
