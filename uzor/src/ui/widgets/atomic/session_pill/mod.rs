//! Session pill: a centred, floating rounded label (critter + name + chevron)
//! that opens the session control panel.

use crate::input::core::coordinator::LayerId;
use crate::input::{InputCoordinator, Sense, WidgetKind};
use crate::render::{RenderContext, TextAlign};
use crate::ui::css_text::{fill_text_css, INTER};
use crate::types::{Rect, WidgetId};

use super::critter::{draw_critter_motion, CritterMotion, CritterView};

/// Plain view data.
#[derive(Clone, Debug)]
pub struct SessionPillView<'a> {
    /// Session name.
    pub label: &'a str,
    /// Avatar on the left, if any.
    pub critter: Option<CritterView>,
    /// Avatar animation frame (`None`: at rest).
    pub motion: Option<CritterMotion>,
    /// Hover tint.
    pub hovered: bool,
    /// Panel currently open.
    pub open: bool,
}

/// Colours, font and metrics. [`SessionPillStyle::default`] is the HQ web
/// `#headbtn` (hatchery-hq-web style.css): 40px high, 1px border, padding
/// `0 13px 0 7px`, 24px avatar, 10px gap, `600 15px Inter`, soft drop shadow.
#[derive(Clone, Debug)]
pub struct SessionPillStyle {
    /// Background.
    pub bg: &'static str,
    /// Hover / open background.
    pub bg_hover: &'static str,
    /// Hairline border.
    pub border: &'static str,
    /// Label colour.
    pub text: &'static str,
    /// Chevron colour.
    pub sub: &'static str,
    /// CSS font.
    pub font: &'static str,
    /// Font size in px (matches `font`).
    pub font_px: f64,
    /// Height.
    pub height: f64,
    /// Distance from the top of the area.
    pub top: f64,
    /// Left padding (inside the border).
    pub pad_l: f64,
    /// Right padding (inside the border).
    pub pad_r: f64,
    /// Avatar size.
    pub avatar: f64,
    /// Avatar → label gap.
    pub gap: f64,
    /// Drop shadow `(dy, blur, colour)`; `None` = flat.
    pub shadow: Option<(f64, f64, &'static str)>,
    /// Draw the `›` chevron while hovered / open (HQ web shows it on hover only).
    pub chevron_on_hover: bool,
}

impl Default for SessionPillStyle {
    fn default() -> Self {
        Self {
            bg: "#1f1f1f",
            bg_hover: "#262626",
            border: "#2b2b2b",
            text: "#f5f5f5",
            sub: "#acacac",
            font: "600 15px Inter, sans-serif",
            font_px: 15.0,
            height: 40.0,
            top: 8.0,
            pad_l: 7.0,
            pad_r: 13.0,
            avatar: 24.0,
            gap: 10.0,
            shadow: Some((2.0, 10.0, "rgba(0,0,0,0.333)")),
            chevron_on_hover: true,
        }
    }
}

const CHEV_W: f64 = 14.0;

fn chevron(view: &SessionPillView<'_>, style: &SessionPillStyle) -> bool {
    !style.chevron_on_hover || view.hovered || view.open
}

/// Width the pill needs for a label.
pub fn session_pill_width(ctx: &mut dyn RenderContext, view: &SessionPillView<'_>, style: &SessionPillStyle) -> f64 {
    ctx.set_font(style.font);
    let t = ctx.measure_text(view.label);
    let av = if view.critter.is_some() { style.avatar + style.gap } else { 0.0 };
    let ch = if chevron(view, style) { CHEV_W } else { 0.0 };
    2.0 + style.pad_l + av + t + ch + style.pad_r
}

/// Pill rect centred at the top of `area`.
pub fn session_pill_rect(area: Rect, width: f64, style: &SessionPillStyle) -> Rect {
    let w = width.min(area.width - 24.0).max(40.0);
    Rect::new(area.x + (area.width - w) / 2.0, area.y + style.top, w, style.height)
}

/// CSS-style rounded rect (circular corners as cubic Béziers).
fn round_rect(ctx: &mut dyn RenderContext, r: Rect, rad: f64) {
    let rad = rad.min(r.height / 2.0).min(r.width / 2.0).max(0.0);
    let k = rad * 0.552_284_749_8;
    let (x0, y0, x1, y1) = (r.x, r.y, r.x + r.width, r.y + r.height);
    ctx.begin_path();
    ctx.move_to(x0 + rad, y0);
    ctx.line_to(x1 - rad, y0);
    ctx.bezier_curve_to(x1 - rad + k, y0, x1, y0 + rad - k, x1, y0 + rad);
    ctx.line_to(x1, y1 - rad);
    ctx.bezier_curve_to(x1, y1 - rad + k, x1 - rad + k, y1, x1 - rad, y1);
    ctx.line_to(x0 + rad, y1);
    ctx.bezier_curve_to(x0 + rad - k, y1, x0, y1 - rad + k, x0, y1 - rad);
    ctx.line_to(x0, y0 + rad);
    ctx.bezier_curve_to(x0, y0 + rad - k, x0 + rad - k, y0, x0 + rad, y0);
    ctx.close_path();
}

/// Paint the pill into `rect`.
pub fn draw_session_pill(ctx: &mut dyn RenderContext, rect: Rect, view: &SessionPillView<'_>, style: &SessionPillStyle) {
    let r = rect.height / 2.0;
    if let Some((dy, blur, col)) = style.shadow {
        ctx.set_shadow(0.0, dy, blur, col);
    }
    round_rect(ctx, rect, r);
    ctx.set_fill_color(style.border);
    ctx.fill();
    ctx.clear_shadow();
    round_rect(ctx, Rect::new(rect.x + 1.0, rect.y + 1.0, rect.width - 2.0, rect.height - 2.0), r - 1.0);
    ctx.set_fill_color(if view.hovered || view.open { style.bg_hover } else { style.bg });
    ctx.fill();
    let mut x = rect.x + 1.0 + style.pad_l;
    let cy = rect.y + rect.height / 2.0;
    if let Some(c) = &view.critter {
        let m = view.motion.unwrap_or(CritterMotion { reduced: true, ..Default::default() });
        draw_critter_motion(ctx, Rect::new(x, cy - style.avatar / 2.0, style.avatar, style.avatar), c, &m);
        x += style.avatar + style.gap;
    }
    ctx.set_font(style.font);
    ctx.set_fill_color(style.text);
    let lh = style.font_px * 1.5;
    fill_text_css(ctx, view.label, x, cy - lh / 2.0, style.font_px, lh, INTER, TextAlign::Left);
    if chevron(view, style) {
        // ›  (600 16px, #acacac)
        let chx = rect.x + rect.width - 1.0 - style.pad_r - 8.0;
        ctx.begin_path();
        ctx.move_to(chx, cy - 4.0);
        ctx.line_to(chx + 4.0, cy);
        ctx.line_to(chx, cy + 4.0);
        ctx.set_stroke_color(style.sub);
        ctx.set_stroke_width(1.6);
        ctx.stroke();
    }
}

/// Declare the pill's click rect.
pub fn register_session_pill(coord: &mut InputCoordinator, id: impl Into<WidgetId>, rect: Rect, layer: &LayerId) {
    coord.register_atomic(id, WidgetKind::Button, rect, Sense::CLICK, layer);
}

/// Shared rounded-rect path helper (also used by the composer).
pub fn path_round_rect(ctx: &mut dyn RenderContext, r: Rect, rad: f64) {
    round_rect(ctx, r, rad);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rect_is_centred() {
        let s = SessionPillStyle::default();
        let r = session_pill_rect(Rect::new(100.0, 0.0, 600.0, 400.0), 200.0, &s);
        assert_eq!(r.x, 300.0);
        assert_eq!(r.height, 40.0);
        assert_eq!(r.y, 8.0);
    }
}
