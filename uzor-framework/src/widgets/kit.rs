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
//! | input    | [`input`]`(id, rect, text)`    | hovered / focused / clicked     |
//! | select   | [`select`]`(id, rect, options)`| `index` picked, `changed`       |
//! | menu     | [`menu`]`(id, at, items, open)`| `index` picked                  |
//! | dialog   | [`dialog`]`(id, rect)`         | `index` footer button, `clicked` close |
//! | tabs     | [`tabs`]`(id, rect, labels)`   | `index` clicked, `changed`      |
//! | list     | [`list`]`(id, rect, rows)`     | `index` clicked, `changed`      |
//! | splitter | [`splitter`]`(id, axis, span, &mut pos)` | `changed`, `pressed` while dragging |
//! | tooltip  | [`tooltip`]`(id, rect)`        | —                               |
//! | toast    | [`toasts`]`(id, area, &mut stack, now_ms)` | `changed` while any is shown |
//!
//! Overlay-like pieces (select's list, menu, dialog) paint where they are
//! built: build them last in a panel, or inside an overlay hook, so they sit
//! on top.
//!
//! ```ignore
//! use uzor_framework::widgets::kit;
//! let r = kit::tabs("settings-tabs", tabs_rect, &["General", "Usage"]).bind(&mut self.tab).build(cx);
//! kit::input("name", name_rect, &self.name).placeholder("Name").build(cx);
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
    draw_input, InputType, InputView, TextFieldConfig, TextInputSettings,
};
use uzor::ui::widgets::atomic::toast::{draw_toast_stack, ToastStackState};
use uzor::ui::widgets::composite::context_menu::{
    draw_context_menu, register_input_coordinator_context_menu, ContextMenuItem,
    ContextMenuRenderKind, ContextMenuSettings, ContextMenuState, ContextMenuView,
};
use uzor::ui::widgets::composite::dropdown::{
    draw_dropdown, register_input_coordinator_dropdown, DropdownItem, DropdownItemRight,
    DropdownRenderKind, DropdownSettings, DropdownState, DropdownView, DropdownViewKind,
};
use uzor::ui::widgets::composite::modal::render::draw_modal;
use uzor::ui::widgets::composite::modal::{
    register_input_coordinator_modal, BackdropKind, FooterBtn, FooterBtnStyle, ModalRenderKind,
    ModalSettings, ModalState, ModalView,
};
use uzor::ui::widgets::atomic::button::ButtonView;
use uzor::{Rect, WidgetId, WidgetState};

use crate::handle::VisualView;
use crate::types::spec::Spec;

use super::button::{button_settings, paint_button};
use super::host::ContentCx;
use super::paint::{interaction_state, parts, theme_of};
use super::splitter::{grab_rect, register as register_splitter, Splitter, SplitterEvent};

pub use super::lm::{button, tooltip, ButtonBuilder, TooltipBuilder};
pub use super::splitter::Axis;
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

// -- input ------------------------------------------------------------------

/// Single-line text field. Start with [`input`].
pub struct InputBuilder<'a> {
    id: WidgetId,
    rect: Rect,
    text: &'a str,
    placeholder: &'a str,
    label: Option<&'a str>,
    cursor: Option<usize>,
    kind: InputType,
    disabled: bool,
}

