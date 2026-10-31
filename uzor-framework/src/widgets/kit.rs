//! The component kit: one builder shape for the reusable pieces of a
//! DOM-like app (roadmap item 10).
//!
//! Every piece follows the same three steps:
//!
//! 1. `kit::thing(id, rect, data)` starts a builder. The id is a stable
//!    [`WidgetId`]; child parts get `"{id}:…"` ids (`"{id}:tab:2"`,
//!    `"{id}:row:5"`, `"{id}:footer:0"`).
//! 2. Chain options (`.selected(i)`, `.bind(&mut v)`, `.disabled(true)`, …).
//! 3. `.build(cx)` registers hits, names the parts for assistive tech,
//!    paints with the token theme of `cx`, and returns a [`Response`].
//!
//! The app keeps its own values (text, selection, sizes): a piece reads them
//! from the builder and reports changes through `Response` or `.bind`. State
//! that only exists to drive a widget (a select's open flag, a dialog's
//! drag position, a splitter's drag) lives in the frame's widget state store
//! under the piece's id, as a DOM element keeps its own `:open`.
//!
//! | Piece    | Start                          | Reports                         |
//! |----------|--------------------------------|---------------------------------|
//! | button   | [`button`]`(id, rect)`         | `.on_click`, `.bind_count`      |
//! | input    | [`input`]`(id, rect, &mut text)` | `changed`, focused; edits itself |
//! | select   | [`select`]`(id, rect, options)` + `.key`, `.within` | `index` picked, `changed`, `checked` open |
//! | menu     | [`menu`]`(id, at, items, open)`| `index` picked                  |
//! | dialog   | [`dialog`]`(id, rect)`         | `index` footer button, `clicked` close |
//! | tabs     | [`tabs`]`(id, rect, labels)`   | `index` clicked, `changed`      |
//! | list     | [`list`]`(id, rect, rows)`     | `index` clicked, `changed`      |
//! | splitter | [`splitter`]`(id, axis, span, &mut pos)` | `changed`, `pressed` while dragging |
//! | tooltip  | [`tooltip`]`(id, rect)`        | —                               |
//! | toast    | [`toasts`]`(id, area, &mut stack, now_ms)` | `changed` while any is shown |
//! | checkbox | [`checkbox`]`(id, rect)` + `.bind(&mut b)` | `.build_response`: `checked`, `changed` |
//! | switch   | [`switch`]`(id, rect)` + `.bind(&mut b)`   | `.build_response`: `checked`, `changed` |
//! | table    | [`table`]`(id, rect, columns, rows)`       | `index` row, `column` header, `changed` |
//! | drawer   | [`drawer`]`(id, area, &mut open)`          | `changed` when the scrim closes it |
//! | field    | [`field`]`(id, rect)` in the app's own chrome | `text` shown, `focused`; edits itself |
//!
//! An app that paints its own chrome (its CSS, not the token theme) and
//! registers hits itself uses the paint half from
//! `uzor::ui::widgets::composite::dialog`: `text_button_at` (sized text
//! button), `draw_close_button` (`✕` on a circle, a rounded rect or bare),
//! `fill_circle`, `draw_switch`, `draw_card`.
//!
//! Overlay-like pieces (select's list, menu, dialog) paint where they are
//! built: build them last in a panel, or inside an overlay hook, so they sit
//! on top.
//!
//! ```ignore
//! use uzor_framework::widgets::kit;
//! let r = kit::tabs("settings-tabs", tabs_rect, &["General", "Usage"]).bind(&mut self.tab).build(cx);
//! kit::input("name", name_rect, &mut self.name).placeholder("Name").build(cx);
//! if kit::button("save", save_rect).text("Save").build_response(cx).clicked { self.save(); }
//! ```

use uzor::input::{Sense, WidgetKind};
use uzor::ui::widgets::atomic::item::{draw_item, ItemRenderKind, ItemSettings, ItemView};
use uzor::ui::widgets::atomic::separator::{
    draw_separator, DefaultSeparatorStyle, SeparatorOrientation, SeparatorSettings, SeparatorType,
    SeparatorView,
};
use uzor::ui::widgets::atomic::tab::{draw_tab, register_tab_on_layer, TabConfig, TabSettings, TabView};
use uzor::ui::widgets::atomic::text_input::{
    draw_input, draw_input_cursor, InputType, InputView, TextFieldConfig as InputConfig, TextInputSettings,
};
use uzor::ui::widgets::atomic::toast::{draw_toast_stack, ToastStackState};
use uzor::ui::widgets::composite::context_menu::{
    draw_context_menu, register_input_coordinator_context_menu, ContextMenuItem,
    ContextMenuRenderKind, ContextMenuSettings, ContextMenuState, ContextMenuView,
};
use uzor::ui::widgets::composite::modal::render::draw_modal;
use uzor::ui::widgets::composite::select::{draw_list, filter, place, room, SelectKey, SelectState};
pub use uzor::ui::widgets::composite::select::SelectStyle;
use uzor::ui::widgets::composite::modal::{
    register_input_coordinator_modal, BackdropKind, FooterBtn, FooterBtnStyle, ModalRenderKind,
    ModalSettings, ModalState, ModalView,
};
use uzor::ui::widgets::atomic::button::ButtonView;
use uzor::render::RenderContext;
use uzor::{Rect, WidgetId, WidgetState};

use crate::handle::VisualView;
use crate::KeyInput;
use crate::types::spec::Spec;

use super::button::{button_settings, paint_button};
use super::host::ContentCx;
use super::paint::{interaction_state, parts, theme_of};
use super::splitter::{grab_rect, register as register_splitter, Splitter, SplitterEvent};

pub use super::lm::{
    button, checkbox, toggle as switch, tooltip, ButtonBuilder, CheckboxBuilder, ToggleBuilder as SwitchBuilder,
    TooltipBuilder,
};
pub use super::splitter::Axis;
/// The DOM widget catalog (CSS-styled controls, select popover, chat blocks).
pub use uzor::ui::domkit;
pub use uzor::ui::widgets::atomic::toast::{ToastSeverity, ToastType};

/// What a piece reports after `.build(cx)`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Response {
    /// The pointer is over the piece (any part of it).
    pub hovered: bool,
    /// The piece is held (a press, or a splitter drag).
    pub pressed: bool,
    /// Keyboard focus is on the piece or one of its parts.
    pub focused: bool,
    /// The piece itself was clicked (a dialog: its close button).
    pub clicked: bool,
    /// The value changed this frame (`.bind` targets are already updated).
    pub changed: bool,
    /// The part picked this frame: tab, row, option, menu item or footer button.
    pub index: Option<usize>,
    /// A table's header column clicked this frame (for sorting).
    pub column: Option<usize>,
    /// The state a checkbox or switch shows after this frame.
    pub checked: bool,
}

impl Response {
    fn of<S: Spec>(v: &VisualView<S>, id: &WidgetId) -> Self {
        Response {
            hovered: v.is_hover_within(id) || v.is_hovered(id),
            pressed: v.is_pressed(id),
            focused: v.is_focus_within(id) || v.is_focused(id),
            clicked: v.clicked(id),
            ..Default::default()
        }
    }
}

fn child(id: &WidgetId, part: &str, i: usize) -> WidgetId {
    WidgetId::from(format!("{}:{}:{}", id.as_str(), part, i))
}

/// The index of the clicked `"{id}:{part}:{i}"` child, if any.
fn picked<S: Spec>(v: &VisualView<S>, id: &WidgetId, part: &str) -> Option<usize> {
    let pre = format!("{}:{}:", id.as_str(), part);
    v.clicks().iter().find_map(|(w, _)| w.as_str().strip_prefix(pre.as_str())?.parse().ok())
}

/// A click this frame that landed outside every `"{id}…"` part.
fn clicked_outside<S: Spec>(v: &VisualView<S>, id: &WidgetId) -> bool {
    !v.clicks().is_empty() && !v.clicks().iter().any(|(w, _)| w.as_str().starts_with(id.as_str()))
}

