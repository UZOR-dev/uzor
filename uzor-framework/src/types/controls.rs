//! Floating window controls: the minimize / maximize / close buttons drawn in
//! the app's own space (no separate title bar) for an undecorated window.
//!
//! Set through [`ChromeModel::floating`](super::command::ChromeModel::floating).
//! The layout engine hit-tests the buttons (above content), compose paints them
//! last (above the dock), and the band `drag_height` tall along the top edge is
//! the window drag zone wherever no content widget sits under the pointer.
//! Placement follows the platform: Windows / Linux top-right (min, max, close),
//! macOS top-left in traffic-light order (close, min, max).

use uzor::render::RenderContext;
use uzor::Rect;

/// Which corner the buttons sit in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ControlsPlacement {
    /// The platform's usual place (macOS: left, otherwise right).
    #[default]
    Auto,
    /// Top-right, min / max / close (Windows, Linux).
    Right,
    /// Top-left, close / min / max (macOS).
    Left,
}

impl ControlsPlacement {
    /// `Auto` resolved for the compile target.
    pub fn resolve(self) -> ControlsPlacement {
        match self {
            ControlsPlacement::Auto if cfg!(target_os = "macos") => ControlsPlacement::Left,
            ControlsPlacement::Auto => ControlsPlacement::Right,
            p => p,
        }
    }
}

/// One floating control button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlButton {
    /// Minimize.
    Minimize,
    /// Maximize / restore.
    Maximize,
    /// Close.
    Close,
}

/// Look of the floating controls. Colours are CSS strings; a `None` fill is
/// transparent (the app's own surface shows through).
#[derive(Clone, Debug, PartialEq)]
pub struct WindowControlsStyle {
    /// Idle button fill (`None` = transparent).
    pub fill: Option<String>,
    /// Min / max hover fill.
    pub hover_fill: String,
    /// Min / max pressed fill.
    pub press_fill: String,
    /// Close hover fill.
    pub close_hover_fill: String,
    /// Glyph colour.
    pub glyph: String,
    /// Glyph colour on the close hover fill.
    pub close_hover_glyph: String,
    /// Button width (right placement), logical px.
    pub button_w: f64,
    /// Button height, logical px.
    pub button_h: f64,
    /// Glyph box size, logical px.
    pub glyph_size: f64,
    /// Glyph stroke width, logical px.
    pub stroke: f64,
    /// Left placement: dot diameter / gap / inset (traffic lights).
    pub dot: f64,
    /// Left placement: gap between dots.
    pub dot_gap: f64,
    /// Left placement: inset from the top-left corner.
    pub dot_inset: f64,
}

impl Default for WindowControlsStyle {
    /// Dark, Windows-11-like: transparent idle, subtle hover, red close.
    fn default() -> Self {
        Self {
            fill: None,
            hover_fill: "rgba(255,255,255,0.08)".into(),
            press_fill: "rgba(255,255,255,0.05)".into(),
            close_hover_fill: "#e81123".into(),
            glyph: "#e6e6e6".into(),
            close_hover_glyph: "#ffffff".into(),
            button_w: 46.0,
            button_h: 32.0,
            glyph_size: 10.0,
            stroke: 1.0,
            dot: 12.0,
            dot_gap: 8.0,
            dot_inset: 14.0,
        }
    }
}

impl WindowControlsStyle {
    /// The default look with the idle fill and glyph from design tokens
    /// (`fill` = app chrome surface, glyph = primary text).
    pub fn from_tokens(tokens: &uzor::tokens::Tokens) -> Self {
        Self {
            fill: Some(tokens.semantic.surface_app_chrome.to_css().into_owned()),
            glyph: tokens.semantic.text_primary.to_css().into_owned(),
            ..Self::default()
        }
    }

    /// Builder: idle fill (`None` = transparent).
    pub fn with_fill(mut self, fill: Option<&str>) -> Self {
        self.fill = fill.map(str::to_string);
        self
    }
}

