//! Paint slots driven by registered painters ([`uzor::ui::dom::SlotRegistry`]): a
//! revenue sparkline that tracks the pointer (role `img`, its own accessible name) and
//! a dial that takes the wheel, a drag and the arrow keys (role `slider`, a Tab stop),
//! in a Settings-style page. Colours come from the sheet (`color`, `--k-*`
//! variables), so the light theme is one class away. [`SlotsDemo::tree`] is pure and
//! [`SlotsDemo::registry`] holds the painters: the same page renders live, in tests
//! and in the `dom-slots-frames` PNG bin.

use std::f64::consts::PI;

use uzor::render::{RenderContext, TextAlign, TextBaseline};
use uzor::ui::dom::{El, SlotAction, SlotEvent, SlotPaint, SlotPainter, SlotRegistry};
use uzor::ui::style::Sheet;

/// Page rules on top of the kit sheet.
pub const SLOTS_CSS: &str = r#"
div.ds-page { display: flex; flex-direction: column; gap: 14px; padding: 20px; background: var(--k-bg); color: var(--k-fg) }
div.ds-page.light { --k-bg: #f6f7f9; --k-panel: #ffffff; --k-fg: #1d1f24; --k-muted: #6b6f78; --k-line: #dfe1e6; --k-hover: #eef0f4 }
div.ds-h1 { font-size: 15px; font-weight: 600 }
div.ds-sub { font-size: 12px; color: var(--k-muted) }
div.ds-row { display: flex; gap: 14px }
div.ds-card { display: flex; flex-direction: column; gap: 8px; padding: 12px 14px; border-radius: 10px; border: 1px solid var(--k-line); background: var(--k-panel); flex-grow: 1 }
div.ds-card.dial { flex-grow: 0; width: 200px }
div.ds-cap { font-size: 12px; font-weight: 600 }
div.ds-note { font-size: 11px; color: var(--k-muted) }
div.ds-spark { height: 120px; color: var(--k-fg) }
div.ds-dial { height: 120px; color: var(--k-fg) }
"#;

/// Strings: Russian by default, English as the fallback.
fn tx(ru: bool, r: &'static str, e: &'static str) -> &'static str {
    if ru {
        r
    } else {
        e
    }
}

/// Demo state.
#[derive(Clone, Debug, PartialEq)]
pub struct SlotsDemo {
    pub ru: bool,
    pub dark: bool,
}

impl Default for SlotsDemo {
    fn default() -> Self {
        SlotsDemo { ru: true, dark: true }
    }
}

impl SlotsDemo {
    /// Kit rules and the page's.
    pub fn sheet() -> Sheet {
        Sheet::parse(&[uzor::ui::domkit::KIT_CSS, SLOTS_CSS].concat()).expect("slots demo css")
    }

    /// The page: two cards, each a paint slot with a caption.
    pub fn tree(&self, w: f64) -> El {
        let ru = self.ru;
        let page = if self.dark { "div.ds-page" } else { "div.ds-page.light" };
        El::new(page)
            .css(&format!("width: {w}px"))
            .child(El::new("div.ds-h1").text(tx(ru, "Слоты с отрисовкой", "Paint slots")))
            .child(El::new("div.ds-sub").text(tx(ru, "Каждый слот рисует и слушает свой зарегистрированный painter", "Each slot is painted and driven by its registered painter")))
            .child(
                El::new("div.ds-row")
                    .child(
                        El::new("div.ds-card")
                            .child(El::new("div.ds-cap").text(tx(ru, "Выручка · 30 дней", "Revenue · 30 days")))
                            .child(El::new("div.ds-spark").paint_slot("spark"))
                            .child(El::new("div.ds-note").text(tx(ru, "Наведите курсор: значение дня", "Hover: the day's value"))),
                    )
                    .child(
                        El::new("div.ds-card.dial")
                            .child(El::new("div.ds-cap").text(tx(ru, "Громкость", "Volume")))
                            .child(El::new("div.ds-dial").paint_slot("dial"))
                            .child(El::new("div.ds-note").text(tx(ru, "Колесо, мышь, стрелки", "Wheel, drag, arrows"))),
                    ),
            )
    }

