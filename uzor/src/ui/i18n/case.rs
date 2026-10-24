//! Table-free case mapping for search and initials: ASCII, Latin-1, Greek and
//! Cyrillic (the scripts HQ shows). `str::to_lowercase` pulls the full Unicode
//! case tables into a wasm build (about 25 KB raw); this covers matching and
//! avatars at a few hundred bytes. Other characters map to themselves.

/// Lower case of one character.
pub fn lower_char(c: char) -> char {
    let u = c as u32;
    let d = match u {
        0x41..=0x5a | 0xc0..=0xd6 | 0xd8..=0xde | 0x391..=0x3a1 | 0x3a3..=0x3ab | 0x410..=0x42f => 0x20,
        0x400..=0x40f => 0x50,
        _ => 0,
    };
    char::from_u32(u + d).unwrap_or(c)
}

/// Upper case of one character (inverse of [`lower_char`]; `ß`, `ÿ` stay).
pub fn upper_char(c: char) -> char {
    let u = c as u32;
    let d = match u {
        0x61..=0x7a | 0xe0..=0xf6 | 0xf8..=0xfe | 0x3b1..=0x3c1 | 0x3c3..=0x3cb | 0x430..=0x44f => 0x20,
        0x450..=0x45f => 0x50,
        _ => 0,
    };
    char::from_u32(u - d).unwrap_or(c)
}

/// [`lower_char`] over a string (case-insensitive search).
pub fn lower(s: &str) -> String {
    s.chars().map(lower_char).collect()
}

/// `s` with its first character upper-cased (initials, labels).
pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| upper_char(f).to_string() + c.as_str()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matches_std_on_covered_scripts() {
        for s in ["Hello WORLD", "ÀÉÎÕÜ Ø", "ΑΒΓ ΣΩ", "Привет ЁЖИК Ђ", "x-1_Y"] {
            assert_eq!(lower(s), s.to_lowercase(), "{s}");
            assert_eq!(s.to_lowercase().chars().map(upper_char).collect::<String>(), s.to_lowercase().to_uppercase(), "{s}");
        }
        assert_eq!(capitalize("агент"), "Агент");
        assert_eq!(lower_char('×'), '×');
        assert_eq!(upper_char('÷'), '÷');
    }
}