/// Floating controls of one window.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FloatingControls {
    /// Corner.
    pub placement: ControlsPlacement,
    /// Look.
    pub style: WindowControlsStyle,
    /// Height of the top drag band, logical px (`0` = no drag band).
    pub drag_height: f64,
    /// Show the maximize button.
    pub show_maximize: bool,
    /// Show the minimize button.
    pub show_minimize: bool,
    /// 1 px outer window border painted inside the viewport edge (`None` =
    /// none). On Windows set `WindowSpec::border_color` too (DWM border).
    pub border: Option<String>,
}

impl FloatingControls {
    /// Min, max and close in the platform's place, a 40 px drag band.
    pub fn new(style: WindowControlsStyle) -> Self {
        Self { placement: ControlsPlacement::Auto, style, drag_height: 40.0, show_maximize: true, show_minimize: true, border: None }
    }

    /// The buttons and their rects inside `viewport`, in paint order.
    pub fn buttons(&self, viewport: Rect) -> Vec<(ControlButton, Rect)> {
        let mut order = Vec::with_capacity(3);
        let s = &self.style;
        match self.placement.resolve() {
            ControlsPlacement::Left => {
                order.push(ControlButton::Close);
                if self.show_minimize {
                    order.push(ControlButton::Minimize);
                }
                if self.show_maximize {
                    order.push(ControlButton::Maximize);
                }
                let y = viewport.y + s.dot_inset;
                order
                    .into_iter()
                    .enumerate()
                    .map(|(i, b)| (b, Rect::new(viewport.x + s.dot_inset + i as f64 * (s.dot + s.dot_gap), y, s.dot, s.dot)))
                    .collect()
            }
            _ => {
                if self.show_minimize {
                    order.push(ControlButton::Minimize);
                }
                if self.show_maximize {
                    order.push(ControlButton::Maximize);
                }
                order.push(ControlButton::Close);
                let n = order.len() as f64;
                let x0 = viewport.x + viewport.width - n * s.button_w;
                order
                    .into_iter()
                    .enumerate()
                    .map(|(i, b)| (b, Rect::new(x0 + i as f64 * s.button_w, viewport.y, s.button_w, s.button_h)))
                    .collect()
            }
        }
    }

    /// The button under `p`, if any.
    pub fn button_at(&self, viewport: Rect, p: (f64, f64)) -> Option<ControlButton> {
        self.buttons(viewport).into_iter().find(|(_, r)| contains(*r, p)).map(|(b, _)| b)
    }

    /// `p` is in the top drag band (buttons excluded).
    pub fn in_drag_band(&self, viewport: Rect, p: (f64, f64)) -> bool {
        self.drag_height > 0.0
            && contains(Rect::new(viewport.x, viewport.y, viewport.width, self.drag_height), p)
            && self.button_at(viewport, p).is_none()
    }

    /// Paint the buttons. `hover` / `pressed` = the button under the pointer.
    pub fn paint(&self, ctx: &mut dyn RenderContext, viewport: Rect, maximized: bool, hover: Option<ControlButton>, pressed: bool) {
        let s = &self.style;
        if let Some(c) = &self.border {
            if !maximized {
                ctx.set_stroke_color(c);
                ctx.set_stroke_width(1.0);
                ctx.begin_path();
                ctx.rect(viewport.x + 0.5, viewport.y + 0.5, viewport.width - 1.0, viewport.height - 1.0);
                ctx.stroke();
            }
        }
        let left = self.placement.resolve() == ControlsPlacement::Left;
        for (b, r) in self.buttons(viewport) {
            let hot = hover == Some(b);
            if left {
                let c = match (hot || hover.is_some(), b) {
                    (true, ControlButton::Close) => "#ff5f57",
                    (true, ControlButton::Minimize) => "#febc2e",
                    (true, ControlButton::Maximize) => "#28c840",
                    (false, _) => s.fill.as_deref().unwrap_or("rgba(255,255,255,0.22)"),
                };
                ctx.set_fill_color(c);
                ctx.begin_path();
                ctx.arc(r.x + r.width / 2.0, r.y + r.height / 2.0, r.width / 2.0, 0.0, std::f64::consts::TAU);
                ctx.fill();
                continue;
            }
            let fill = match (hot, b) {
                (true, ControlButton::Close) => Some(s.close_hover_fill.as_str()),
                (true, _) if pressed => Some(s.press_fill.as_str()),
                (true, _) => Some(s.hover_fill.as_str()),
                (false, _) => s.fill.as_deref(),
            };
            if let Some(f) = fill {
                ctx.set_fill_color(f);
                ctx.fill_rect(r.x, r.y, r.width, r.height);
            }
            let glyph = if hot && b == ControlButton::Close { &s.close_hover_glyph } else { &s.glyph };
            let g = s.glyph_size;
            // Pixel-snapped glyph box, centred.
            let gx = (r.x + (r.width - g) / 2.0).round() + 0.5;
            let gy = (r.y + (r.height - g) / 2.0).round() + 0.5;
            ctx.set_stroke_color(glyph);
            ctx.set_stroke_width(s.stroke);
            ctx.begin_path();
            match b {
                ControlButton::Minimize => {
                    ctx.move_to(gx, gy + g / 2.0);
                    ctx.line_to(gx + g, gy + g / 2.0);
                }
                ControlButton::Maximize if maximized => {
                    // Restore: two overlapping squares.
                    let k = 2.0;
                    ctx.rect(gx, gy + k, g - k, g - k);
                    ctx.move_to(gx + k, gy + k);
                    ctx.line_to(gx + k, gy);
                    ctx.line_to(gx + g, gy);
                    ctx.line_to(gx + g, gy + g - k);
                    ctx.line_to(gx + g - k, gy + g - k);
                }
                ControlButton::Maximize => ctx.rect(gx, gy, g - 1.0, g - 1.0),
                ControlButton::Close => {
                    ctx.move_to(gx, gy);
                    ctx.line_to(gx + g, gy + g);
                    ctx.move_to(gx + g, gy);
                    ctx.line_to(gx, gy + g);
                }
            }
            ctx.stroke();
        }
    }
}

