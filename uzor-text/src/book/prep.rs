//! Text preparation that changes only break opportunities, never glyphs
//! the reader sees: ordinary spaces become no-break spaces (U+00A0), which
//! every packer in this crate already treats as non-breaking.

use std::ops::Range;

const NBSP: char = '\u{00A0}';

/// Replace every breakable space inside each byte range with U+00A0, so the
/// range is never split across lines (names, "5 кг", "т. е.").
/// Ranges are clamped to char boundaries; out-of-range parts are ignored.
pub fn bind_non_breaking(text: &str, ranges: &[Range<usize>]) -> String {
    let mut out = String::with_capacity(text.len() + ranges.len() * 2);
    for (i, ch) in text.char_indices() {
        let inside = ranges.iter().any(|r| i >= r.start && i < r.end);
        if inside && is_breakable_space(ch) {
            out.push(NBSP);
        } else {
            out.push(ch);
        }
    }
    out
}

/// Glue an em dash (—) or en dash (–) to the word before it: the space
/// before such a dash becomes U+00A0, so a dash never starts a line.
/// A dash that opens a paragraph or follows a forced break (dialogue) is
/// left alone, as is the space after it.
pub fn bind_dashes(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    for (i, &ch) in chars.iter().enumerate() {
        let next_is_dash = matches!(chars.get(i + 1), Some('\u{2014}' | '\u{2013}'));
        let prev_is_text = i > 0 && !chars[i - 1].is_whitespace();
        if ch == ' ' && next_is_dash && prev_is_text {
            out.push(NBSP);
        } else {
            out.push(ch);
        }
    }
    out
}

fn is_breakable_space(ch: char) -> bool {
    ch.is_whitespace() && ch != '\n' && ch != '\u{00A0}' && ch != '\u{202F}'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_breaking_range_replaces_only_inside() {
        let s = "a b c d";
        assert_eq!(bind_non_breaking(s, &[2..5]), "a b\u{a0}c d");
    }

    #[test]
    fn non_breaking_keeps_forced_breaks() {
        assert_eq!(bind_non_breaking("x\ny z", &[0..5]), "x\ny\u{a0}z");
    }

    #[test]
    fn dash_bound_to_previous_word() {
        assert_eq!(bind_dashes("Мама — врач"), "Мама\u{a0}— врач");
        assert_eq!(bind_dashes("— Привет, — сказал он."), "— Привет,\u{a0}— сказал он.");
        assert_eq!(bind_dashes("строка\n— реплика"), "строка\n— реплика");
    }
}