    /// Lay the page out `w` wide (`h` the viewport), fill the background and paint it
    /// with `reg`'s painters in the slots. Returns the laid-out tree.
    pub fn render(&self, ctx: &mut dyn RenderContext, reg: &mut SlotRegistry, w: f64, h: f64) -> uzor::ui::dom::Dom {
        ctx.set_fill_color(if self.dark { "#17181b" } else { "#f6f7f9" });
        ctx.fill_rect(0.0, 0.0, w, h);
        let d = uzor::ui::dom::lay_out_in(ctx, &Self::sheet(), &self.tree(w), 0.0, 0.0, w, Some(uzor::Rect::new(0.0, 0.0, w, h)), &|_| uzor::ui::style::State::NONE);
        reg.paint(ctx, &d);
        d
    }

    /// The painters, keyed by slot.
    pub fn registry(&self) -> SlotRegistry {
        let mut r = SlotRegistry::new();
        r.register("spark", Spark::new(self.ru, revenue())).register("dial", Dial::new(self.ru, 0.4));
        r
    }
}

/// 30 made-up daily values.
pub fn revenue() -> Vec<f64> {
    (0..30).map(|i| {
        let x = i as f64;
        900.0 + 12.0 * x + 140.0 * (x * 0.55).sin() + 60.0 * (x * 1.7).cos()
    })
    .collect()
}

/// A sparkline with a hover readout.
#[derive(Clone, Debug, PartialEq)]
pub struct Spark {
    pub ru: bool,
    pub values: Vec<f64>,
    /// The hovered day.
    pub hover: Option<usize>,
}

impl Spark {
    pub fn new(ru: bool, values: Vec<f64>) -> Self {
        Spark { ru, values, hover: None }
    }
    fn range(&self) -> (f64, f64) {
        let lo = self.values.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = self.values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        if hi > lo {
            (lo, hi)
        } else {
            (lo - 1.0, lo + 1.0)
        }
    }
    /// The plot area inside the slot (room for the readout on top).
    fn plot(w: f64, h: f64) -> (f64, f64, f64, f64) {
        (4.0, 22.0, (w - 8.0).max(1.0), (h - 26.0).max(1.0))
    }
    /// The day under slot-local `x`.
    pub fn day_at(&self, x: f64, w: f64) -> Option<usize> {
        let n = self.values.len();
        if n == 0 {
            return None;
        }
        let (px, _, pw, _) = Self::plot(w, 0.0);
        let t = ((x - px) / pw).clamp(0.0, 1.0);
        Some(((t * (n - 1) as f64).round() as usize).min(n - 1))
    }
}

impl SlotPainter for Spark {
    fn paint(&mut self, ctx: &mut dyn RenderContext, f: &SlotPaint<'_>) {
        let r = f.rect;
        let n = self.values.len();
        if n < 2 {
            return;
        }
        let fg = f.color("#e8e8ea").to_string();
        let muted = f.var("--k-muted", "#9a9ca3").to_string();
        let line = f.var("--k-line", "#33363c").to_string();
        let (px, py, pw, ph) = Self::plot(r.width, r.height);
        let (x0, y0) = (r.x + px, r.y + py);
        let (lo, hi) = self.range();
        let at = |i: usize| (x0 + pw * i as f64 / (n - 1) as f64, y0 + ph - ph * (self.values[i] - lo) / (hi - lo));
        // Grid: three hairlines.
        ctx.set_fill_color(&line);
        for k in 0..3 {
            ctx.fill_rect(x0, (y0 + ph * k as f64 / 2.0).round(), pw, 1.0);
        }
        // Area under the line, faint.
        ctx.begin_path();
        ctx.move_to(x0, y0 + ph);
        for i in 0..n {
            let (x, y) = at(i);
            ctx.line_to(x, y);
        }
        ctx.line_to(x0 + pw, y0 + ph);
        ctx.close_path();
        ctx.set_fill_color(&fg);
        ctx.set_global_alpha(0.08);
        ctx.fill();
        ctx.set_global_alpha(1.0);
        // The line.
        ctx.begin_path();
        for i in 0..n {
            let (x, y) = at(i);
            if i == 0 {
                ctx.move_to(x, y);
            } else {
                ctx.line_to(x, y);
            }
        }
        ctx.set_stroke_color(&fg);
        ctx.set_stroke_width(1.5);
        ctx.set_line_join("round");
        ctx.stroke();
        // Readout: the hovered day, else the last one.
        let i = self.hover.unwrap_or(n - 1);
        let (x, y) = at(i);
        if self.hover.is_some() {
            ctx.set_fill_color(&muted);
            ctx.fill_rect(x.round(), y0, 1.0, ph);
        }
        ctx.begin_path();
        ctx.arc(x, y, 3.0, 0.0, 2.0 * PI);
        ctx.set_fill_color(&fg);
        ctx.fill();
        ctx.set_font("600 13px Inter");
        ctx.set_text_baseline(TextBaseline::Top);
        ctx.set_text_align(TextAlign::Left);
        ctx.fill_text(&format!("{:.0}", self.values[i]), r.x + px, r.y);
        ctx.set_font("400 11px Inter");
        ctx.set_fill_color(&muted);
        ctx.set_text_align(TextAlign::Right);
        let day = format!("{} {}", tx(self.ru, "день", "day"), i + 1);
        ctx.fill_text(&day, r.x + px + pw, r.y + 2.0);
        ctx.set_text_align(TextAlign::Left);
    }

