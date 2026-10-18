//! Session pill: a centred, floating rounded label (critter + name + chevron)
//! that opens the session control panel.

use crate::input::core::coordinator::LayerId;
use crate::input::{InputCoordinator, Sense, WidgetKind};
use crate::render::{RenderContext, TextAlign, TextBaseline};
use crate::types::{Rect, WidgetId};

use super::critter::{draw_critter, CritterView};

/// Plain view data.
#[derive(Clone, Debug)]
pub struct SessionPillView<'a> {
    /// Session name.
    pub label: &'a str,
    /// Avatar on the left, if any.
    pub critter: Option<CritterView>,
    /// Hover tint.
    pub hovered: bool,
    /// Panel currently open.
    pub open: bool,
}

/// Colours and font.
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
    /// Height.
    pub height: f64,
}

impl Default for SessionPillStyle {
    fn default() -> Self {
        Self {
            bg: "#ffffff",
            bg_hover: "#f2f2f3",
            border: "#e4e4e7",
            text: "#18181b",
            sub: "#8e8e96",
            font: "500 14px sans-serif",
            height: 36.0,
        }
    }
}

/// Width the pill needs for a label.
pub fn session_pill_width(ctx: &mut dyn RenderContext, view: &SessionPillView<'_>, style: &SessionPillStyle) -> f64 {
    ctx.set_font(style.font);
    let t = ctx.measure_text(view.label);
    let av = if view.critter.is_some() { 26.0 } else { 0.0 };
    14.0 + av + t + 8.0 + 10.0 + 14.0
}

/// Pill rect centred at the top of `area`.
pub fn session_pill_rect(area: Rect, width: f64, style: &SessionPillStyle) -> Rect {
    let w = width.min(area.width - 24.0).max(40.0);
    Rect::new(area.x + (area.width - w) / 2.0, area.y + 10.0, w, style.height)
}

fn round_rect(ctx: &mut dyn RenderContext, r: Rect, rad: f64) {
    let rad = rad.min(r.height / 2.0).min(r.width / 2.0);
    ctx.begin_path();
    ctx.move_to(r.x + rad, r.y);
    ctx.line_to(r.x + r.width - rad, r.y);
    ctx.quadratic_curve_to(r.x + r.width, r.y, r.x + r.width, r.y + rad);
    ctx.line_to(r.x + r.width, r.y + r.height - rad);
    ctx.quadratic_curve_to(r.x + r.width, r.y + r.height, r.x + r.width - rad, r.y + r.height);
    ctx.line_to(r.x + rad, r.y + r.height);
    ctx.quadratic_curve_to(r.x, r.y + r.height, r.x, r.y + r.height - rad);
    ctx.line_to(r.x, r.y + rad);
    ctx.quadratic_curve_to(r.x, r.y, r.x + rad, r.y);
    ctx.close_path();
}

/// Paint the pill.
pub fn draw_session_pill(ctx: &mut dyn RenderContext, rect: Rect, view: &SessionPillView<'_>, style: &SessionPillStyle) {
    round_rect(ctx, rect, rect.height / 2.0);
    ctx.set_fill_color(if view.hovered || view.open { style.bg_hover } else { style.bg });
    ctx.fill();
    ctx.set_stroke_color(style.border);
    ctx.set_stroke_width(1.0);
    ctx.stroke();
    let mut x = rect.x + 14.0;
    let cy = rect.y + rect.height / 2.0;
    if let Some(c) = &view.critter {
        draw_critter(ctx, Rect::new(x, cy - 10.0, 20.0, 20.0), c);
        x += 26.0;
    }
    ctx.set_font(style.font);
    ctx.set_fill_color(style.text);
    ctx.set_text_align(TextAlign::Left);
    ctx.set_text_baseline(TextBaseline::Middle);
    ctx.fill_text(view.label, x, cy);
    // chevron ›
    let chx = rect.x + rect.width - 20.0;
    ctx.begin_path();
    ctx.move_to(chx, cy - 4.0);
    ctx.line_to(chx + 4.0, cy);
    ctx.line_to(chx, cy + 4.0);
    ctx.set_stroke_color(style.sub);
    ctx.set_stroke_width(1.6);
    ctx.stroke();
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
        assert_eq!(r.height, 36.0);
    }
}
