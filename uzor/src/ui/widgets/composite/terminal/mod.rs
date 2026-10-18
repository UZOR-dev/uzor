//! Terminal view: paints a cell grid produced by a terminal emulator (the
//! app feeds the PTY byte stream into its emulator and hands the cells here),
//! and encodes keys into the bytes a PTY expects.
//!
//! No emulation lives here: [`TermView`] is plain data. [`draw_terminal`]
//! places every glyph on its cell (column * cell width), so box drawing lines
//! up regardless of the font's advance; [`encode_key`] is the xterm keyboard
//! encoding; [`register_terminal`] declares the focusable hit rect.

use crate::input::core::coordinator::LayerId;
use crate::input::keyboard::KeyCode;
use crate::input::{InputCoordinator, ModifierKeys, Sense, WidgetKind};
use crate::render::{RenderContext, TextAlign, TextBaseline};
use crate::types::{Rect, WidgetId};

/// One screen cell.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TermCell {
    /// Grapheme in the cell (empty = blank or wide-char continuation).
    pub text: String,
    /// Foreground, `None` = terminal default.
    pub fg: Option<[u8; 3]>,
    /// Background, `None` = terminal default (transparent).
    pub bg: Option<[u8; 3]>,
    /// Bold.
    pub bold: bool,
    /// Italic.
    pub italic: bool,
    /// Underline.
    pub underline: bool,
    /// Occupies two columns.
    pub wide: bool,
}

/// A screen: `rows * cols` cells, row-major.
#[derive(Clone, Debug, Default)]
pub struct TermView<'a> {
    /// Columns.
    pub cols: u16,
    /// Rows.
    pub rows: u16,
    /// Cells, row-major, `rows * cols` long (shorter is painted as blank).
    pub cells: &'a [TermCell],
    /// Cursor (row, col) when visible.
    pub cursor: Option<(u16, u16)>,
    /// Has keyboard focus (cursor drawn solid, else outlined).
    pub focused: bool,
}

/// Colours, font and cell metrics.
#[derive(Clone, Debug)]
pub struct TermStyle {
    /// Background.
    pub bg: &'static str,
    /// Default foreground.
    pub fg: &'static str,
    /// Cursor.
    pub cursor: &'static str,
    /// Font size, px.
    pub size: f64,
    /// Font family (CSS); `monospace` / JetBrains Mono resolve to the bundled mono face.
    pub family: &'static str,
    /// Cell width, px.
    pub cell_w: f64,
    /// Cell height, px.
    pub cell_h: f64,
}

impl Default for TermStyle {
    fn default() -> Self {
        Self {
            bg: "#ffffff",
            fg: "#18181b",
            cursor: "#1d77f2",
            size: 13.0,
            family: "'JetBrains Mono', monospace",
            cell_w: 7.8,
            cell_h: 18.0,
        }
    }
}

/// How many cells fit in `rect`.
pub fn terminal_grid_size(rect: Rect, style: &TermStyle) -> (u16, u16) {
    let cols = (rect.width / style.cell_w).floor().clamp(2.0, 1000.0) as u16;
    let rows = (rect.height / style.cell_h).floor().clamp(1.0, 1000.0) as u16;
    (cols, rows)
}

