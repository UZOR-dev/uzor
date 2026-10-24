//! The opt-in heavy fonts (features `color-emoji`, `cjk`) into `OUT_DIR`, where `lib.rs`
//! `include_bytes!`es them. Without those features this script does nothing.
//!
//! Source, first match wins:
//! 1. `$UZOR_FONTS_DIR/<file>` when `UZOR_FONTS_DIR` is set (offline / vendored builds);
//! 2. on docs.rs (`DOCS_RS`): nothing; `lib.rs` uses the light stand-ins instead;
//! 3. `curl` from `$UZOR_FONTS_URL/<file>`, by default the pinned commit of
//!    https://github.com/UZOR-dev/uzor-fonts.
//!
//! Every file is checked against its SHA-256 below, whatever its source. Any failure stops
//! the build and says what to do; nothing ever ends up half-written or empty.

#[cfg(any(feature = "color-emoji", feature = "cjk"))]
mod heavy {
    use std::path::Path;
    use std::process::Command;

    /// Commit of github.com/UZOR-dev/uzor-fonts the default URLs point at.
    const REPO_COMMIT: &str = "432ffbe6edfcdb99740d50be93b8e08c9cf7ee24";

    pub struct Font {
        pub file: &'static str,
        pub sha256: &'static str,
        pub feature: &'static str,
    }

    pub const COLOR_EMOJI: Font = Font {
        file: "NotoColorEmoji.ttf",
        sha256: "72a635cb3d2f3524c51620cdde406b217204e8a6a06c6a096ff8ed4b5fd6e27b",
        feature: "color-emoji",
    };
    pub const CJK_SC: Font = Font {
        file: "NotoSansCJKsc-Regular.otf",
        sha256: "2c76254f6fc379fddfce0a7e84fb5385bb135d3e399294f6eeb6680d0365b74b",
        feature: "cjk",
    };

    /// FIPS 180-4 SHA-256 (no build-dependency, so the features work in any offline setup).
    pub fn sha256_hex(data: &[u8]) -> String {
        const K: [u32; 64] = [
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
            0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
            0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
            0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
            0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
            0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
            0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
        ];
        let mut h: [u32; 8] = [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
        ];
        let mut tail = data[data.len() / 64 * 64..].to_vec();
        tail.push(0x80);
        while tail.len() % 64 != 56 {
            tail.push(0);
        }
        tail.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
        for block in data[..data.len() / 64 * 64].chunks_exact(64).chain(tail.chunks_exact(64)) {
            let mut w = [0u32; 64];
            for (i, c) in block.chunks_exact(4).enumerate() {
                w[i] = u32::from_be_bytes([c[0], c[1], c[2], c[3]]);
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
            }
            let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
            for i in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let ch = (e & f) ^ (!e & g);
                let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let maj = (a & b) ^ (a & c) ^ (b & c);
                let t2 = s0.wrapping_add(maj);
                hh = g;
                g = f;
                f = e;
                e = d.wrapping_add(t1);
                d = c;
                c = b;
                b = a;
                a = t1.wrapping_add(t2);
            }
            for (x, y) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
                *x = x.wrapping_add(y);
            }
        }
        h.iter().map(|x| format!("{x:08x}")).collect()
    }

    fn fail(font: &Font, why: String) -> ! {
        panic!(
            "\nuzor-fonts: feature `{}` needs {} and {why}.\n\
             Either put the file (sha256 {}) in a directory and set UZOR_FONTS_DIR to it,\n\
             or allow the download from https://github.com/UZOR-dev/uzor-fonts (needs curl),\n\
             or turn the feature off (the light stand-in font is used then).\n",
            font.feature, font.file, font.sha256
        )
    }

    /// Ok(true): `dest` holds the pinned bytes; Ok(false): skipped (docs.rs).
    pub fn provide(font: &Font, out: &Path) -> bool {
        let dest = out.join(font.file);
        if std::fs::read(&dest).is_ok_and(|b| sha256_hex(&b) == font.sha256) {
            return true;
        }
        let bytes = if let Some(dir) = std::env::var_os("UZOR_FONTS_DIR") {
            let src = Path::new(&dir).join(font.file);
            std::fs::read(&src)
                .unwrap_or_else(|e| fail(font, format!("UZOR_FONTS_DIR has no readable {} ({e})", src.display())))
        } else if std::env::var_os("DOCS_RS").is_some() {
            return false;
        } else {
            let base = std::env::var("UZOR_FONTS_URL").unwrap_or_else(|_| {
                format!("https://raw.githubusercontent.com/UZOR-dev/uzor-fonts/{REPO_COMMIT}")
            });
            let url = format!("{}/{}", base.trim_end_matches('/'), font.file);
            let tmp = out.join(format!("{}.part", font.file));
            let status = Command::new("curl")
                .args(["-fsSL", "--retry", "2", "-o"])
                .arg(&tmp)
                .arg(&url)
                .status()
                .unwrap_or_else(|e| fail(font, format!("curl could not be run ({e})")));
            if !status.success() {
                let _ = std::fs::remove_file(&tmp);
                fail(font, format!("downloading {url} failed ({status})"));
            }
            let b = std::fs::read(&tmp).unwrap_or_else(|e| fail(font, format!("reading the download failed ({e})")));
            let _ = std::fs::remove_file(&tmp);
            b
        };
        let got = sha256_hex(&bytes);
        if got != font.sha256 {
            fail(font, format!("the file found has sha256 {got}, not the pinned one"));
        }
        std::fs::write(&dest, &bytes)
            .unwrap_or_else(|e| fail(font, format!("writing {} failed ({e})", dest.display())));
        true
    }
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=UZOR_FONTS_DIR");
    println!("cargo:rerun-if-env-changed=UZOR_FONTS_URL");
    println!("cargo:rustc-check-cfg=cfg(uzor_fonts_color_emoji)");
    println!("cargo:rustc-check-cfg=cfg(uzor_fonts_cjk)");
    #[cfg(any(feature = "color-emoji", feature = "cjk"))]
    {
        let out = std::env::var("OUT_DIR").expect("OUT_DIR must be set by cargo");
        let out = std::path::Path::new(&out);
        // Known-answer check of the hasher itself ("abc", FIPS 180-4 B.1).
        assert_eq!(
            heavy::sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        #[cfg(feature = "color-emoji")]
        if heavy::provide(&heavy::COLOR_EMOJI, out) {
            println!("cargo:rustc-cfg=uzor_fonts_color_emoji");
        }
        #[cfg(feature = "cjk")]
        if heavy::provide(&heavy::CJK_SC, out) {
            println!("cargo:rustc-cfg=uzor_fonts_cjk");
        }
    }
}
