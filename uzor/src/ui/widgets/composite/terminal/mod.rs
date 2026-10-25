//! Terminal view: paints a cell grid produced by a terminal emulator (the
//! app feeds the PTY byte stream into its emulator and hands the cells here),
//! and encodes keys into the bytes a PTY expects.
//!
//! No emulation lives here: [`TermView`] is plain data. [`draw_terminal`]
//! places every glyph on its cell (column * cell width), so box drawing lines
//! up regardless of the font's advance; [`encode_key`] is the xterm keyboard
//! encoding; [`register_terminal`] declares the focusable hit rect.

mod tokens;

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
            bg: tokens::BG,
            fg: tokens::FG,
            cursor: tokens::CURSOR,
            size: 13.0,
            family: "'JetBrains Mono', monospace",
            cell_w: 7.8,
            cell_h: 18.0,
        }
    }
}

impl TermStyle {
    /// This style with the cell size taken from the font the renderer actually
    /// resolves (`measure_text`), snapped to whole device pixels.
    ///
    /// A fixed `cell_w` only fits one font: when the browser falls back to
    /// another monospace face (no JetBrains Mono installed) its advance is
    /// narrower than the cell, and every glyph leaves a gap: block art
    /// (`█▛▜`) comes out in vertical stripes and `─` rules dashed. A cell that
    /// is a whole number of device pixels also puts every glyph on the pixel
    /// grid, so no column is blurred by a fractional origin.
    pub fn measured(mut self, ctx: &mut dyn RenderContext) -> Self {
        let (w, h) = terminal_cell_size(ctx, self.size, self.family);
        self.cell_w = w;
        self.cell_h = h;
        self
    }
}

/// Cell size for `size`px `family` on `ctx`: the font's advance (measured
/// over ten `M`s) and 1.17 em of line height, each a whole number of device
/// pixels (at least one). Falls back to 0.6 em wide when the backend
/// measures nothing.
pub fn terminal_cell_size(ctx: &mut dyn RenderContext, size: f64, family: &str) -> (f64, f64) {
    let dpr = sane_dpr(ctx.dpr());
    ctx.set_font(&format!("{size}px {family}"));
    let ten = ctx.measure_text("MMMMMMMMMM");
    let adv = if ten.is_finite() && ten > 0.0 { ten / 10.0 } else { size * 0.6 };
    let w = (adv * dpr).round().max(1.0) / dpr;
    let h = (size * 1.17 * dpr).round().max(1.0) / dpr;
    (w, h)
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

fn sane_dpr(d: f64) -> f64 {
    if d.is_finite() && d > 0.0 { d } else { 1.0 }
}

/// Rounds a CSS coordinate to the nearest device pixel.
#[derive(Clone, Copy)]
struct Snap(f64);

impl Snap {
    fn at(self, v: f64) -> f64 {
        (v * self.0).round() / self.0
    }
    /// `n` device pixels in CSS pixels.
    fn px(self, n: f64) -> f64 {
        n / self.0
    }
}

/// Cell edges along one axis: `edges[i]` is the device-pixel-snapped start
/// of cell `i` (and the end of cell `i - 1`), so neighbours share an edge
/// exactly: no seam and no overlap between cells, at any device pixel ratio.
fn cell_edges(origin: f64, cell: f64, n: usize, snap: Snap) -> Vec<f64> {
    (0..=n).map(|i| snap.at(origin + i as f64 * cell)).collect()
}

/// Fill colour set on the context, so a run of cells in one colour sets it once.
struct Pen {
    fill: String,
    font: String,
}

impl Pen {
    fn fill(&mut self, ctx: &mut dyn RenderContext, c: &str) {
        if self.fill != c {
            ctx.set_fill_color(c);
            self.fill.clear();
            self.fill.push_str(c);
        }
    }
    fn font(&mut self, ctx: &mut dyn RenderContext, f: &str) {
        if self.font != f {
            ctx.set_font(f);
            self.font.clear();
            self.font.push_str(f);
        }
    }
}

fn fill_box(ctx: &mut dyn RenderContext, x0: f64, y0: f64, x1: f64, y1: f64) {
    if x1 > x0 && y1 > y0 {
        ctx.fill_rect(x0, y0, x1 - x0, y1 - y0);
    }
}

/// The four arms of a box-drawing character: 0 none, 1 light, 2 heavy, 3 double.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Arms {
    l: u8,
    u: u8,
    r: u8,
    d: u8,
}