fn focusable() -> Sense {
    (Sense::CLICK | Sense::HOVER).with_focus()
}

// -- button response --------------------------------------------------------

impl<'a> ButtonBuilder<'a> {
    /// [`build`](Self::build), then report like the other kit pieces.
    pub fn build_response<'c, C: ContentCx<'c>>(self, cx: &mut C) -> Response {
        let id = self.id_ref().clone();
        self.build(cx);
        cx.with_paint(|_, _, v, _| Response::of(v, &id))
    }
}

// -- checkbox / switch ------------------------------------------------------

impl<'a> CheckboxBuilder<'a> {
    /// [`build`](Self::build), name it for assistive tech, and report:
    /// `checked` is the new state, `changed` a user flip.
    pub fn build_response<'c, C: ContentCx<'c>>(self, cx: &mut C) -> Response {
        let (id, label) = self.id_label();
        let (id, label) = (id.clone(), label.unwrap_or("").to_owned());
        let on = self.build(cx);
        cx.with_paint(|_, w, v, _| {
            w.coord.set_a11y(&id, &label, Some(on));
            let r = Response::of(v, &id);
            Response { changed: r.clicked, checked: on, ..r }
        })
    }
}

impl<'a> SwitchBuilder<'a> {
    /// [`build`](Self::build), name it for assistive tech, and report:
    /// `checked` is the new state, `changed` a user flip.
    pub fn build_response<'c, C: ContentCx<'c>>(self, cx: &mut C) -> Response {
        let (id, label, disabled) = self.id_label();
        let (id, label) = (id.clone(), label.unwrap_or("").to_owned());
        let on = self.build(cx);
        cx.with_paint(|_, w, v, _| {
            w.coord.set_a11y(&id, &label, Some(on));
            let r = Response::of(v, &id);
            Response { changed: r.clicked && !disabled, checked: on, ..r }
        })
    }
}

// -- input ------------------------------------------------------------------

/// Single-line editable text field. Start with [`input`].
///
/// The framework's text store edits it: caret, selection (drag, double and
/// triple click, Shift+arrows, select all), clipboard, IME composition with
/// the candidate window at the caret, and undo / redo (Ctrl+Z, Ctrl+Shift+Z
/// or Ctrl+Y; typing undoes a word at a time). The bound `String` is
/// two-way: a user edit is written back to it (`changed`), and a value the
/// app sets is pushed into the field, which drops its undo history the way
/// `input.value = ..` does. Enter reports `TextIntent::Submitted`.
pub struct InputBuilder<'a> {
    id: WidgetId,
    rect: Rect,
    value: &'a mut String,
    placeholder: &'a str,
    label: Option<&'a str>,
    kind: InputType,
    disabled: bool,
    max_len: Option<usize>,
}

/// An editable text field bound to `value`.
pub fn input<'a>(id: impl Into<WidgetId>, rect: Rect, value: &'a mut String) -> InputBuilder<'a> {
    InputBuilder {
        id: id.into(),
        rect,
        value,
        placeholder: "",
        label: None,
        kind: InputType::Text,
        disabled: false,
        max_len: None,
    }
}

impl<'a> InputBuilder<'a> {
    /// Grey hint shown while empty. Also the accessible name without `.label`.
    pub fn placeholder(mut self, p: &'a str) -> Self {
        self.placeholder = p;
        self
    }
    /// Accessible name.
    pub fn label(mut self, l: &'a str) -> Self {
        self.label = Some(l);
        self
    }
    /// Field kind (text, number, search, password).
    pub fn kind(mut self, k: InputType) -> Self {
        self.kind = k;
        self
    }
    /// Masked password field (copy and cut are refused).
    pub fn password(self) -> Self {
        self.kind(InputType::Password)
    }
    /// Most chars the field takes, like `maxlength`.
    pub fn max_len(mut self, n: usize) -> Self {
        self.max_len = Some(n);
        self
    }
    /// Disabled colours; not focusable or editable.
    pub fn disabled(mut self, on: bool) -> Self {
        self.disabled = on;
        self
    }

    /// Register, sync the value, and draw.
    pub fn build<'c, C: ContentCx<'c>>(self, cx: &mut C) -> Response {
        let now_ms = cx.time_ms();
        let theme = theme_of(cx);
        let InputBuilder { id, rect, value, placeholder, label, kind, disabled, max_len } = self;
        let (r, ime) = cx.with_paint(|render, widgets, v, _| {
            let mut r = Response::of(v, &id);
            let settings = TextInputSettings::with_config(InputConfig::text()).with_theme(Box::new(theme));
            let (coord, states, layer) = parts(widgets);
            if disabled {
                coord.register_atomic(id.clone(), WidgetKind::Custom, rect, Sense::HOVER, layer);
            } else {
                let mut config = if kind == InputType::Password { uzor::TextFieldConfig::password() } else { uzor::TextFieldConfig::text() };
                if let Some(n) = max_len {
                    config = config.with_max_len(n);
                }
                coord.set_default_layer(Some(layer.clone()));
                coord.register_text_field(id.clone(), rect, config);
                coord.set_default_layer(None);
            }
            coord.set_a11y(&id, label.unwrap_or(placeholder), None);
            // Two-way sync against the value both sides saw last frame.
            let synced = states.get_or_insert_with(WidgetId::from(format!("{}:value", id.as_str())), String::new);
            let fields = coord.text_fields_mut();
            if fields.has_field(&id) {
                let now = fields.text(&id).to_owned();
                if now != *synced {
                    *value = now.clone();
                    *synced = now;
                    r.changed = true;
                } else if *value != *synced {
                    fields.set_text(&id, value);
                    *synced = value.clone();
                }
            }
            let (shown, caret, preedit, engaged, selection) = match fields.field_state(&id) {
                Some(s) => {
                    let (text, caret, preedit) = s.display();
                    let selection = preedit.is_none().then(|| s.selection_range()).flatten();
                    (text, caret, preedit, s.engaged, selection)
                }
                None => (value.clone(), value.chars().count(), None, false, None),
            };
            let shown = if kind == InputType::Password { "\u{2022}".repeat(shown.chars().count()) } else { shown };
            let focused = !disabled && (r.focused || fields.is_focused(&id));
            r.focused = focused;
            let view = InputView {
                text: &shown,
                placeholder,
                cursor: caret,
                selection,
                focused,
                disabled,
                input_type: kind,
            };
            let ws = interaction_state(v, &id, disabled);
            let res = draw_input(render, rect, ws, &view, &settings);
            let tint = settings.theme.cursor();
            let mut ime = None;
            if focused {
                ime = Some(Rect::new(res.cursor_x, res.cursor_y, 1.0, res.cursor_height));
                if engaged && fields.cursor_visible(now_ms) {
                    draw_input_cursor(render, res.cursor_x, res.cursor_y, res.cursor_height, 1.0, tint);
                }
            }
            if let Some((a, b)) = preedit {
                // Composition text is underlined, as the DOM draws it.
                let x = |i: usize| res.char_x_positions.get(i).copied().unwrap_or(res.cursor_x);
                draw_input_cursor(render, x(a), res.cursor_y + res.cursor_height, (x(b) - x(a)).max(0.0), 1.0, tint);
            }
            let fields = coord.text_fields_mut();
            if let Some(s) = fields.field_state(&id) {
                let pos = s.buffer_positions(res.char_x_positions);
                fields.update_field(&id, (rect.x, rect.y, rect.width, rect.height), pos);
            }
            (r, ime)
        });
        if let Some(at) = ime {
            cx.ime_area(at);
        }
        r
    }
}

// -- field ------------------------------------------------------------------

