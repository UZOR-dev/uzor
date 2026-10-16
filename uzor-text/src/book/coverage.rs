//! Missing-glyph report: which characters a font cannot draw, and where
//! they are in the text. Callers decide; nothing is substituted silently.

use crate::model::StyledRun;

/// One character the font has no glyph for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissingGlyph {
    /// Index into the runs slice (0 for [`missing_glyphs`]).
    pub run_index: usize,
    /// Byte offset of the character in its run's text.
    pub byte_offset: usize,
    /// Character index (in chars) in its run's text.
    pub char_index: usize,
    pub ch: char,
}

/// The font bytes could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageError(pub String);

impl std::fmt::Display for CoverageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "font parse error: {}", self.0)
    }
}
impl std::error::Error for CoverageError {}

/// Every character of `text` (except whitespace and controls) that the
/// TTF/OTF in `font_data` (face `face_index`) has no `cmap` entry for.
pub fn missing_glyphs(font_data: &[u8], face_index: u32, text: &str) -> Result<Vec<MissingGlyph>, CoverageError> {
    let face = ttf_parser::Face::parse(font_data, face_index).map_err(|e| CoverageError(e.to_string()))?;
    Ok(scan(&face, 0, text))
}

/// [`missing_glyphs`] for every run, against the bundled font each run's
/// [`crate::FontSpec`] resolves to (the same bytes the shaper loads).
pub fn missing_glyphs_in_runs(runs: &[StyledRun<'_>]) -> Result<Vec<MissingGlyph>, CoverageError> {
    let mut out = Vec::new();
    for (i, run) in runs.iter().enumerate() {
        let bytes = uzor::fonts::font_bytes(run.font.family, run.font.bold, run.font.italic);
        let face = ttf_parser::Face::parse(bytes, 0).map_err(|e| CoverageError(e.to_string()))?;
        out.extend(scan(&face, i, run.text));
    }
    Ok(out)
}

fn scan(face: &ttf_parser::Face<'_>, run_index: usize, text: &str) -> Vec<MissingGlyph> {
    text.char_indices()
        .enumerate()
        .filter(|(_, (_, ch))| !ch.is_whitespace() && !ch.is_control() && !is_format(*ch))
        .filter(|(_, (_, ch))| face.glyph_index(*ch).is_none())
        .map(|(char_index, (byte_offset, ch))| MissingGlyph { run_index, byte_offset, char_index, ch })
        .collect()
}

fn is_format(ch: char) -> bool {
    matches!(ch, '\u{00AD}' | '\u{200B}'..='\u{200F}' | '\u{2060}' | '\u{FEFF}')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FontSpec;
    use uzor::fonts::FontFamily;

    #[test]
    fn cyrillic_is_covered_by_bundled_roboto() {
        let runs = [StyledRun::new("Съешь же ещё этих мягких французских булок, ё Ё", FontSpec::new(FontFamily::Roboto, 12.0))];
        assert_eq!(missing_glyphs_in_runs(&runs).unwrap(), vec![]);
    }

    #[test]
    fn reports_position_of_missing_character() {
        let bytes = uzor::fonts::font_bytes(FontFamily::Roboto, false, false);
        // U+4E2D (CJK) is not in Roboto.
        let text = "аб\u{4E2D}в";
        let m = missing_glyphs(bytes, 0, text).unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].ch, '\u{4E2D}');
        assert_eq!(m[0].char_index, 2);
        assert_eq!(m[0].byte_offset, 4);
    }

    #[test]
    fn bad_font_bytes_are_an_error() {
        assert!(missing_glyphs(b"not a font", 0, "a").is_err());
    }
}