const fn a(l: u8, u: u8, r: u8, d: u8) -> Arms {
    Arms { l, u, r, d }
}

/// Arms of each box-drawing character U+2500..U+257F. Dashed lines, rounded
/// corners and diagonals are not in it (no arms).
const BOX_ARMS: [Arms; 128] = {
    const N: Arms = a(0, 0, 0, 0);
    [
        // 2500 ─ ━ │ ┃ ┄ ┅ ┆ ┇ ┈ ┉ ┊ ┋ ┌ ┍ ┎ ┏
        a(1, 0, 1, 0), a(2, 0, 2, 0), a(0, 1, 0, 1), a(0, 2, 0, 2), N, N, N, N, N, N, N, N,
        a(0, 0, 1, 1), a(0, 0, 2, 1), a(0, 0, 1, 2), a(0, 0, 2, 2),
        // 2510 ┐ ┑ ┒ ┓ └ ┕ ┖ ┗ ┘ ┙ ┚ ┛ ├ ┝ ┞ ┟
        a(1, 0, 0, 1), a(2, 0, 0, 1), a(1, 0, 0, 2), a(2, 0, 0, 2),
        a(0, 1, 1, 0), a(0, 1, 2, 0), a(0, 2, 1, 0), a(0, 2, 2, 0),
        a(1, 1, 0, 0), a(2, 1, 0, 0), a(1, 2, 0, 0), a(2, 2, 0, 0),
        a(0, 1, 1, 1), a(0, 1, 2, 1), a(0, 2, 1, 1), a(0, 1, 1, 2),
        // 2520 ┠ ┡ ┢ ┣ ┤ ┥ ┦ ┧ ┨ ┩ ┪ ┫ ┬ ┭ ┮ ┯
        a(0, 2, 1, 2), a(0, 2, 2, 1), a(0, 1, 2, 2), a(0, 2, 2, 2),
        a(1, 1, 0, 1), a(2, 1, 0, 1), a(1, 2, 0, 1), a(1, 1, 0, 2),
        a(1, 2, 0, 2), a(2, 2, 0, 1), a(2, 1, 0, 2), a(2, 2, 0, 2),
        a(1, 0, 1, 1), a(2, 0, 1, 1), a(1, 0, 2, 1), a(2, 0, 2, 1),
        // 2530 ┰ ┱ ┲ ┳ ┴ ┵ ┶ ┷ ┸ ┹ ┺ ┻ ┼ ┽ ┾ ┿
        a(1, 0, 1, 2), a(2, 0, 1, 2), a(1, 0, 2, 2), a(2, 0, 2, 2),
        a(1, 1, 1, 0), a(2, 1, 1, 0), a(1, 1, 2, 0), a(2, 1, 2, 0),
        a(1, 2, 1, 0), a(2, 2, 1, 0), a(1, 2, 2, 0), a(2, 2, 2, 0),
        a(1, 1, 1, 1), a(2, 1, 1, 1), a(1, 1, 2, 1), a(2, 1, 2, 1),
        // 2540 ╀ ╁ ╂ ╃ ╄ ╅ ╆ ╇ ╈ ╉ ╊ ╋ ╌ ╍ ╎ ╏
        a(1, 2, 1, 1), a(1, 1, 1, 2), a(1, 2, 1, 2), a(2, 2, 1, 1),
        a(1, 2, 2, 1), a(2, 1, 1, 2), a(1, 1, 2, 2), a(2, 2, 2, 1),
        a(2, 1, 2, 2), a(2, 2, 1, 2), a(1, 2, 2, 2), a(2, 2, 2, 2),
        N, N, N, N,
        // 2550 ═ ║ ╒ ╓ ╔ ╕ ╖ ╗ ╘ ╙ ╚ ╛ ╜ ╝ ╞ ╟
        a(3, 0, 3, 0), a(0, 3, 0, 3), a(0, 0, 3, 1), a(0, 0, 1, 3),
        a(0, 0, 3, 3), a(3, 0, 0, 1), a(1, 0, 0, 3), a(3, 0, 0, 3),
        a(0, 1, 3, 0), a(0, 3, 1, 0), a(0, 3, 3, 0), a(3, 1, 0, 0),
        a(1, 3, 0, 0), a(3, 3, 0, 0), a(0, 1, 3, 1), a(0, 3, 1, 3),
        // 2560 ╠ ╡ ╢ ╣ ╤ ╥ ╦ ╧ ╨ ╩ ╪ ╫ ╬ ╭ ╮ ╯
        a(0, 3, 3, 3), a(3, 1, 0, 1), a(1, 3, 0, 3), a(3, 3, 0, 3),
        a(3, 0, 3, 1), a(1, 0, 1, 3), a(3, 0, 3, 3), a(3, 1, 3, 0),
        a(1, 3, 1, 0), a(3, 3, 3, 0), a(3, 1, 3, 1), a(1, 3, 1, 3),
        a(3, 3, 3, 3), N, N, N,
        // 2570 ╰ ╱ ╲ ╳ ╴ ╵ ╶ ╷ ╸ ╹ ╺ ╻ ╼ ╽ ╾ ╿
        N, N, N, N,
        a(1, 0, 0, 0), a(0, 1, 0, 0), a(0, 0, 1, 0), a(0, 0, 0, 1),
        a(2, 0, 0, 0), a(0, 2, 0, 0), a(0, 0, 2, 0), a(0, 0, 0, 2),
        a(1, 0, 2, 0), a(0, 1, 0, 2), a(2, 0, 1, 0), a(0, 2, 0, 1),
    ]
};

