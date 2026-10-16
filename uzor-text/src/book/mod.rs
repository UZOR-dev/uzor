//! Book-layout primitives (print typography for illustrated books).
//!
//! Plain functions and plain state, no app loop:
//! - [`manuscript`]: immutable manuscript text with stable paragraph ids;
//!   soft line breaks live in a separate layout layer.
//! - [`exclusion`] + [`wrap`]: text wrapped around image rectangles with
//!   per-side gaps; text returns to full width below an image.
//! - [`prep`]: non-breaking ranges and dash binding (a dash never starts a line).
//! - [`fit`]: shrink-to-fit with a hard minimum font size floor.
//! - [`coverage`]: missing-glyph report with text positions (no silent substitution).
//!
//! Every function is a pure function of its inputs, so editor, render and
//! PDF paths that call the same function get the same lines.

pub mod coverage;
pub mod exclusion;
pub mod fit;
pub mod manuscript;
pub mod prep;
pub mod wrap;

pub use coverage::{missing_glyphs, missing_glyphs_in_runs, CoverageError, MissingGlyph};
pub use exclusion::{available_span, Exclusion, Gaps, WrapMode};
pub use fit::{fit_with_font_floor, FitResult};
pub use manuscript::{LayoutLayer, Manuscript, ParagraphId, TextRange};
pub use prep::{bind_dashes, bind_non_breaking};
pub use wrap::{layout_around, WrapLine, WrappedParagraph};