/// A borderless text field inside the app's own chrome. Start with [`field`].
pub struct FieldBuilder<'a> {
    id: WidgetId,
    rect: Rect,
    text_rect: Option<Rect>,
    config: uzor::TextFieldConfig,
    name: &'a str,
    placeholder: &'a str,
    masked: bool,
    clip: bool,
    px: f64,
    family: &'a str,
    color: Option<&'a str>,
    placeholder_color: Option<&'a str>,
    set: Option<&'a str>,
    parent: Option<WidgetId>,
}

/// What a [`field`] shows this frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FieldResponse {
    /// The shown text, composition included (unmasked).
    pub text: String,
    /// Keyboard focus is in the field.
    pub focused: bool,
}

/// A text field whose box the app paints itself (a search pill, a dialog row, a sign-in
/// input): the kit registers it at `rect`, names it, draws text, placeholder, selection,
/// a blinking caret and composition, and feeds caret geometry and the IME area back to the text store. The
/// edit state lives in the store; seed or clear it there before `build`.
pub fn field<'a>(id: impl Into<WidgetId>, rect: Rect) -> FieldBuilder<'a> {
    FieldBuilder {
        id: id.into(),
        rect,
        text_rect: None,
        config: uzor::TextFieldConfig::text(),
        name: "",
        placeholder: "",
        masked: false,
        clip: false,
        px: 13.0,
        family: "sans-serif",
        color: None,
        placeholder_color: None,
        set: None,
        parent: None,
    }
}

fn css_hex(c: [u8; 4]) -> String {
    format!("#{:02x}{:02x}{:02x}{:02x}", c[0], c[1], c[2], c[3])
}

/// Selection fill for a borderless field. `text` is what the painter shows
/// (bullets when masked). Indices are chars of that string. Drawn before
/// the glyphs so they sit on the highlight, same order as `draw_input`.
fn paint_field_selection(render: &mut dyn RenderContext, origin_x: f64, y: f64, h: f64, text: &str, range: (usize, usize), color: &str) {
    let lo = range.0.min(range.1);
    let hi = range.0.max(range.1);
    if lo == hi {
        return;
    }
    let before: String = text.chars().take(lo).collect();
    let mid: String = text.chars().skip(lo).take(hi - lo).collect();
    let x = origin_x + render.measure_text(&before);
    let w = render.measure_text(&mid);
    render.set_fill_color(color);
    render.fill_rect(x, y, w.max(0.0), h);
}

impl<'a> FieldBuilder<'a> {
    /// Editing behaviour (text, search, password, multiline, max length).
    pub fn config(mut self, c: uzor::TextFieldConfig) -> Self {
        self.config = c;
        self
    }
    /// Accessible name (defaults to the placeholder).
    pub fn name(mut self, n: &'a str) -> Self {
        self.name = n;
        self
    }
    /// Grey hint while empty.
    pub fn placeholder(mut self, p: &'a str) -> Self {
        self.placeholder = p;
        self
    }
    /// Draw bullets instead of the text.
    pub fn masked(mut self, on: bool) -> Self {
        self.masked = on;
        self
    }
    /// Where the text runs, when it is inset inside the hit rect.
    pub fn text_rect(mut self, r: Rect) -> Self {
        self.text_rect = Some(r);
        self
    }
    /// Clip the text to its rect.
    pub fn clip(mut self, on: bool) -> Self {
        self.clip = on;
        self
    }
    /// Font size and CSS family.
    pub fn font(mut self, px: f64, family: &'a str) -> Self {
        self.px = px;
        self.family = family;
        self
    }
    /// Text and placeholder colours (CSS); the theme's when unset.
    pub fn colors(mut self, text: &'a str, placeholder: &'a str) -> Self {
        self.color = Some(text);
        self.placeholder_color = Some(placeholder);
        self
    }

    /// Replace the text this frame (`Some("")` clears it, undo included); the
    /// caret goes to the end.
    pub fn set(mut self, text: Option<&'a str>) -> Self {
        self.set = text;
        self
    }
    /// Nest the field under a container (a dialog scrim) for focus scoping.
    pub fn parent(mut self, p: impl Into<WidgetId>) -> Self {
        self.parent = Some(p.into());
        self
    }

    /// Register, draw, and feed the store.
    pub fn build<'c, C: ContentCx<'c>>(self, cx: &mut C) -> FieldResponse {
        use uzor::ui::widgets::atomic::text_input::TextInputTheme;
        use uzor::ui::widgets::composite::dialog::draw_field_text_ime;
        let now_ms = cx.time_ms();
        let theme = theme_of(cx);
        let color = self.color.map_or_else(|| css_hex(theme.text_normal()), str::to_owned);
        let ph_color = self.placeholder_color.map_or_else(|| css_hex(theme.placeholder()), str::to_owned);
        let sel_color = css_hex(theme.selection());
        let FieldBuilder { id, rect, text_rect, config, name, placeholder, masked, clip, px, family, set, parent, .. } = self;
        let fr = text_rect.unwrap_or(rect);
        let (out, ime) = cx.with_paint(|render, widgets, v, _| {
            let (coord, _, layer) = parts(widgets);
            coord.set_default_layer(Some(layer.clone()));
            coord.register_text_field(id.clone(), rect, config);
            if let Some(p) = &parent {
                coord.set_widget_parent(&id, p);
            }
            coord.set_a11y(&id, if name.is_empty() { placeholder } else { name }, None);
            coord.set_default_layer(None);
            if let Some(t) = set {
                let fields = coord.text_fields_mut();
                fields.clear(&id);
                if !t.is_empty() {
                    fields.set_text(&id, t);
                }
            }
            let focused = v.is_focused(&id);
            let (text, caret, preedit, engaged, selection) = {
                let fields = coord.text_fields();
                match fields.field_state(&id) {
                    Some(s) => {
                        let (text, caret, preedit) = s.display();
                        let selection = preedit.is_none().then(|| s.selection_range()).flatten();
                        (text, caret, preedit, s.engaged, selection)
                    }
                    None => (String::new(), 0, None, false, None),
                }
            };
            let shown = if masked { "\u{2022}".repeat(text.chars().count()) } else { text.clone() };
            let blink = focused && engaged && coord.text_fields().cursor_visible(now_ms);
            if clip {
                render.save();
                render.begin_path();
                render.rect(fr.x, fr.y, fr.width, fr.height);
                render.clip();
            }
            if let Some(range) = selection {
                render.set_font(&format!("400 {px}px {family}"));
                paint_field_selection(render, fr.x, fr.y, fr.height, &shown, range, &sel_color);
            }
            let pos = draw_field_text_ime(render, fr, &shown, placeholder, blink.then_some(caret), preedit, px, &color, &ph_color, family);
            if clip {
                render.restore();
            }
            let ime = (focused && caret < pos.len()).then(|| Rect::new(pos[caret], fr.y, 1.0, fr.height));
            let fields = coord.text_fields_mut();
            let pos = match fields.field_state(&id) {
                Some(s) => s.buffer_positions(pos),
                None => pos,
            };
            fields.update_field(&id, (fr.x, fr.y, fr.width, fr.height), pos);
            (FieldResponse { text, focused }, ime)
        });
        if let Some(at) = ime {
            cx.ime_area(at);
        }
        out
    }
}

// -- tabs -------------------------------------------------------------------

/// A row of equal-width tabs. Start with [`tabs`].
pub struct TabsBuilder<'a> {
    id: WidgetId,
    rect: Rect,
    labels: &'a [&'a str],
    selected: usize,
    bind: Option<&'a mut usize>,
}