/// Cell geometry for one special glyph.
struct CellBox {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    snap: Snap,
    /// Light line width, CSS px (a whole number of device pixels).
    thin: f64,
}

impl CellBox {
    /// `[a, a + t)` centred in `[lo, hi)`, on the device pixel grid.
    fn band(&self, lo: f64, hi: f64, t: f64) -> (f64, f64) {
        let a = self.snap.at(lo + (hi - lo - t) / 2.0);
        (a, a + t)
    }
    fn weight(&self, w: u8) -> f64 {
        match w {
            0 => 0.0,
            1 => self.thin,
            2 => 2.0 * self.thin,
            _ => 3.0 * self.thin,
        }
    }
}

/// Paint a block element (U+2580..U+259F) or box-drawing character
/// (U+2500..U+257F) as rectangles filling its cell edge to edge, the way
/// xterm.js and every GPU terminal do, instead of through the font: a font
/// glyph only covers the font's own advance and line box, so on any cell
/// that is wider or taller than those the art falls apart. Returns `false`
/// for anything else (the caller draws it as text).
fn draw_special(ctx: &mut dyn RenderContext, pen: &mut Pen, ch: char, b: &CellBox, fg: &str) -> bool {
    let c = ch as u32;
    let (x0, y0, x1, y1) = (b.x0, b.y0, b.x1, b.y1);
    let (w, h) = (x1 - x0, y1 - y0);
    let hx = |f: f64| b.snap.at(x0 + w * f);
    let vy = |f: f64| b.snap.at(y0 + h * f);
    match c {
        0x2580..=0x259F => {
            pen.fill(ctx, fg);
            match c {
                0x2580 => fill_box(ctx, x0, y0, x1, vy(0.5)),
                0x2581..=0x2587 => fill_box(ctx, x0, vy(1.0 - (c - 0x2580) as f64 / 8.0), x1, y1),
                0x2588 => fill_box(ctx, x0, y0, x1, y1),
                0x2589..=0x258F => fill_box(ctx, x0, y0, hx((0x2590 - c) as f64 / 8.0), y1),
                0x2590 => fill_box(ctx, hx(0.5), y0, x1, y1),
                0x2591..=0x2593 => {
                    ctx.set_global_alpha((c - 0x2590) as f64 * 0.25);
                    fill_box(ctx, x0, y0, x1, y1);
                    ctx.set_global_alpha(1.0);
                }
                0x2594 => fill_box(ctx, x0, y0, x1, vy(0.125)),
                0x2595 => fill_box(ctx, hx(0.875), y0, x1, y1),
                _ => {
                    // quadrants: bit 1 upper left, 2 upper right, 4 lower left, 8 lower right
                    const Q: [u8; 10] = [4, 8, 1, 1 | 4 | 8, 1 | 8, 1 | 2 | 4, 1 | 2 | 8, 2, 2 | 4, 2 | 4 | 8];
                    let q = Q[(c - 0x2596) as usize];
                    let (mx, my) = (hx(0.5), vy(0.5));
                    if q & 1 != 0 { fill_box(ctx, x0, y0, mx, my) }
                    if q & 2 != 0 { fill_box(ctx, mx, y0, x1, my) }
                    if q & 4 != 0 { fill_box(ctx, x0, my, mx, y1) }
                    if q & 8 != 0 { fill_box(ctx, mx, my, x1, y1) }
                }
            }
            true
        }
        // dashed: ┄┅┆┇ (3), ┈┉┊┋ (4), ╌╍╎╏ (2); even = horizontal, odd pair = heavy
        0x2504..=0x250B | 0x254C..=0x254F => {
            let (n, k) = match c {
                0x2504..=0x2507 => (3, c - 0x2504),
                0x2508..=0x250B => (4, c - 0x2508),
                _ => (2, c - 0x254C),
            };
            let t = if k % 2 == 0 { b.thin } else { 2.0 * b.thin };
            pen.fill(ctx, fg);
            for i in 0..n {
                let (f0, f1) = ((i as f64 + 0.2) / n as f64, (i as f64 + 0.8) / n as f64);
                if k < 2 {
                    let (a, z) = b.band(y0, y1, t);
                    fill_box(ctx, hx(f0), a, hx(f1), z);
                } else {
                    let (a, z) = b.band(x0, x1, t);
                    fill_box(ctx, a, vy(f0), z, vy(f1));
                }
            }
            true
        }
        // rounded corners ╭ ╮ ╯ ╰: a quarter curve between the two arms' centre lines
        0x256D..=0x2570 => {
            let t = b.thin;
            let (vl, vr) = b.band(x0, x1, t);
            let (ht, hb) = b.band(y0, y1, t);
            let (cx, cy) = ((vl + vr) / 2.0, (ht + hb) / 2.0);
            // (horizontal arm towards +x?, vertical arm towards +y?)
            let (right, down) = match c {
                0x256D => (true, true),
                0x256E => (false, true),
                0x256F => (false, false),
                _ => (true, false),
            };
            let xe = if right { x1 } else { x0 };
            let ye = if down { y1 } else { y0 };
            let r = (xe - cx).abs().min((ye - cy).abs());
            let (sx, sy) = (if right { 1.0 } else { -1.0 }, if down { 1.0 } else { -1.0 });
            ctx.set_stroke_color(fg);
            ctx.set_stroke_width(t);
            ctx.set_line_cap("butt");
            ctx.begin_path();
            ctx.move_to(cx, ye);
            ctx.line_to(cx, cy + sy * r);
            ctx.quadratic_curve_to(cx, cy, cx + sx * r, cy);
            ctx.line_to(xe, cy);
            ctx.stroke();
            true
        }
        0x2500..=0x257F => {
            let Arms { l, u, r, d } = BOX_ARMS[(c - 0x2500) as usize];
            if l | u | r | d == 0 {
                return false;
            }
            pen.fill(ctx, fg);
            // A horizontal arm runs into the vertical band (and vice versa), so
            // corners and tees close without a notch.
            let tv = b.weight(u.max(d));
            let th = b.weight(l.max(r));
            let (vl, vr) = b.band(x0, x1, tv);
            let (ht, hb) = b.band(y0, y1, th);
            let arm = |ctx: &mut dyn RenderContext, wgt: u8, horiz: bool, from: f64, to: f64| {
                if wgt == 0 {
                    return;
                }
                let lines: &[(f64, f64)] = &if wgt == 3 { [(0.0, 1.0), (2.0, 3.0)] } else { [(0.0, b.weight(wgt) / b.thin), (0.0, 0.0)] };
                let total = b.weight(wgt);
                let (a, _) = if horiz { b.band(y0, y1, total) } else { b.band(x0, x1, total) };
                for &(s, e) in lines {
                    if e <= s {
                        continue;
                    }
                    let (p, q) = (a + s * b.thin, a + e * b.thin);
                    if horiz { fill_box(ctx, from, p, to, q) } else { fill_box(ctx, p, from, q, to) }
                }
            };
            let cxm = b.snap.at(x0 + w / 2.0);
            let cym = b.snap.at(y0 + h / 2.0);
            arm(ctx, l, true, x0, if tv > 0.0 { vr } else { cxm });
            arm(ctx, r, true, if tv > 0.0 { vl } else { cxm }, x1);
            arm(ctx, u, false, y0, if th > 0.0 { hb } else { cym });
            arm(ctx, d, false, if th > 0.0 { ht } else { cym }, y1);
            true
        }
        _ => false,
    }
}

