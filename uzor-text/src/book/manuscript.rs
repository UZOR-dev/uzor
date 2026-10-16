//! Immutable manuscript with stable paragraph ids, plus a separate layout
//! layer (soft line breaks) that never edits the manuscript text.

use std::collections::{BTreeMap, BTreeSet};

/// Stable paragraph id. Never reused, never renumbered by edits elsewhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParagraphId(pub u64);

/// A byte range inside one paragraph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextRange {
    pub paragraph: ParagraphId,
    pub start: usize,
    pub end: usize,
}

/// Manuscript text. Values are immutable: every edit returns a new
/// manuscript with `revision + 1`; existing ids keep pointing at the same
/// paragraphs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Manuscript {
    paragraphs: Vec<(ParagraphId, String)>,
    next_id: u64,
    revision: u64,
}

impl Manuscript {
    /// Ids are assigned 1, 2, 3, … in order.
    pub fn from_paragraphs<I, S>(paragraphs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut m = Self { next_id: 1, ..Self::default() };
        for p in paragraphs {
            m.paragraphs.push((ParagraphId(m.next_id), p.into()));
            m.next_id += 1;
        }
        m
    }

    /// Split plain text into paragraphs at blank lines.
    pub fn from_text(text: &str) -> Self {
        Self::from_paragraphs(text.split("\n\n").map(str::trim).filter(|p| !p.is_empty()))
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn ids(&self) -> impl Iterator<Item = ParagraphId> + '_ {
        self.paragraphs.iter().map(|(id, _)| *id)
    }
    pub fn len(&self) -> usize {
        self.paragraphs.len()
    }
    pub fn is_empty(&self) -> bool {
        self.paragraphs.is_empty()
    }
    pub fn text(&self, id: ParagraphId) -> Option<&str> {
        self.paragraphs.iter().find(|(i, _)| *i == id).map(|(_, t)| t.as_str())
    }
    /// The text of a range, `None` when the paragraph is unknown or the
    /// range is out of bounds / not on char boundaries.
    pub fn range_text(&self, range: TextRange) -> Option<&str> {
        let t = self.text(range.paragraph)?;
        if range.start > range.end {
            return None;
        }
        t.get(range.start..range.end)
    }

    /// New manuscript with `text` inserted after `after` (or at the start for
    /// `None`). Returns the new paragraph's id. `None` if `after` is unknown.
    pub fn with_inserted_after(&self, after: Option<ParagraphId>, text: impl Into<String>) -> Option<(Self, ParagraphId)> {
        let pos = match after {
            None => 0,
            Some(a) => self.paragraphs.iter().position(|(i, _)| *i == a)? + 1,
        };
        let mut m = self.clone();
        let id = ParagraphId(m.next_id.max(1));
        m.next_id = id.0 + 1;
        m.paragraphs.insert(pos, (id, text.into()));
        m.revision += 1;
        Some((m, id))
    }

    /// New manuscript with one paragraph's text replaced (id unchanged).
    pub fn with_replaced(&self, id: ParagraphId, text: impl Into<String>) -> Option<Self> {
        let mut m = self.clone();
        let slot = m.paragraphs.iter_mut().find(|(i, _)| *i == id)?;
        slot.1 = text.into();
        m.revision += 1;
        Some(m)
    }
}

/// Layout-only attributes keyed by stable ids. Soft line breaks are byte
/// offsets into the paragraph text; they are applied at layout time.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LayoutLayer {
    soft_breaks: BTreeMap<ParagraphId, BTreeSet<usize>>,
}

impl LayoutLayer {
    /// Add a soft break before byte `offset` of paragraph `id`. Returns
    /// `false` (and changes nothing) when the id is unknown or the offset is
    /// not an inner char boundary.
    pub fn add_soft_break(&mut self, manuscript: &Manuscript, id: ParagraphId, offset: usize) -> bool {
        let Some(t) = manuscript.text(id) else { return false };
        if offset == 0 || offset >= t.len() || !t.is_char_boundary(offset) {
            return false;
        }
        self.soft_breaks.entry(id).or_default().insert(offset)
    }

    pub fn remove_soft_break(&mut self, id: ParagraphId, offset: usize) -> bool {
        self.soft_breaks.get_mut(&id).is_some_and(|s| s.remove(&offset))
    }

    pub fn soft_breaks(&self, id: ParagraphId) -> Vec<usize> {
        self.soft_breaks.get(&id).map(|s| s.iter().copied().collect()).unwrap_or_default()
    }

    /// Paragraph text with soft breaks rendered as forced line breaks
    /// (`\n`, a new line in the same paragraph). A space right before a
    /// break is replaced by the break. Offsets no longer valid for the
    /// current text are skipped.
    pub fn layout_text(&self, manuscript: &Manuscript, id: ParagraphId) -> Option<String> {
        let t = manuscript.text(id)?;
        let mut out = String::with_capacity(t.len() + 4);
        let mut cursor = 0;
        for &off in self.soft_breaks.get(&id).into_iter().flatten() {
            if off <= cursor || off >= t.len() || !t.is_char_boundary(off) {
                continue;
            }
            let mut seg = &t[cursor..off];
            seg = seg.strip_suffix(' ').unwrap_or(seg);
            out.push_str(seg);
            out.push('\n');
            cursor = off;
            if t[cursor..].starts_with(' ') {
                cursor += 1;
            }
        }
        out.push_str(&t[cursor..]);
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_stable_across_inserts() {
        let m = Manuscript::from_text("Первый.\n\nВторой.\n\nТретий.");
        let ids: Vec<_> = m.ids().collect();
        assert_eq!(ids, vec![ParagraphId(1), ParagraphId(2), ParagraphId(3)]);
        let (m2, new) = m.with_inserted_after(Some(ParagraphId(1)), "Новый.").unwrap();
        assert_eq!(new, ParagraphId(4));
        assert_eq!(m2.text(ParagraphId(2)), Some("Второй."));
        assert_eq!(m2.ids().collect::<Vec<_>>(), vec![ParagraphId(1), ParagraphId(4), ParagraphId(2), ParagraphId(3)]);
        assert_eq!(m2.revision(), 1);
        assert_eq!(m.revision(), 0, "original unchanged");
    }

    #[test]
    fn soft_break_is_layout_only() {
        let m = Manuscript::from_paragraphs(["Жили-были дед да баба"]);
        let id = ParagraphId(1);
        let mut l = LayoutLayer::default();
        let off = "Жили-были дед".len() + 1;
        assert!(l.add_soft_break(&m, id, off));
        assert!(!l.add_soft_break(&m, id, 1), "not a char boundary");
        assert_eq!(l.layout_text(&m, id).unwrap(), "Жили-были дед\nда баба");
        assert_eq!(m.text(id), Some("Жили-были дед да баба"));
        assert!(l.remove_soft_break(id, off));
        assert_eq!(l.layout_text(&m, id).unwrap(), "Жили-были дед да баба");
    }

    #[test]
    fn range_text_checks_bounds() {
        let m = Manuscript::from_paragraphs(["абв"]);
        assert_eq!(m.range_text(TextRange { paragraph: ParagraphId(1), start: 2, end: 4 }), Some("б"));
        assert_eq!(m.range_text(TextRange { paragraph: ParagraphId(1), start: 1, end: 4 }), None);
        assert_eq!(m.range_text(TextRange { paragraph: ParagraphId(9), start: 0, end: 0 }), None);
    }
}