/// Tabs labelled `labels`, laid out across `rect`. Parts are `"{id}:tab:{i}"`.
pub fn tabs<'a>(id: impl Into<WidgetId>, rect: Rect, labels: &'a [&'a str]) -> TabsBuilder<'a> {
    TabsBuilder { id: id.into(), rect, labels, selected: 0, bind: None }
}

impl<'a> TabsBuilder<'a> {
    /// The active tab.
    pub fn selected(mut self, i: usize) -> Self {
        self.selected = i;
        self
    }
    /// Read the active tab from `i` and write a click back to it.
    pub fn bind(mut self, i: &'a mut usize) -> Self {
        self.selected = *i;
        self.bind = Some(i);
        self
    }

    /// Register and draw.
    pub fn build<'c, C: ContentCx<'c>>(self, cx: &mut C) -> Response {
        let theme = theme_of(cx);
        let n = self.labels.len().max(1) as f64;
        let w = self.rect.width / n;
        let mut r = cx.with_paint(|render, widgets, v, _| {
            let mut r = Response::of(v, &self.id);
            r.index = picked(v, &self.id, "tab");
            let settings = TabSettings { theme: Box::new(theme), ..Default::default() };
            let (coord, _, layer) = parts(widgets);
            let active = r.index.unwrap_or(self.selected);
            for (i, label) in self.labels.iter().enumerate() {
                let tid = child(&self.id, "tab", i);
                let tr = Rect::new(self.rect.x + w * i as f64, self.rect.y, w, self.rect.height);
                register_tab_on_layer(coord, tid.clone(), tr, focusable(), None, layer);
                coord.set_a11y(&tid, *label, Some(i == active));
                r.hovered |= v.is_hovered(&tid);
                r.focused |= v.is_focused(&tid);
                let mut cfg = TabConfig::new(tid.as_str(), *label);
                cfg.active = i == active;
                let view = TabView { tab: &cfg, hovered: v.is_hovered(&tid), pressed: v.is_pressed(&tid), close_btn_hovered: false };
                draw_tab(render, tr, &view, &settings);
            }
            r
        });
        if let Some(i) = r.index {
            r.changed = i != self.selected;
            if let Some(b) = self.bind {
                *b = i;
            }
        }
        r
    }
}

// -- list -------------------------------------------------------------------

/// A vertical list of selectable text rows. Start with [`list`].
pub struct ListBuilder<'a> {
    id: WidgetId,
    rect: Rect,
    rows: &'a [&'a str],
    row_height: f64,
    scroll: f64,
    selected: Option<usize>,
    bind: Option<&'a mut Option<usize>>,
}

/// Rows `rows` inside `rect`. Parts are `"{id}:row:{i}"`. Rows outside
/// `rect` (after `.scroll`) are neither painted nor hit.
pub fn list<'a>(id: impl Into<WidgetId>, rect: Rect, rows: &'a [&'a str]) -> ListBuilder<'a> {
    ListBuilder { id: id.into(), rect, rows, row_height: 28.0, scroll: 0.0, selected: None, bind: None }
}

impl<'a> ListBuilder<'a> {
    /// Row height in px (default 28).
    pub fn row_height(mut self, h: f64) -> Self {
        self.row_height = h;
        self
    }
    /// Scroll offset in px from the top.
    pub fn scroll(mut self, y: f64) -> Self {
        self.scroll = y;
        self
    }
    /// The highlighted row.
    pub fn selected(mut self, i: Option<usize>) -> Self {
        self.selected = i;
        self
    }
    /// Read the selection from `i` and write a click back to it.
    pub fn bind(mut self, i: &'a mut Option<usize>) -> Self {
        self.selected = *i;
        self.bind = Some(i);
        self
    }
    /// Total content height, for a scrollbar.
    pub fn content_height(&self) -> f64 {
        self.rows.len() as f64 * self.row_height
    }

    /// Register and draw.
    pub fn build<'c, C: ContentCx<'c>>(self, cx: &mut C) -> Response {
        let theme = theme_of(cx);
        let mut r = cx.with_paint(|render, widgets, v, _| {
            let mut r = Response::of(v, &self.id);
            r.index = picked(v, &self.id, "row");
            let settings = ItemSettings::default().with_theme(Box::new(theme));
            let (coord, _, layer) = parts(widgets);
            let sel = r.index.or(self.selected);
            let (top, bottom) = (self.rect.y, self.rect.y + self.rect.height);
            for (i, row) in self.rows.iter().enumerate() {
                let y = top + i as f64 * self.row_height - self.scroll;
                if y < top || y + self.row_height > bottom {
                    continue;
                }
                let rid = child(&self.id, "row", i);
                let rr = Rect::new(self.rect.x, y, self.rect.width, self.row_height);
                coord.register_atomic(rid.clone(), WidgetKind::Item, rr, focusable(), layer);
                coord.set_a11y(&rid, *row, Some(sel == Some(i)));
                r.hovered |= v.is_hovered(&rid);
                r.focused |= v.is_focused(&rid);
                let ws = if sel == Some(i) { WidgetState::Active } else { interaction_state(v, &rid, false) };
                let view = ItemView { label: Some(row), icon: None, svg: None };
                draw_item(render, rr, ws, &view, &settings, &ItemRenderKind::Label, |_, _, _, _| {}, |_, _, _, _| {});
            }
            r
        });
        if let Some(i) = r.index {
            r.changed = Some(i) != self.selected;
            if let Some(b) = self.bind {
                *b = Some(i);
            }
        }
        r
    }
}

// -- table ------------------------------------------------------------------

/// A table with a header row and selectable body rows. Start with [`table`].
pub struct TableBuilder<'a> {
    id: WidgetId,
    rect: Rect,
    columns: &'a [&'a str],
    rows: &'a [Vec<String>],
    tracks: Option<&'a str>,
    row_height: f64,
    scroll: f64,
    selected: Option<usize>,
    bind: Option<&'a mut Option<usize>>,
    font: &'a str,
}

/// A table of `rows` under `columns` in `rect`. Parts are `"{id}:col:{j}"`
/// (header cells, a click reports `column`) and `"{id}:row:{i}"` (rows, a
/// click reports `index`). Rows outside `rect` after `.scroll` are skipped.
pub fn table<'a>(id: impl Into<WidgetId>, rect: Rect, columns: &'a [&'a str], rows: &'a [Vec<String>]) -> TableBuilder<'a> {
    TableBuilder {
        id: id.into(),
        rect,
        columns,
        rows,
        tracks: None,
        row_height: 28.0,
        scroll: 0.0,
        selected: None,
        bind: None,
        font: "13px sans-serif",
    }
}

impl<'a> TableBuilder<'a> {
    /// Column widths as CSS grid tracks (`"2fr 1fr 80px"`, `minmax()`); default equal `1fr`.
    pub fn columns(mut self, tracks: &'a str) -> Self {
        self.tracks = Some(tracks);
        self
    }
    /// Header and row height in px (default 28).
    pub fn row_height(mut self, h: f64) -> Self {
        self.row_height = h;
        self
    }
    /// Body scroll offset in px (the header stays put, like `position: sticky`).
    pub fn scroll(mut self, y: f64) -> Self {
        self.scroll = y;
        self
    }
    /// The highlighted row.
    pub fn selected(mut self, i: Option<usize>) -> Self {
        self.selected = i;
        self
    }
    /// Read the selection from `i` and write a row click back to it.
    pub fn bind(mut self, i: &'a mut Option<usize>) -> Self {
        self.selected = *i;
        self.bind = Some(i);
        self
    }
    /// Cell font shorthand.
    pub fn font(mut self, f: &'a str) -> Self {
        self.font = f;
        self
    }
    /// Body content height, for a scrollbar.
    pub fn content_height(&self) -> f64 {
        self.rows.len() as f64 * self.row_height
    }