/// A text field showing `text` (the app owns the value).
pub fn input<'a>(id: impl Into<WidgetId>, rect: Rect, text: &'a str) -> InputBuilder<'a> {
    InputBuilder {
        id: id.into(),
        rect,
        text,
        placeholder: "",
        label: None,
        cursor: None,
        kind: InputType::Text,
        disabled: false,
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
    /// Caret position in chars (default: end of text).
    pub fn cursor(mut self, c: usize) -> Self {
        self.cursor = Some(c);
        self
    }
    /// Field kind (text, number, search, password).
    pub fn kind(mut self, k: InputType) -> Self {
        self.kind = k;
        self
    }
    /// Masked password field.
    pub fn password(self) -> Self {
        self.kind(InputType::Password)
    }
    /// Disabled colours; no focus.
    pub fn disabled(mut self, on: bool) -> Self {
        self.disabled = on;
        self
    }

    /// Register and draw.
    pub fn build<'c, C: ContentCx<'c>>(self, cx: &mut C) -> Response {
        let theme = theme_of(cx);
        cx.with_paint(|render, widgets, v, _| {
            let r = Response::of(v, &self.id);
            let config = if self.kind == InputType::Password { TextFieldConfig::password() } else { TextFieldConfig::text() };
            let settings = TextInputSettings::with_config(config).with_theme(Box::new(theme));
            let (coord, _, layer) = parts(widgets);
            uzor::ui::widgets::atomic::text_input::register(coord, self.id.clone(), self.rect, layer, &settings);
            coord.set_a11y(&self.id, self.label.unwrap_or(self.placeholder), None);
            let view = InputView {
                text: self.text,
                placeholder: self.placeholder,
                cursor: self.cursor.unwrap_or_else(|| self.text.chars().count()),
                selection: None,
                focused: r.focused && !self.disabled,
                disabled: self.disabled,
                input_type: self.kind,
            };
            let ws = interaction_state(v, &self.id, self.disabled);
            draw_input(render, self.rect, ws, &view, &settings);
            r
        })
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

// -- select -----------------------------------------------------------------

#[derive(Default)]
struct SelectOpen(bool);

/// A button that opens a list of options. Start with [`select`].
pub struct SelectBuilder<'a> {
    id: WidgetId,
    rect: Rect,
    options: &'a [&'a str],
    selected: usize,
    bind: Option<&'a mut usize>,
    disabled: bool,
}

/// A select showing `options[selected]`. The trigger is `id`; options are
/// `"{id}:list:item:{i}"`. The open flag lives in the widget state store.
pub fn select<'a>(id: impl Into<WidgetId>, rect: Rect, options: &'a [&'a str]) -> SelectBuilder<'a> {
    SelectBuilder { id: id.into(), rect, options, selected: 0, bind: None, disabled: false }
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

    /// Register and draw (the open list paints after the trigger).
    pub fn build<'c, C: ContentCx<'c>>(self, cx: &mut C) -> Response {
        let theme = theme_of(cx);
        let ids: Vec<String> = (0..self.options.len()).map(|i| i.to_string()).collect();
        let mut r = cx.with_paint(|render, widgets, v, tokens| {
            let mut r = Response::of(v, &self.id);
            r.index = picked(v, &self.id, "list:item");
            let open = {
                let st = widgets.states.get_or_insert_with(WidgetId::from(format!("{}:open", self.id.as_str())), SelectOpen::default);
                if self.disabled || r.index.is_some() || (st.0 && clicked_outside(v, &self.id)) {
                    st.0 = false;
                } else if r.clicked {
                    st.0 = !st.0;
                }
                st.0
            };
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
                let items: Vec<DropdownItem<'_>> = self
                    .options
                    .iter()
                    .zip(&ids)
                    .map(|(l, id)| DropdownItem::Item {
                        id: id.as_str(),
                        label: l,
                        icon: None,
                        right: DropdownItemRight::None,
                        disabled: false,
                        danger: false,
                        accent_color: None,
                    })
                    .collect();
                let hovered = v.hovered().and_then(|w| w.as_str().strip_prefix(&format!("{}:list:item:", self.id.as_str())).map(str::to_owned));
                let mut state = DropdownState::default();
                state.open_below(self.rect, 4.0);
                let dv = DropdownView {
                    anchor: Some(self.rect),
                    position_override: None,
                    open: true,
                    kind: DropdownViewKind::Flat { items: &items, hovered_id: hovered.as_deref(), submenu_items: None, submenu_hovered_id: None },
                    size_mode: Default::default(),
                    overflow: Default::default(),
                    submenu_width: Default::default(),
                };
                let ds = DropdownSettings { theme: Box::new(theme), ..Default::default() };
                let (coord, _, layer) = parts(widgets);
                register_input_coordinator_dropdown(coord, WidgetId::from(format!("{}:list", self.id.as_str())), self.rect, &state, &dv, &ds, DropdownRenderKind::Flat, layer);
                draw_dropdown(render, self.rect, &state, &dv, &ds, DropdownRenderKind::Flat);
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