    fn event(&mut self, ev: &SlotEvent, f: &SlotPaint<'_>) -> Option<SlotAction> {
        match *ev {
            SlotEvent::Enter { x, .. } | SlotEvent::Move { x, .. } | SlotEvent::Pan { x, .. } => self.hover = self.day_at(x, f.rect.width),
            SlotEvent::Leave | SlotEvent::PanEnd { .. } => self.hover = None,
            SlotEvent::Click { x, .. } => return self.day_at(x, f.rect.width).map(|d| SlotAction::new("day").num(d as f64)),
            _ => {}
        }
        None
    }

    fn role(&self) -> &'static str {
        "img"
    }

    fn name(&self) -> Option<String> {
        let last = self.values.last().copied().unwrap_or(0.0);
        Some(format!("{}: {:.0}", tx(self.ru, "Выручка за 30 дней, последний день", "Revenue over 30 days, last day"), last))
    }
}

/// A dial 0 … 1: the wheel, a vertical drag and the arrow keys turn it.
#[derive(Clone, Debug, PartialEq)]
pub struct Dial {
    pub ru: bool,
    pub value: f64,
}

impl Dial {
    pub fn new(ru: bool, value: f64) -> Self {
        Dial { ru, value: value.clamp(0.0, 1.0) }
    }
    fn set(&mut self, v: f64) -> Option<SlotAction> {
        let v = (v.clamp(0.0, 1.0) * 100.0).round() / 100.0;
        if v == self.value {
            return None;
        }
        self.value = v;
        Some(SlotAction::new("value").num(v))
    }
}

impl SlotPainter for Dial {
    fn paint(&mut self, ctx: &mut dyn RenderContext, f: &SlotPaint<'_>) {
        let r = f.rect;
        let fg = f.color("#e8e8ea").to_string();
        let line = f.var("--k-line", "#33363c").to_string();
        let (cx, cy) = (r.x + r.width / 2.0, r.y + r.height / 2.0);
        let rad = (r.width.min(r.height) / 2.0 - 8.0).max(4.0);
        // 270° sweep from 7:30 clockwise to 4:30.
        let (a0, sweep) = (0.75 * PI, 1.5 * PI);
        ctx.set_line_cap("round");
        ctx.set_stroke_width(6.0);
        ctx.set_stroke_color(&line);
        ctx.begin_path();
        ctx.arc(cx, cy, rad, a0, a0 + sweep);
        ctx.stroke();
        if self.value > 0.0 {
            ctx.set_stroke_color(&fg);
            ctx.begin_path();
            ctx.arc(cx, cy, rad, a0, a0 + sweep * self.value);
            ctx.stroke();
        }
        ctx.set_line_cap("butt");
        // The knob at the value.
        let a = a0 + sweep * self.value;
        ctx.begin_path();
        ctx.arc(cx + rad * a.cos(), cy + rad * a.sin(), 6.0, 0.0, 2.0 * PI);
        ctx.set_fill_color(&fg);
        ctx.fill();
        ctx.set_font("600 18px Inter");
        ctx.set_text_align(TextAlign::Center);
        ctx.set_text_baseline(TextBaseline::Middle);
        ctx.fill_text(&format!("{:.0}%", self.value * 100.0), cx, cy);
        ctx.set_text_align(TextAlign::Left);
        // Focus ring: a 2 px ring around the dial while it has the keyboard.
        if f.focused {
            ctx.set_stroke_color(&fg);
            ctx.set_stroke_width(2.0);
            ctx.begin_path();
            ctx.arc(cx, cy, rad + 7.0, 0.0, 2.0 * PI);
            ctx.stroke();
        }
    }