    /// Register and draw.
    pub fn build<'c, C: ContentCx<'c>>(self, cx: &mut C) -> Response {
        use uzor::ui::widgets::atomic::text::theme::TextTheme;
        use uzor::ui::widgets::composite::panel::theme::PanelTheme;
        let theme = theme_of(cx);
        let Self { id, rect, columns, rows, tracks, row_height, scroll, selected, bind, font } = self;
        let mut r = cx.with_paint(|render, widgets, v, _| {
            let mut r = Response::of(v, &id);
            r.index = picked(v, &id, "row");
            r.column = picked(v, &id, "col");
            let (coord, _, layer) = parts(widgets);
            let even = vec!["1fr"; columns.len().max(1)].join(" ");
            let spec = tracks.unwrap_or(&even);
            let cols = uzor::ui::flow::grid_tracks(rect.width, &uzor::ui::flow::parse_tracks(spec, rect.width, 0.0), 0.0);
            let sel = r.index.or(selected);
            render.set_font(font);
            render.set_text_align(uzor::render::TextAlign::Left);
            render.set_text_baseline(uzor::render::TextBaseline::Middle);
            let cells = |render: &mut dyn RenderContext, y: f64, texts: &mut dyn Iterator<Item = &str>, color: &str| {
                render.set_fill_color(color);
                for ((x, w), t) in cols.iter().zip(texts) {
                    render.save();
                    render.clip_rect(rect.x + x, y, *w, row_height);
                    render.fill_text(t, rect.x + x + 8.0, y + row_height / 2.0);
                    render.restore();
                }
            };
            // Header.
            render.set_fill_color(theme.column_header_bg());
            render.fill_rect(rect.x, rect.y, rect.width, row_height);
            for (j, (x, w)) in cols.iter().enumerate().take(columns.len()) {
                let cid = child(&id, "col", j);
                coord.register_atomic(cid.clone(), WidgetKind::Item, Rect::new(rect.x + x, rect.y, *w, row_height), focusable(), layer);
                coord.set_a11y(&cid, columns[j], None);
                r.hovered |= v.is_hovered(&cid);
                r.focused |= v.is_focused(&cid);
            }
            cells(render, rect.y, &mut columns.iter().copied(), theme.column_header_text());
            // Body.
            let (top, bottom) = (rect.y + row_height, rect.y + rect.height);
            for (i, row) in rows.iter().enumerate() {
                let y = top + i as f64 * row_height - scroll;
                if y < top || y + row_height > bottom {
                    continue;
                }
                let rid = child(&id, "row", i);
                let rr = Rect::new(rect.x, y, rect.width, row_height);
                coord.register_atomic(rid.clone(), WidgetKind::Item, rr, focusable(), layer);
                coord.set_a11y(&rid, &row.join(", "), Some(sel == Some(i)));
                let hov = v.is_hovered(&rid);
                r.hovered |= hov;
                r.focused |= v.is_focused(&rid);
                let bg = if sel == Some(i) { theme.row_bg_selected() } else if hov { theme.row_bg_hover() } else { theme.row_bg_normal() };
                render.set_fill_color(bg);
                render.fill_rect(rr.x, rr.y, rr.width, rr.height);
                render.set_fill_color(theme.divider());
                render.fill_rect(rr.x, rr.y + rr.height - 1.0, rr.width, 1.0);
                cells(render, y, &mut row.iter().map(String::as_str), theme.text_color());
            }
            r
        });
        if let Some(i) = r.index {
            r.changed = Some(i) != selected;
            if let Some(b) = bind {
                *b = Some(i);
            }
        }
        r
    }
}

// -- drawer -----------------------------------------------------------------

/// The edge a drawer slides from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Side {
    /// From the left edge.
    Left,
    /// From the right edge (default).
    #[default]
    Right,
}

/// A side panel over the content with a scrim. Start with [`drawer`].
pub struct DrawerBuilder<'a> {
    id: WidgetId,
    area: Rect,
    open: &'a mut bool,
    side: Side,
    width: f64,
    title: Option<&'a str>,
    progress: f64,
    dim: bool,
}

/// A drawer over `area`, shown while `*open`. A click on the scrim
/// (`"{id}:scrim"`) clears `open` and reports `changed`. Paint the body in
/// [`DrawerBuilder::body_rect`] after `build`.
pub fn drawer<'a>(id: impl Into<WidgetId>, area: Rect, open: &'a mut bool) -> DrawerBuilder<'a> {
    DrawerBuilder { id: id.into(), area, open, side: Side::Right, width: 320.0, title: None, progress: 1.0, dim: true }
}

impl<'a> DrawerBuilder<'a> {
    /// Edge it slides from.
    pub fn side(mut self, s: Side) -> Self {
        self.side = s;
        self
    }
    /// Panel width in px (default 320, capped to the area).
    pub fn width(mut self, w: f64) -> Self {
        self.width = w;
        self
    }
    /// Accessible name.
    pub fn title(mut self, t: &'a str) -> Self {
        self.title = Some(t);
        self
    }
    /// Slide position, 0 (hidden) to 1 (shown): drive it from a
    /// [`Motion`](uzor::ui::motion::Motion) value or a timer for an animated open.
    pub fn progress(mut self, t: f64) -> Self {
        self.progress = t.clamp(0.0, 1.0);
        self
    }
    /// Dim the content behind (default on).
    pub fn dim(mut self, on: bool) -> Self {
        self.dim = on;
        self
    }
    /// The panel's rect at the current progress.
    pub fn body_rect(&self) -> Rect {
        let w = self.width.min(self.area.width);
        let shown = w * self.progress;
        let x = match self.side {
            Side::Left => self.area.x - w + shown,
            Side::Right => self.area.x + self.area.width - shown,
        };
        Rect::new(x, self.area.y, w, self.area.height)
    }

    /// Register and draw the scrim and the panel frame.
    pub fn build<'c, C: ContentCx<'c>>(self, cx: &mut C) -> Response {
        use uzor::ui::widgets::composite::modal::theme::ModalTheme;
        use uzor::ui::widgets::composite::panel::theme::PanelTheme;
        if !*self.open {
            return Response::default();
        }
        let theme = theme_of(cx);
        let body = self.body_rect();
        let scrim = WidgetId::from(format!("{}:scrim", self.id.as_str()));
        let r = cx.with_paint(|render, widgets, v, _| {
            let (coord, _, layer) = parts(widgets);
            coord.register_atomic(scrim.clone(), WidgetKind::Custom, self.area, Sense::CLICK, layer);
            coord.register_atomic(self.id.clone(), WidgetKind::Custom, body, Sense::CLICK | Sense::HOVER, layer);
            coord.set_a11y(&self.id, self.title.unwrap_or(""), None);
            if self.dim {
                render.set_global_alpha(self.progress);
                render.set_fill_color(ModalTheme::backdrop_dim(&theme));
                render.fill_rect(self.area.x, self.area.y, self.area.width, self.area.height);
                render.set_global_alpha(1.0);
            }
            render.set_fill_color(PanelTheme::bg(&theme));
            render.fill_rect(body.x, body.y, body.width, body.height);
            let edge = if self.side == Side::Right { body.x } else { body.x + body.width - 1.0 };
            render.set_fill_color(PanelTheme::border(&theme));
            render.fill_rect(edge, body.y, 1.0, body.height);
            let mut r = Response::of(v, &self.id);
            r.clicked = v.clicked(&scrim);
            r
        });
        if r.clicked {
            *self.open = false;
            return Response { changed: true, ..r };
        }
        r
    }
}

// -- select -----------------------------------------------------------------

#[derive(Default)]
struct SelectOpen {
    open: bool,
    list: SelectState,
}

/// A button that opens a list of options. Start with [`select`].
pub struct SelectBuilder<'a> {
    id: WidgetId,
    rect: Rect,
    options: &'a [&'a str],
    selected: usize,
    bind: Option<&'a mut usize>,
    disabled: bool,
    searchable: Option<bool>,
    within: Option<Rect>,
    style: Option<SelectStyle>,
    key: Option<&'a KeyInput>,
}