/// Paint the grid into `rect` (top-left aligned, clipped).
///
/// Every cell edge sits on a device pixel and neighbouring cells share it, so
/// backgrounds tile without seams; block elements and box drawing are filled
/// as rectangles over the whole cell ([`draw_special`]) rather than left to
/// the font; text is drawn from the snapped cell origin.
pub fn draw_terminal(ctx: &mut dyn RenderContext, rect: Rect, view: &TermView<'_>, style: &TermStyle) {
    let snap = Snap(sane_dpr(ctx.dpr()));
    let cols = view.cols as usize;
    let rows = view.rows as usize;
    let xs = cell_edges(rect.x, style.cell_w, cols, snap);
    let ys = cell_edges(rect.y, style.cell_h, rows, snap);
    let mut pen = Pen { fill: String::new(), font: String::new() };
    pen.fill(ctx, style.bg);
    fill_box(ctx, snap.at(rect.x), snap.at(rect.y), snap.at(rect.x + rect.width), snap.at(rect.y + rect.height));
    ctx.save();
    ctx.begin_path();
    ctx.rect(rect.x, rect.y, rect.width, rect.height);
    ctx.clip();
    let font = format!("{}px {}", style.size, style.family);
    let bold = format!("bold {}px {}", style.size, style.family);
    let italic = format!("italic {}px {}", style.size, style.family);
    let thin = snap.px((style.size * snap.0 / 13.0).round().max(1.0));
    ctx.set_text_align(TextAlign::Left);
    ctx.set_text_baseline(TextBaseline::Middle);
    let bottom = rect.y + rect.height;
    for row in 0..rows {
        let (y0, y1) = (ys[row], ys[row + 1]);
        if y0 >= bottom {
            break;
        }
        let line = view.cells.get(row * cols..((row + 1) * cols).min(view.cells.len())).unwrap_or(&[]);
        // Backgrounds: one fill per run of equal colour.
        let mut col = 0;
        while col < line.len() {
            let Some(bg) = line[col].bg else {
                col += 1;
                continue;
            };
            let start = col;
            while col < line.len() && line[col].bg == Some(bg) {
                col += 1;
            }
            pen.fill(ctx, &hex(bg));
            fill_box(ctx, xs[start], y0, xs[col.min(cols)], y1);
        }
        for (col, cell) in line.iter().enumerate() {
            if cell.text.is_empty() || cell.text == " " {
                continue;
            }
            let x0 = xs[col];
            let x1 = xs[(col + if cell.wide { 2 } else { 1 }).min(cols)];
            let fg = cell.fg.map(hex);
            let fg = fg.as_deref().unwrap_or(style.fg);
            let mut it = cell.text.chars();
            if let (Some(ch), None) = (it.next(), it.next()) {
                let b = CellBox { x0, y0, x1, y1, snap, thin };
                if draw_special(ctx, &mut pen, ch, &b, fg) {
                    if cell.underline {
                        pen.fill(ctx, fg);
                        fill_box(ctx, x0, y1 - snap.px(2.0 * snap.0.round().max(1.0)), x1, y1 - snap.px(snap.0.round().max(1.0)));
                    }
                    continue;
                }
            }
            pen.fill(ctx, fg);
            pen.font(ctx, if cell.bold { &bold } else if cell.italic { &italic } else { &font });
            ctx.fill_text(&cell.text, x0, snap.at((y0 + y1) / 2.0));
            if cell.underline {
                let one = snap.px(snap.0.round().max(1.0));
                fill_box(ctx, x0, y1 - 2.0 * one, x1, y1 - one);
            }
        }
    }
    if let Some((r, c)) = view.cursor {
        if r < view.rows && c < view.cols {
            let (x0, x1) = (xs[c as usize], xs[c as usize + 1]);
            let (y0, y1) = (ys[r as usize], ys[r as usize + 1]);
            pen.fill(ctx, style.cursor);
            if view.focused {
                ctx.set_global_alpha(0.35);
                fill_box(ctx, x0, y0, x1, y1);
                ctx.set_global_alpha(1.0);
            } else {
                // Outline from fills (no hairline strokes: CPU rasterisers
                // assert on some sub-pixel hairlines).
                let one = snap.px(snap.0.round().max(1.0));
                fill_box(ctx, x0, y0, x1, y0 + one);
                fill_box(ctx, x0, y1 - one, x1, y1);
                fill_box(ctx, x0, y0, x0 + one, y1);
                fill_box(ctx, x1 - one, y0, x1, y1);
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
    use crate::testing::{DrawOp, RecordingRenderContext};

    fn cell(t: &str, fg: Option<[u8; 3]>, bg: Option<[u8; 3]>) -> TermCell {
        TermCell { text: t.into(), fg, bg, ..TermCell::default() }
    }

    fn rects(ctx: &RecordingRenderContext) -> Vec<(f64, f64, f64, f64)> {
        ctx.ops.iter().filter_map(|o| match o { DrawOp::FillRect { x, y, w, h } => Some((*x, *y, *w, *h)), _ => None }).collect()
    }

    fn style(cell_w: f64) -> TermStyle {
        TermStyle { cell_w, cell_h: 15.0, size: 13.0, ..TermStyle::default() }
    }

    #[test]
    fn cell_edges_are_shared_and_on_device_pixels() {
        for dpr in [1.0, 1.25, 1.5, 2.0] {
            let e = cell_edges(260.5, 7.15, 120, Snap(dpr));
            for w in e.windows(2) {
                assert!(w[1] > w[0], "empty cell at dpr {dpr}");
                // a whole number of device pixels apart, never more than one off the exact width
                let dev = (w[1] - w[0]) * dpr;
                assert!((dev - dev.round()).abs() < 1e-9);
                assert!(((w[1] - w[0]) - 7.15).abs() <= 1.0 / dpr + 1e-9);
            }
            assert!((e[120] - (260.5 + 120.0 * 7.15)).abs() <= 0.5 / dpr + 1e-9, "no drift at dpr {dpr}");
        }
    }

    #[test]
    fn cell_size_comes_from_the_font_on_whole_pixels() {
        // a fallback mono face (Consolas-like 7.15px advance at 13px)
        let mut ctx = RecordingRenderContext::with_char_width(7.15);
        assert_eq!(terminal_cell_size(&mut ctx, 13.0, "'JetBrains Mono', monospace"), (7.0, 15.0));
        let mut ctx = RecordingRenderContext::with_char_width(7.8);
        assert_eq!(terminal_cell_size(&mut ctx, 13.0, "monospace"), (8.0, 15.0));
        // nothing measured: 0.6 em
        let mut ctx = RecordingRenderContext::new();
        assert_eq!(terminal_cell_size(&mut ctx, 13.0, "monospace").0, 8.0);
    }

    /// Claude's mascot row: ` ▐` + `▛███▜` on a black background, orange.
    #[test]
    fn block_art_fills_whole_cells_without_text() {
        let o = Some([0xd7, 0x77, 0x57]);
        let k = Some([0, 0, 0]);
        let cells: Vec<TermCell> = [(" ", None, None), ("▐", o, None), ("▛", o, k), ("█", o, k), ("█", o, k), ("█", o, k), ("▜", o, k), ("▌", o, None)]
            .iter()
            .map(|(t, f, b)| cell(t, *f, *b))
            .collect();
        let view = TermView { cols: 8, rows: 1, cells: &cells, cursor: None, focused: false };
        let mut ctx = RecordingRenderContext::new();
        let st = style(7.15);
        draw_terminal(&mut ctx, Rect::new(0.0, 0.0, 200.0, 15.0), &view, &st);
        assert!(ctx.fill_texts().is_empty(), "block elements go through the font: {:?}", ctx.fill_texts());
        let xs = cell_edges(0.0, 7.15, 8, Snap(1.0));
        let r = rects(&ctx);
        // the three full blocks are each exactly their cell, so the run has no gap
        for c in 3..6 {
            assert!(r.contains(&(xs[c], 0.0, xs[c + 1] - xs[c], 15.0)), "█ in col {c} not edge to edge: {r:?}");
        }
        // ▐ = right half; ▌ = left half; ▛ = all but the lower right quadrant
        let mid = |c: usize| (xs[c] + (xs[c + 1] - xs[c]) / 2.0).round();
        assert!(r.contains(&(mid(1), 0.0, xs[2] - mid(1), 15.0)));
        assert!(r.contains(&(xs[7], 0.0, mid(7) - xs[7], 15.0)));
        assert!(r.contains(&(xs[2], 0.0, mid(2) - xs[2], 8.0)));
        assert!(r.contains(&(mid(2), 0.0, xs[3] - mid(2), 8.0)));
        assert!(r.contains(&(xs[2], 8.0, mid(2) - xs[2], 7.0)));
        // the black background is one run over cols 2..7, on the same edges
        assert!(r.contains(&(xs[2], 0.0, xs[7] - xs[2], 15.0)));
    }

    #[test]
    fn rules_are_solid_across_cells() {
        let cells: Vec<TermCell> = (0..10).map(|_| cell("─", None, None)).collect();
        let view = TermView { cols: 10, rows: 1, cells: &cells, cursor: None, focused: false };
        for dpr_cell in [7.15, 8.05, 7.8] {
            let mut ctx = RecordingRenderContext::new();
            draw_terminal(&mut ctx, Rect::new(0.5, 0.0, 200.0, 15.0), &view, &style(dpr_cell));
            assert!(ctx.fill_texts().is_empty());
            // skip the background fill
            let r: Vec<_> = rects(&ctx).into_iter().skip(1).collect();
            let mut spans: Vec<(f64, f64)> = r.iter().map(|q| (q.0, q.0 + q.2)).collect();
            spans.sort_by(|a, b| a.partial_cmp(b).unwrap());
            for w in spans.windows(2) {
                assert!(w[1].0 <= w[0].1 + 1e-9, "gap between {:?} and {:?} (cell {dpr_cell})", w[0], w[1]);
            }
            assert!(r.iter().all(|q| q.3 == 1.0 && q.1 == r[0].1), "one 1px line at one height: {r:?}");
        }
    }

    #[test]
    fn corners_and_tees_close() {
        // ┌: the right arm starts at the vertical line's left edge, the down arm at the
        // horizontal line's top edge, so the corner pixel is covered by both.
        let cells = vec![cell("┌", None, None)];
        let view = TermView { cols: 1, rows: 1, cells: &cells, cursor: None, focused: false };
        let mut ctx = RecordingRenderContext::new();
        draw_terminal(&mut ctx, Rect::new(0.0, 0.0, 8.0, 15.0), &view, &style(8.0));
        let r: Vec<_> = rects(&ctx).into_iter().skip(1).collect();
        assert_eq!(r, vec![(4.0, 7.0, 4.0, 1.0), (4.0, 7.0, 1.0, 8.0)]);
        assert_eq!(BOX_ARMS[0x253C - 0x2500], a(1, 1, 1, 1));
        assert_eq!(BOX_ARMS[0x254B - 0x2500], a(2, 2, 2, 2));
        assert_eq!(BOX_ARMS[0x2550 - 0x2500], a(3, 0, 3, 0));
        assert_eq!(BOX_ARMS[0x2518 - 0x2500], a(1, 1, 0, 0));
        assert_eq!(BOX_ARMS[0x257F - 0x2500], a(0, 2, 0, 1));
    }

    #[test]
    fn text_sits_on_the_pixel_grid_and_rounded_corners_are_strokes() {
        let cells = vec![cell("a", None, None), cell("╭", None, None), cell("╱", None, None)];
        let view = TermView { cols: 3, rows: 1, cells: &cells, cursor: None, focused: false };
        let mut ctx = RecordingRenderContext::new();
        draw_terminal(&mut ctx, Rect::new(0.3, 0.3, 30.0, 15.0), &view, &style(7.15));
        let t = ctx.fill_texts();
        // `a` and the diagonal (not drawn by hand) are text, at whole-pixel origins
        assert_eq!(t.iter().map(|x| x.0).collect::<Vec<_>>(), vec!["a", "╱"]);
        for (_, x, y) in t {
            assert_eq!((x.fract(), y.fract()), (0.0, 0.0));
        }
        assert!(ctx.ops.iter().any(|o| matches!(o, DrawOp::Stroke)));
    }

    #[test]
    fn grid_size() {
        let s = TermStyle::default();
        assert_eq!(terminal_grid_size(Rect::new(0.0, 0.0, 780.0, 180.0), &s), (100, 10));
    }
}