fn contains(r: Rect, p: (f64, f64)) -> bool {
    p.0 >= r.x && p.0 < r.x + r.width && p.1 >= r.y && p.1 < r.y + r.height
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vp() -> Rect {
        Rect::new(0.0, 0.0, 1000.0, 700.0)
    }

    #[test]
    fn right_placement_is_min_max_close_at_top_right() {
        let c = FloatingControls { placement: ControlsPlacement::Right, ..FloatingControls::new(WindowControlsStyle::default()) };
        let b = c.buttons(vp());
        assert_eq!(b.iter().map(|x| x.0).collect::<Vec<_>>(), [ControlButton::Minimize, ControlButton::Maximize, ControlButton::Close]);
        assert_eq!(b[2].1, Rect::new(954.0, 0.0, 46.0, 32.0));
        assert_eq!(c.button_at(vp(), (990.0, 10.0)), Some(ControlButton::Close));
        assert_eq!(c.button_at(vp(), (920.0, 10.0)), Some(ControlButton::Maximize));
        assert_eq!(c.button_at(vp(), (500.0, 10.0)), None);
    }

    #[test]
    fn left_placement_is_traffic_light_order() {
        let c = FloatingControls { placement: ControlsPlacement::Left, ..FloatingControls::new(WindowControlsStyle::default()) };
        let b = c.buttons(vp());
        assert_eq!(b.iter().map(|x| x.0).collect::<Vec<_>>(), [ControlButton::Close, ControlButton::Minimize, ControlButton::Maximize]);
        assert_eq!(b[0].1.x, 14.0);
    }

    #[test]
    fn drag_band_excludes_buttons() {
        let c = FloatingControls { placement: ControlsPlacement::Right, ..FloatingControls::new(WindowControlsStyle::default()) };
        assert!(c.in_drag_band(vp(), (500.0, 20.0)));
        assert!(!c.in_drag_band(vp(), (990.0, 10.0)));
        assert!(!c.in_drag_band(vp(), (500.0, 60.0)));
    }

    #[test]
    fn auto_resolves_per_platform() {
        let want = if cfg!(target_os = "macos") { ControlsPlacement::Left } else { ControlsPlacement::Right };
        assert_eq!(ControlsPlacement::Auto.resolve(), want);
    }

    #[test]
    fn transparent_fill_is_allowed() {
        let s = WindowControlsStyle::default().with_fill(None);
        assert!(s.fill.is_none());
        assert_eq!(WindowControlsStyle::default().with_fill(Some("#111")).fill.as_deref(), Some("#111"));
    }
}