/// A select showing `options[selected]`. The trigger is `id`; options are
/// `"{id}:list:item:{i}"` (`i` = index into `options`). The open state (flag,
/// highlighted row, search text, scroll) lives in the widget state store.
///
/// The open list behaves like a native one ([`SelectState`]): forward this
/// frame's key with [`.key`](SelectBuilder::key) and arrows / Page / Home /
/// End move, Enter picks, Escape closes, typing filters a searchable list;
/// Enter, Space or ArrowDown on the focused trigger opens it. The list opens
/// under the trigger and stays inside [`.within`](SelectBuilder::within)
/// (the dialog that hosts it), flipping above or shifting as needed.
pub fn select<'a>(id: impl Into<WidgetId>, rect: Rect, options: &'a [&'a str]) -> SelectBuilder<'a> {
    SelectBuilder { id: id.into(), rect, options, selected: 0, bind: None, disabled: false, searchable: None, within: None, style: None, key: None }
}

/// Whether the select `id` is open (its list shows). An app that closes its
/// dialog on Escape asks this first: Escape belongs to the open list.
pub fn select_is_open<'c, C: ContentCx<'c>>(cx: &mut C, id: &str) -> bool {
    cx.with_paint(|_, widgets, _, _| widgets.states.get_or_insert_with(WidgetId::from(format!("{id}:open")), SelectOpen::default).open)
}

impl<'a> SelectBuilder<'a> {
    /// The chosen option.
    pub fn selected(mut self, i: usize) -> Self {
        self.selected = i;
        self
    }
    /// Read the choice from `i` and write a pick back to it.
    pub fn bind(mut self, i: &'a mut usize) -> Self {
        self.selected = *i;
        self.bind = Some(i);
        self
    }
    /// Disabled colours; does not open.
    pub fn disabled(mut self, on: bool) -> Self {
        self.disabled = on;
        self
    }
    /// A search line on top of the list (default: more options than the list shows).
    pub fn searchable(mut self, on: bool) -> Self {
        self.searchable = Some(on);
        self
    }
    /// Keep the open list inside `container` (a modal card, a panel).
    pub fn within(mut self, container: Rect) -> Self {
        self.within = Some(container);
        self
    }
    /// The list's look (e.g. `SelectStyle::from_sheet(&sheet, "div.select-list")`).
    pub fn style(mut self, s: SelectStyle) -> Self {
        self.style = Some(s);
        self
    }
    /// This frame's key press, if any (the app forwards unhandled keys).
    pub fn key(mut self, k: Option<&'a KeyInput>) -> Self {
        self.key = k.filter(|k| k.state != crate::KeyState::Up);
        self
    }

    /// Register and draw (the open list paints after the trigger). `Response::checked`
    /// tells whether the list is open after this frame; `Response::pressed` that the key
    /// given to [`.key`](Self::key) was used.
    pub fn build<'c, C: ContentCx<'c>>(self, cx: &mut C) -> Response {
        let theme = theme_of(cx);
        let ls = self.style.unwrap_or_default();
        let searchable = self.searchable.unwrap_or(self.options.len() > ls.window());
        let ls = match self.within {
            Some(w) => ls.fit(room(self.rect, w, ls.gap), searchable),
            None => ls,
        };
        let win = ls.window();
        let labels: Vec<(&str, &str)> = self.options.iter().map(|o| (*o, *o)).collect();
        let mut r = cx.with_paint(|render, widgets, v, tokens| {
            let mut r = Response::of(v, &self.id);
            let sid = WidgetId::from(format!("{}:open", self.id.as_str()));
            let mut key_used = false;
            let mut close = false;
            let (open, list) = {
                let st = widgets.states.get_or_insert_with(sid, SelectOpen::default);
                r.index = picked(v, &self.id, "list:item");
                let was = st.open;
                if self.disabled || r.index.is_some() || (st.open && clicked_outside(v, &self.id)) {
                    st.open = false;
                } else if r.clicked {
                    st.open = !st.open;
                } else if let Some(k) = self.key {
                    use uzor::input::keyboard::KeyCode as K;
                    if st.open {
                        let rows = filter(&labels, &st.list.query);
                        match st.list.key(k.code, k.text.as_deref(), k.mods.ctrl || k.mods.meta, rows.len(), searchable, win) {
                            SelectKey::Changed => key_used = true,
                            SelectKey::Pick(i) => {
                                r.index = rows.get(i).copied();
                                st.open = false;
                                key_used = true;
                            }
                            SelectKey::Close => {
                                close = true;
                                key_used = true;
                            }
                            SelectKey::Ignored => {}
                        }
                    } else if r.focused && matches!(k.code, K::Enter | K::Space | K::ArrowDown) {
                        st.open = true;
                        key_used = true;
                    }
                }
                if close {
                    st.open = false;
                }
                if st.open && !was {
                    st.list = SelectState::open_at(Some(self.selected), win);
                }
                (st.open, st.list.clone())
            };
            r.pressed = key_used;
            let label = self.options.get(r.index.unwrap_or(self.selected)).copied().unwrap_or("");
            let view = ButtonView {
                icon: None,
                text: Some(label),
                active: open,
                disabled: self.disabled,
                active_border: None,
                hover_chevron: None,
            };
            let mut bs = button_settings(tokens);
            bs.theme = Box::new(theme.clone());
            paint_button(render, widgets, v, self.id.clone(), self.rect, &view, &bs);
            widgets.coord.set_a11y(&self.id, label, None);
            if open {
                let rows = filter(&labels, &list.query);
                let (w, h) = ls.list_size(rows.len(), searchable, self.rect.width);
                let within = self.within.unwrap_or(Rect::new(-1.0e6, -1.0e6, 2.0e6, 2.0e6));
                let b = place(self.rect, w, h, within, ls.gap);
                let shown: Vec<(&str, bool)> = rows.iter().map(|&i| (self.options[i], i == self.selected)).collect();
                let pre = format!("{}:list:item:", self.id.as_str());
                let hovered: Option<usize> = v.hovered().and_then(|w| w.as_str().strip_prefix(pre.as_str())?.parse().ok());
                let hover = |k: usize| hovered.is_some() && rows.get(k).copied() == hovered;
                let painted = draw_list(render, b, &shown, &list, searchable.then_some(list.query.as_str()), &hover, &ls);
                let (coord, _, layer) = parts(widgets);
                coord.register_atomic(WidgetId::from(format!("{}:list", self.id.as_str())), WidgetKind::Custom, b, Sense::CLICK | Sense::HOVER, layer);
                for (k, rr) in painted {
                    let oi = rows[k];
                    let rid = child(&self.id, "list:item", oi);
                    coord.register_atomic(rid.clone(), WidgetKind::Item, rr, Sense::CLICK | Sense::HOVER, layer);
                    coord.set_a11y(&rid, self.options[oi], Some(oi == self.selected));
                }
            }
            r.checked = open;
            r
        });
        if let Some(i) = r.index {
            r.changed = i != self.selected;
            if let Some(b) = self.bind {
                *b = i;
            }
        }
        r
    }
}

// -- menu -------------------------------------------------------------------

/// A context menu. Start with [`menu`].
pub struct MenuBuilder<'a> {
    id: WidgetId,
    at: (f64, f64),
    items: &'a [&'a str],
    open: &'a mut bool,
    title: Option<&'a str>,
    danger: Option<usize>,
}

/// A menu of `items` at `at` while `*open`. Items are `"{id}:item:{i}"`.
/// A pick or a click elsewhere clears `*open`.
pub fn menu<'a>(id: impl Into<WidgetId>, at: (f64, f64), items: &'a [&'a str], open: &'a mut bool) -> MenuBuilder<'a> {
    MenuBuilder { id: id.into(), at, items, open, title: None, danger: None }
}

impl<'a> MenuBuilder<'a> {
    /// Header line.
    pub fn title(mut self, t: &'a str) -> Self {
        self.title = Some(t);
        self
    }
    /// Draw item `i` in the danger colour (delete, sign out).
    pub fn danger(mut self, i: usize) -> Self {
        self.danger = Some(i);
        self
    }