fn hex(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

/// Paint the grid into `rect` (top-left aligned, clipped).
pub fn draw_terminal(ctx: &mut dyn RenderContext, rect: Rect, view: &TermView<'_>, style: &TermStyle) {
    ctx.set_fill_color(style.bg);
    ctx.fill_rect(rect.x, rect.y, rect.width, rect.height);
    ctx.save();
    ctx.begin_path();
    ctx.rect(rect.x, rect.y, rect.width, rect.height);
    ctx.clip();
    let cols = view.cols as usize;
    let font = format!("{}px {}", style.size, style.family);
    let bold = format!("bold {}px {}", style.size, style.family);
    let italic = format!("italic {}px {}", style.size, style.family);
    ctx.set_text_align(TextAlign::Left);
    ctx.set_text_baseline(TextBaseline::Middle);
    for row in 0..view.rows as usize {
        let y = rect.y + row as f64 * style.cell_h;
        if y >= rect.y + rect.height {
            break;
        }
        for col in 0..cols {
            let Some(cell) = view.cells.get(row * cols + col) else { break };
            let x = rect.x + col as f64 * style.cell_w;
            let w = if cell.wide { 2.0 } else { 1.0 } * style.cell_w;
            if let Some(bg) = cell.bg {
                ctx.set_fill_color(&hex(bg));
                ctx.fill_rect(x, y, w, style.cell_h);
            }
            if cell.text.is_empty() || cell.text == " " {
                continue;
            }
            let fg = cell.fg.map(hex);
            ctx.set_fill_color(fg.as_deref().unwrap_or(style.fg));
            ctx.set_font(if cell.bold { &bold } else if cell.italic { &italic } else { &font });
            ctx.fill_text(&cell.text, x, y + style.cell_h / 2.0);
            if cell.underline {
                ctx.fill_rect(x.round(), (y + style.cell_h - 2.0).round(), w.round().max(1.0), 1.0);
            }
        }
    }
    if let Some((r, c)) = view.cursor {
        if r < view.rows && c < view.cols {
            // Whole-pixel cursor box: thin fills at sub-pixel x trip the
            // tiny-skia 0.11 hairline assert (hairline_aa.rs:124).
            let x = (rect.x + c as f64 * style.cell_w).round();
            let y = (rect.y + r as f64 * style.cell_h).round();
            let (cw, chh) = (style.cell_w.round(), style.cell_h.round());
            if view.focused {
                ctx.set_global_alpha(0.35);
                ctx.set_fill_color(style.cursor);
                ctx.fill_rect(x, y, style.cell_w, style.cell_h);
                ctx.set_global_alpha(1.0);
            } else {
                // Outline from fills (no hairline strokes: CPU rasterisers
                // assert on some sub-pixel hairlines).
                ctx.set_fill_color(style.cursor);
                ctx.fill_rect(x, y, cw, 1.0);
                ctx.fill_rect(x, y + chh - 1.0, cw, 1.0);
                ctx.fill_rect(x, y, 1.0, chh);
                ctx.fill_rect(x + cw - 1.0, y, 1.0, chh);
            }
        }
    }
    ctx.restore();
}

/// Declare the terminal's hit rect (click focuses it).
pub fn register_terminal(coord: &mut InputCoordinator, id: impl Into<WidgetId>, rect: Rect, layer: &LayerId) {
    coord.register_atomic(id, WidgetKind::Button, rect, Sense::CLICK, layer);
}

fn letter(code: KeyCode) -> Option<u8> {
    use KeyCode::*;
    let l = [A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z];
    l.iter().position(|k| *k == code).map(|i| b'a' + i as u8)
}

/// xterm encoding of one key press. `text` is the logical text the host
/// reported for the press (used for printable keys).
pub fn encode_key(code: KeyCode, mods: ModifierKeys, text: Option<&str>) -> Option<Vec<u8>> {
    use KeyCode::*;
    if mods.ctrl && !mods.alt {
        if let Some(c) = letter(code) {
            return Some(vec![c - b'a' + 1]);
        }
    }
    let m = 1 + mods.shift as u8 + 2 * mods.alt as u8 + 4 * mods.ctrl as u8;
    let csi = |fin: char| -> Vec<u8> {
        if m == 1 { format!("\x1b[{fin}").into_bytes() } else { format!("\x1b[1;{m}{fin}").into_bytes() }
    };
    let tilde = |n: u8| -> Vec<u8> {
        if m == 1 { format!("\x1b[{n}~").into_bytes() } else { format!("\x1b[{n};{m}~").into_bytes() }
    };
    let out = match code {
        Enter => b"\r".to_vec(),
        Backspace => vec![0x7f],
        Tab => if mods.shift { b"\x1b[Z".to_vec() } else { b"\t".to_vec() },
        Escape => vec![0x1b],
        ArrowUp => csi('A'),
        ArrowDown => csi('B'),
        ArrowRight => csi('C'),
        ArrowLeft => csi('D'),
        Home => csi('H'),
        End => csi('F'),
        Insert => tilde(2),
        Delete => tilde(3),
        PageUp => tilde(5),
        PageDown => tilde(6),
        _ => {
            let t = text?;
            if t.is_empty() || t.chars().any(|c| c.is_control()) {
                return None;
            }
            let mut v = Vec::new();
            if mods.alt {
                v.push(0x1b);
            }
            v.extend_from_slice(t.as_bytes());
            v
        }
    };
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keys() {
        let n = ModifierKeys::default();
        let ctrl = ModifierKeys { ctrl: true, ..n };
        assert_eq!(encode_key(KeyCode::C, ctrl, None), Some(vec![3]));
        assert_eq!(encode_key(KeyCode::ArrowUp, n, None), Some(b"\x1b[A".to_vec()));
        assert_eq!(encode_key(KeyCode::ArrowUp, ctrl, None), Some(b"\x1b[1;5A".to_vec()));
        assert_eq!(encode_key(KeyCode::A, n, Some("я")), Some("я".as_bytes().to_vec()));
        assert_eq!(encode_key(KeyCode::Enter, n, Some("\r")), Some(b"\r".to_vec()));
        assert_eq!(encode_key(KeyCode::F1, n, None), None);
    }
    #[test]
    fn grid_size() {
        let s = TermStyle::default();
        assert_eq!(terminal_grid_size(Rect::new(0.0, 0.0, 780.0, 180.0), &s), (100, 10));
    }
}