    fn event(&mut self, ev: &SlotEvent, _: &SlotPaint<'_>) -> Option<SlotAction> {
        use uzor::input::KeyCode as K;
        match ev {
            SlotEvent::Wheel { dy, .. } => self.set(self.value - dy / 100.0),
            SlotEvent::Drag { dy, .. } | SlotEvent::Pan { dy, .. } => self.set(self.value - dy / 150.0),
            SlotEvent::Key { code: K::ArrowUp | K::ArrowRight, .. } => self.set(self.value + 0.05),
            SlotEvent::Key { code: K::ArrowDown | K::ArrowLeft, .. } => self.set(self.value - 0.05),
            SlotEvent::Key { code: K::Home, .. } => self.set(0.0),
            SlotEvent::Key { code: K::End, .. } => self.set(1.0),
            _ => None,
        }
    }

    fn role(&self) -> &'static str {
        "slider"
    }

    fn name(&self) -> Option<String> {
        Some(format!("{} {:.0}%", tx(self.ru, "Громкость", "Volume"), self.value * 100.0))
    }

    fn focusable(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uzor::input::KeyCode;
    use uzor::testing::NullRenderContext;
    use uzor::ui::dom::{self, SlotInput, SlotValue};
    use uzor::ui::style::State;

    #[test]
    fn the_page_has_two_slots_with_painters_roles_and_names() {
        let g = SlotsDemo::default();
        let mut ctx = NullRenderContext::default();
        let d = dom::lay_out(&mut ctx, &SlotsDemo::sheet(), &g.tree(720.0), 0.0, 0.0, 720.0, &|_| State::NONE);
        let keys: Vec<&str> = d.slots().map(|(k, _)| k).collect();
        assert_eq!(keys, ["spark", "dial"]);
        let mut reg = g.registry();
        reg.paint(&mut ctx, &d);
        let a11y = reg.a11y(&d);
        let spark = a11y.iter().find(|n| n.id == "spark").unwrap();
        let dial = a11y.iter().find(|n| n.id == "dial").unwrap();
        assert_eq!(spark.role, "img");
        assert!(spark.name.starts_with("Выручка"), "{}", spark.name);
        assert_eq!((dial.role, dial.name.as_str(), dial.focusable), ("slider", "Громкость 40%", true));
    }

    #[test]
    fn the_dial_turns_by_wheel_drag_and_keys_and_the_spark_tracks_the_pointer() {
        let g = SlotsDemo::default();
        let mut reg = g.registry();
        let r = uzor::Rect::new(100.0, 100.0, 300.0, 120.0);
        let a = reg.input("dial", r, &SlotInput { pointer: Some((150.0, 150.0)), wheel: (0.0, -10.0), ..Default::default() });
        assert_eq!(a[0].value, SlotValue::Num(0.5));
        reg.input("dial", r, &SlotInput { pointer: Some((150.0, 150.0)), pressed: Some((150.0, 150.0)), ..Default::default() });
        reg.input("dial", r, &SlotInput { pointer: Some((150.0, 120.0)), pressed: Some((150.0, 120.0)), ..Default::default() });
        assert_eq!(reg.get::<Dial>("dial").unwrap().value, 0.7);
        reg.input("dial", r, &SlotInput { focused: true, key: Some((KeyCode::End, None)), ..Default::default() });
        assert_eq!(reg.get::<Dial>("dial").unwrap().value, 1.0);
        reg.input("spark", r, &SlotInput { pointer: Some((104.0, 150.0)), ..Default::default() });
        assert_eq!(reg.get::<Spark>("spark").unwrap().hover, Some(0));
        reg.input("spark", r, &SlotInput { pointer: Some((396.0, 150.0)), ..Default::default() });
        assert_eq!(reg.get::<Spark>("spark").unwrap().hover, Some(29));
        reg.input("spark", r, &SlotInput::default());
        assert_eq!(reg.get::<Spark>("spark").unwrap().hover, None);
    }
}