    /// Register and draw when open.
    pub fn build<'c, C: ContentCx<'c>>(self, cx: &mut C) -> Response {
        if !*self.open {
            return Response::default();
        }
        let theme = theme_of(cx);
        let r = cx.with_paint(|render, widgets, v, _| {
            let mut r = Response::of(v, &self.id);
            r.index = picked(v, &self.id, "item");
            if r.index.is_some() || clicked_outside(v, &self.id) {
                return (r, false);
            }
            let items: Vec<ContextMenuItem<'_>> = self
                .items
                .iter()
                .enumerate()
                .map(|(i, l)| ContextMenuItem { action: l, label: l, icon: None, danger: self.danger == Some(i), separator_after: false, enabled: true })
                .collect();
            let state = ContextMenuState { is_open: true, x: self.at.0, y: self.at.1, ..Default::default() };
            let view = ContextMenuView { items: &items, target_id: None, title: self.title };
            let settings = ContextMenuSettings { theme: Box::new(theme), ..Default::default() };
            let (coord, _, layer) = parts(widgets);
            register_input_coordinator_context_menu(coord, self.id.clone(), &state, &view, &settings, &ContextMenuRenderKind::Default, layer);
            for (i, l) in self.items.iter().enumerate() {
                coord.set_a11y(&child(&self.id, "item", i), *l, None);
            }
            draw_context_menu(render, &state, &view, &settings, &ContextMenuRenderKind::Default);
            (r, true)
        });
        *self.open = r.1;
        let mut r = r.0;
        r.changed = r.index.is_some();
        r
    }
}

// -- dialog -----------------------------------------------------------------

/// A modal frame with a title, close button and footer. Start with [`dialog`].
pub struct DialogBuilder<'a> {
    id: WidgetId,
    rect: Rect,
    title: Option<&'a str>,
    footer: &'a [&'a str],
    dim: bool,
}

/// A dialog in `rect`. Close is `"{id}:close"` (reported as `clicked`),
/// footer buttons `"{id}:footer:{i}"` (reported as `index`; the first is
/// primary). The app decides whether either closes it.
pub fn dialog<'a>(id: impl Into<WidgetId>, rect: Rect) -> DialogBuilder<'a> {
    DialogBuilder { id: id.into(), rect, title: None, footer: &[], dim: true }
}

impl<'a> DialogBuilder<'a> {
    /// Header title, also the accessible name.
    pub fn title(mut self, t: &'a str) -> Self {
        self.title = Some(t);
        self
    }
    /// Footer button labels, primary first.
    pub fn footer(mut self, labels: &'a [&'a str]) -> Self {
        self.footer = labels;
        self
    }
    /// Dim what is behind (default on).
    pub fn dim(mut self, on: bool) -> Self {
        self.dim = on;
        self
    }

    /// Register and draw the frame. Paint the body after this, inside the
    /// returned content area of `rect`.
    pub fn build<'c, C: ContentCx<'c>>(self, cx: &mut C) -> Response {
        let theme = theme_of(cx);
        cx.with_paint(|render, widgets, v, _| {
            let mut r = Response::of(v, &self.id);
            r.clicked = v.clicked(&WidgetId::from(format!("{}:close", self.id.as_str())));
            r.index = picked(v, &self.id, "footer");
            r.changed = r.clicked || r.index.is_some();
            let buttons: Vec<FooterBtn<'_>> = self
                .footer
                .iter()
                .enumerate()
                .map(|(i, l)| FooterBtn { label: l, style: if i == 0 { FooterBtnStyle::Primary } else { FooterBtnStyle::Ghost } })
                .collect();
            let mut view = ModalView {
                title: self.title,
                tabs: &[],
                footer_buttons: &buttons,
                wizard_pages: &[],
                backdrop: if self.dim { BackdropKind::Dim } else { BackdropKind::None },
                overflow: Default::default(),
                resizable: false,
            };
            let kind = if buttons.is_empty() { ModalRenderKind::WithHeader } else { ModalRenderKind::WithHeaderFooter };
            let settings = ModalSettings { theme: Box::new(theme), ..Default::default() };
            let (coord, states, layer) = parts(widgets);
            let state = states.get_or_insert_with(self.id.clone(), ModalState::default);
            register_input_coordinator_modal(coord, self.id.clone(), self.rect, state, &mut view, &settings, &kind, layer);
            if let Some(t) = self.title {
                coord.set_a11y(&self.id, t, None);
            }
            for (i, l) in self.footer.iter().enumerate() {
                coord.set_a11y(&child(&self.id, "footer", i), *l, None);
            }
            coord.set_a11y(&WidgetId::from(format!("{}:close", self.id.as_str())), "Close", None);
            draw_modal(render, self.rect, state, &mut view, &settings, &kind);
            r
        })
    }
}

// -- splitter ---------------------------------------------------------------

/// A draggable pane edge. Start with [`splitter`].
pub struct SplitterBuilder<'a> {
    id: WidgetId,
    axis: Axis,
    span: Rect,
    pos: &'a mut f64,
    min: f64,
    max: f64,
    grab: f64,
}

/// An edge at `*pos` along `axis` across `span`. Dragging writes `*pos`.
pub fn splitter<'a>(id: impl Into<WidgetId>, axis: Axis, span: Rect, pos: &'a mut f64) -> SplitterBuilder<'a> {
    SplitterBuilder { id: id.into(), axis, span, pos, min: f64::MIN, max: f64::MAX, grab: 9.0 }
}

impl<'a> SplitterBuilder<'a> {
    /// Clamp the edge to `min..=max`.
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self
    }
    /// Width of the grab strip in px (default 9).
    pub fn grab(mut self, px: f64) -> Self {
        self.grab = px;
        self
    }

    /// Register and draw. `changed` is set on every drag step; `pressed`
    /// while dragging. Persist the size when the drag ends (`pressed` falls).
    pub fn build<'c, C: ContentCx<'c>>(self, cx: &mut C) -> Response {
        let theme = theme_of(cx);
        cx.with_paint(|render, widgets, v, _| {
            let mut r = Response::of(v, &self.id);
            let (coord, states, layer) = parts(widgets);
            let s = states.get_or_insert_with(self.id.clone(), Splitter::default);
            if let SplitterEvent::Drag(p) = s.update(v, &self.id, self.axis) {
                let p = p.clamp(self.min, self.max);
                r.changed = (p - *self.pos).abs() > f64::EPSILON;
                *self.pos = p;
            }
            r.pressed = s.dragging();
            let hot = s.hot(v, &self.id);
            register_splitter(coord, self.id.clone(), grab_rect(self.axis, *self.pos, self.span, self.grab), layer);
            let (line, orientation) = match self.axis {
                Axis::X => (Rect::new(*self.pos, self.span.y, 1.0, self.span.height), SeparatorOrientation::Vertical),
                Axis::Y => (Rect::new(self.span.x, *self.pos, self.span.width, 1.0), SeparatorOrientation::Horizontal),
            };
            let view = SeparatorView { kind: SeparatorType::Divider { orientation }, hovered: hot, dragging: r.pressed };
            let settings = SeparatorSettings { theme: Box::new(theme), style: Box::new(DefaultSeparatorStyle) };
            draw_separator(render, line, &view, &settings);
            r
        })
    }
}

// -- toast ------------------------------------------------------------------

/// The toast stack. Start with [`toasts`].
pub struct ToastsBuilder<'a> {
    id: WidgetId,
    area: Rect,
    stack: &'a mut ToastStackState,
    now_ms: u64,
}

/// Paint the live toasts of `stack` in the bottom-right of `area`, dropping
/// expired ones. Push new ones with [`ToastStackState::push`].
pub fn toasts<'a>(id: impl Into<WidgetId>, area: Rect, stack: &'a mut ToastStackState, now_ms: u64) -> ToastsBuilder<'a> {
    ToastsBuilder { id: id.into(), area, stack, now_ms }
}

impl<'a> ToastsBuilder<'a> {
    /// Draw. `changed` is true while any toast is shown, so the app keeps
    /// asking for frames until the last one fades.
    pub fn build<'c, C: ContentCx<'c>>(self, cx: &mut C) -> Response {
        let theme = theme_of(cx);
        let live = self.stack.tick(self.now_ms);
        cx.with_paint(|render, _widgets, v, _| {
            let mut r = Response::of(v, &self.id);
            r.changed = !live.is_empty();
            if r.changed {
                render.save();
                render.translate(self.area.x, self.area.y);
                draw_toast_stack(render, live, &theme, self.area.width, self.area.height, self.now_ms);
                render.restore();
            }
            r
        })
    }
}

#[cfg(test)]
#[path = "kit_tests.rs"]
mod tests;

// -- DOM trees ----------------------------------------------------------------

/// A DOM tree ([`uzor::ui::dom`]) inside a kit panel. Start with [`dom`].
pub struct DomBuilder<'a> {
    rect: Rect,
    sheet: &'a uzor::ui::style::Sheet,
    root: uzor::ui::dom::El,
    scroll: f64,
    viewport: Option<Rect>,
}

/// What a [`dom`] tree did this frame.
#[derive(Clone, Debug, Default)]
pub struct DomResponse {
    /// The laid-out tree (rects by id, embed slots, a11y nodes).
    pub dom: uzor::ui::dom::Dom,
    /// The element id clicked this frame (topmost first).
    pub clicked: Option<String>,
    /// The element id under the pointer.
    pub hovered: Option<String>,
    /// The focused element id.
    pub focused: Option<String>,
    /// Text inputs whose value the user changed this frame: `(id, new value)`.
    pub edited: Vec<(String, String)>,
}

/// Lay out `root` with `sheet` in `rect` (scrolled up by [`DomBuilder::scroll`]),
/// paint it, and register every element with an id: buttons, switches, checkboxes and
/// rows become focusable click targets named by their accessible name ([`El::name`]
/// first, then text) with their checked state; DOM text inputs
/// ([`uzor::ui::dom::Control::Input`]) become text-store fields, so caret, selection,
/// drag, clipboard and IME work as in [`field`]. `position: fixed` elements (a select
/// popover, a side panel) are placed against the panel and stay put while it scrolls.
///
/// Kit widgets go *inside* the tree through embed slots: call them at
/// `resp.dom.slots()` after `build`.
///
/// [`El::name`]: uzor::ui::dom::El::name
pub fn dom<'a>(rect: Rect, sheet: &'a uzor::ui::style::Sheet, root: uzor::ui::dom::El) -> DomBuilder<'a> {
    DomBuilder { rect, sheet, root, scroll: 0.0, viewport: None }
}

impl<'a> DomBuilder<'a> {
    /// Scroll the tree up by `y` (fixed elements do not move).
    pub fn scroll(mut self, y: f64) -> Self {
        self.scroll = y;
        self
    }
    /// The viewport fixed elements are placed against (the panel rect by default).
    pub fn viewport(mut self, r: Rect) -> Self {
        self.viewport = Some(r);
        self
    }

    /// Paint the tree and register its hit targets and fields.
    pub fn build<'c, C: ContentCx<'c>>(self, cx: &mut C) -> DomResponse {
        use uzor::ui::dom::{Control, El};
        use uzor::ui::style::State;
        let now_ms = cx.time_ms();
        let DomBuilder { rect, sheet, mut root, scroll, viewport } = self;
        let (resp, ime) = cx.with_paint(|render, widgets, v, _| {
            let (coord, _, layer) = parts(widgets);
            let mut edited = Vec::new();
            let mut seed: Vec<(WidgetId, String)> = Vec::new();
            // Feed the text store's state into the tree's inputs (seeding new fields).
            fn walk(e: &mut El, f: &mut dyn FnMut(&str, &mut uzor::ui::dom::Edit)) {
                if let (Some(id), Some(Control::Input(ed))) = (e.id.as_deref(), e.control.as_mut()) {
                    f(id, ed);
                }
                for c in &mut e.children {
                    walk(c, f);
                }
            }
            walk(&mut root, &mut |id, ed| {
                let wid = WidgetId::from(id);
                if coord.text_fields().field_state(&wid).is_none() {
                    seed.push((wid.clone(), ed.value.clone()));
                }
                let focused = v.is_focused(&wid);
                if let Some(s) = coord.text_fields().field_state(&wid) {
                    let text = s.text.clone();
                    if text != ed.value {
                        edited.push((id.to_string(), text.clone()));
                        ed.value = text;
                    }
                    let caret = s.cursor.min(ed.value.chars().count());
                    let composing = !s.preedit.is_empty();
                    let sel = (!composing).then(|| s.selection_range()).flatten();
                    ed.caret = focused.then_some(caret);
                    ed.anchor = sel.filter(|_| focused).map(|(a, b)| if b == caret { a } else { b });
                    ed.preedit = (focused && composing).then(|| s.preedit.clone());
                    ed.caret_on = focused && s.engaged && coord.text_fields().cursor_visible(now_ms);
                } else {
                    ed.caret = None;
                }
            });
            let state = |id: &str| {
                let w = WidgetId::from(id);
                let mut s = State::NONE;
                if v.is_hovered(&w) {
                    s = s | State::HOVER;
                }
                if v.is_pressed(&w) {
                    s = s | State::ACTIVE;
                }
                if v.is_focused(&w) {
                    s = s | State::FOCUS_VISIBLE;
                }
                s
            };
            let vp = viewport.unwrap_or(rect);
            let d = uzor::ui::dom::lay_out_in(render, sheet, &root, rect.x, rect.y, rect.width, Some(vp), &state);
            let d = if scroll != 0.0 { d.moved(0.0, -scroll) } else { d };
            render.save();
            render.begin_path();
            render.rect(rect.x, rect.y, rect.width, rect.height);
            render.clip();
            uzor::ui::dom::paint(render, &d);
            render.restore();
            coord.set_default_layer(Some(layer.clone()));
            let mut ime = None;
            for b in &d.boxes {
                let Some(id) = b.id.as_deref() else { continue };
                let wid = WidgetId::from(id);
                let name = b.accessible_name().unwrap_or_default().to_string();
                if let Some(Control::Input(e)) = &b.control {
                    let cfg = if e.masked { uzor::TextFieldConfig::password() } else { uzor::TextFieldConfig::text() };
                    coord.register_text_field(wid.clone(), b.rect, cfg);
                    if let Some((_, v)) = seed.iter().find(|(s, _)| *s == wid) {
                        coord.text_fields_mut().set_text(&wid, v);
                    }
                    coord.set_a11y(&wid, &name, None);
                    let c = uzor::ui::dom::content_box(b);
                    let sx = c.x - b.input_scroll();
                    // Boundaries of the shown text (value + composition), mapped back to the buffer.
                    let pos: Vec<f64> = b.bounds.iter().map(|x| sx + x).collect();
                    let _ = e;
                    let fields = coord.text_fields_mut();
                    let pos = match fields.field_state(&wid) {
                        Some(s) => s.buffer_positions(pos),
                        None => pos,
                    };
                    fields.update_field(&wid, (c.x, c.y, c.width, c.height), pos);
                    if e.caret.is_some() {
                        ime = b.caret_rect();
                    }
                } else if b.tab || b.id.is_some() {
                    let sense = if b.tab { focusable() } else { Sense::CLICK | Sense::HOVER };
                    coord.register_atomic(wid.clone(), WidgetKind::Button, b.rect, sense, layer);
                    coord.set_a11y(&wid, &name, b.control.as_ref().and_then(Control::checked));
                }
            }
            coord.set_default_layer(None);
            let ids: Vec<&str> = d.boxes.iter().rev().filter_map(|b| b.id.as_deref()).collect();
            let clicked = ids.iter().find(|i| v.clicked(&WidgetId::from(**i))).map(|s| s.to_string());
            let hovered = ids.iter().find(|i| v.is_hovered(&WidgetId::from(**i))).map(|s| s.to_string());
            let focused = ids.iter().find(|i| v.is_focused(&WidgetId::from(**i))).map(|s| s.to_string());
            (DomResponse { dom: d, clicked, hovered, focused, edited }, ime)
        });
        if let Some(at) = ime {
            cx.ime_area(at);
        }
        resp
    }
}
