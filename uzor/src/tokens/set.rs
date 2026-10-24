//! [`TokenSet`] (loaded, alias/modifier-resolved, not yet validated for
//! completeness) and [`Tokens`] (the widget-consumable, fully validated
//! result of [`TokenSet::resolve`]) — see the H1 token contract design doc
//! §2. Also the `app.*` typed surface ([`AppTokens`], [`TokenValue`]) and
//! the `component.*` override surface ([`ComponentOverrides`], [`ColorSpec`],
//! [`Modifier`]) — coordinator amendment A1: `app.*` is resolved, typed, and
//! never exposes `serde_json::Value` publicly; `component.*` stays symbolic
//! (an alias + optional modifier) because no widget's own default-alias
//! table exists yet to resolve it against (that lands per-widget from H1
//! Brief 4 onward).

use std::sync::OnceLock;
use std::collections::HashMap;

use crate::tokens::color::ColorValue;
use crate::tokens::dtcg::TokenLoadError;
use crate::tokens::geometry::GeometryRoles;
use crate::tokens::semantic::{Role, SemanticRoles};

/// A loaded token file: every `color.*`/`geometry.*` role actually defined
/// (alias chains already followed, modifiers already applied), the fully
/// resolved `app.*` namespace, and the still-symbolic `component.*`
/// overrides. Built only by [`crate::tokens::loader::load_token_set`].
///
/// Deliberately NOT yet validated for completeness — a file may omit
/// required roles, and `TokenSet` will still hold whatever it did define.
/// [`Self::resolve`] is where "does this file define every required role"
/// is checked, so that check has exactly one place to live.
#[derive(Clone, Debug)]
pub struct TokenSet {
    pub(crate) roles: HashMap<Role, ColorValue>,
    pub(crate) geometry: HashMap<&'static str, f32>,
    pub(crate) app: AppTokens,
    pub(crate) components: ComponentOverrides,
    pub(crate) name: Option<String>,
}

impl TokenSet {
    /// Validates that every required role/geometry field is present and
    /// assembles the final, widget-consumable [`Tokens`]. The one place a
    /// missing required token becomes [`TokenLoadError::MissingRequired`].
    pub fn resolve(&self) -> Result<Tokens, TokenLoadError> {
        let required_role = |role: Role| -> Result<ColorValue, TokenLoadError> {
            self.roles.get(&role).cloned().ok_or_else(|| TokenLoadError::MissingRequired {
                path: format!("color.{}", role.path()),
            })
        };
        let semantic = SemanticRoles {
            surface_app_chrome: required_role(Role::SurfaceAppChrome)?,
            surface_control_idle: required_role(Role::SurfaceControlIdle)?,
            surface_control_hover: required_role(Role::SurfaceControlHover)?,
            surface_control_active: required_role(Role::SurfaceControlActive)?,
            surface_floating: required_role(Role::SurfaceFloating)?,
            surface_header: required_role(Role::SurfaceHeader)?,
            surface_panel: required_role(Role::SurfacePanel)?,
            surface_row_alt: self.roles.get(&Role::SurfaceRowAlt).cloned(),
            text_primary: required_role(Role::TextPrimary)?,
            text_secondary: required_role(Role::TextSecondary)?,
            text_muted: required_role(Role::TextMuted)?,
            text_disabled: required_role(Role::TextDisabled)?,
            text_on_accent: required_role(Role::TextOnAccent)?,
            border_subtle: required_role(Role::BorderSubtle)?,
            border_default: required_role(Role::BorderDefault)?,
            border_strong: required_role(Role::BorderStrong)?,
            accent_default: required_role(Role::AccentDefault)?,
            accent_hover: required_role(Role::AccentHover)?,
            accent_pressed: required_role(Role::AccentPressed)?,
            status_success: required_role(Role::StatusSuccess)?,
            status_success_bg: required_role(Role::StatusSuccessBg)?,
            status_danger: required_role(Role::StatusDanger)?,
            status_danger_bg: required_role(Role::StatusDangerBg)?,
            status_warning: required_role(Role::StatusWarning)?,
            status_warning_bg: required_role(Role::StatusWarningBg)?,
            status_info: required_role(Role::StatusInfo)?,
            status_info_bg: required_role(Role::StatusInfoBg)?,
            selection: required_role(Role::Selection)?,
            focus_ring: required_role(Role::FocusRing)?,
            backdrop_dim: required_role(Role::BackdropDim)?,
            backdrop_full: required_role(Role::BackdropFull)?,
            shadow_default: required_role(Role::ShadowDefault)?,
        };

        let required_radius = |key: &'static str| -> Result<f32, TokenLoadError> {
            self.geometry
                .get(key)
                .copied()
                .ok_or_else(|| TokenLoadError::MissingRequired { path: format!("geometry.{key}") })
        };
        let geometry = GeometryRoles {
            radius_sm: required_radius("radius.sm")?,
            radius_md: required_radius("radius.md")?,
            radius_lg: required_radius("radius.lg")?,
        };

        Ok(Tokens::assemble(semantic, geometry, self.name.clone(), self.app.clone(), self.components.clone()))
    }
}

/// The fully validated, widget-consumable result of [`TokenSet::resolve`].
/// `name` replaces `StyleManager::active_preset` (H1 §3) — `None` for a
/// token set loaded without a `$name` metadata key. Per-widget component
/// token fields (`button`, growing by one per converted widget) stay
/// private — the only public read path is through a widget's own theme
/// trait, implemented by [`crate::tokens::theme::TokenTheme`], never a
/// generic stringly getter. Each widget's tokens resolve from `semantic` and
/// `components` on first read, so a binary carries only the resolvers of the
/// widgets it draws. Cloning copies the inputs and starts with empty caches.
#[derive(Debug)]
pub struct Tokens {
    pub semantic: SemanticRoles,
    pub geometry: GeometryRoles,
    pub name: Option<String>,
    pub app: AppTokens,
    pub components: ComponentOverrides,
    /// One lazily resolved component-token cache per widget (`slot::*`), type-erased so the
    /// binary carries drop code only for the widgets it reads.
    caches: [Cache; slot::COUNT],
}

impl Tokens {
    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `ButtonTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn button(&self) -> &crate::ui::widgets::atomic::button::tokens::ButtonTokens {
        self.cached(slot::BUTTON, crate::ui::widgets::atomic::button::tokens::ButtonTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `CheckboxTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn checkbox(&self) -> &crate::ui::widgets::atomic::checkbox::tokens::CheckboxTokens {
        self.cached(slot::CHECKBOX, crate::ui::widgets::atomic::checkbox::tokens::CheckboxTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `ChevronTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn chevron(&self) -> &crate::ui::widgets::atomic::chevron::tokens::ChevronTokens {
        self.cached(slot::CHEVRON, crate::ui::widgets::atomic::chevron::tokens::ChevronTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `ClockTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn clock(&self) -> &crate::ui::widgets::atomic::clock::tokens::ClockTokens {
        self.cached(slot::CLOCK, crate::ui::widgets::atomic::clock::tokens::ClockTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `CloseButtonTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn close_button(&self) -> &crate::ui::widgets::atomic::close_button::tokens::CloseButtonTokens {
        self.cached(slot::CLOSE_BUTTON, crate::ui::widgets::atomic::close_button::tokens::CloseButtonTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `ColorSwatchTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn color_swatch(&self) -> &crate::ui::widgets::atomic::color_swatch::tokens::ColorSwatchTokens {
        self.cached(slot::COLOR_SWATCH, crate::ui::widgets::atomic::color_swatch::tokens::ColorSwatchTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `DragHandleTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn drag_handle(&self) -> &crate::ui::widgets::atomic::drag_handle::tokens::DragHandleTokens {
        self.cached(slot::DRAG_HANDLE, crate::ui::widgets::atomic::drag_handle::tokens::DragHandleTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `DropdownTriggerTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn dropdown_trigger(
        &self,
    ) -> &crate::ui::widgets::atomic::dropdown_trigger::tokens::DropdownTriggerTokens {
        self.cached(slot::DROPDOWN_TRIGGER, crate::ui::widgets::atomic::dropdown_trigger::tokens::DropdownTriggerTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `ItemTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn item(&self) -> &crate::ui::widgets::atomic::item::tokens::ItemTokens {
        self.cached(slot::ITEM, crate::ui::widgets::atomic::item::tokens::ItemTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `RadioTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn radio(&self) -> &crate::ui::widgets::atomic::radio::tokens::RadioTokens {
        self.cached(slot::RADIO, crate::ui::widgets::atomic::radio::tokens::RadioTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `ScrollbarTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn scrollbar(&self) -> &crate::ui::widgets::atomic::scrollbar::tokens::ScrollbarTokens {
        self.cached(slot::SCROLLBAR, crate::ui::widgets::atomic::scrollbar::tokens::ScrollbarTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `ContainerTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn container(&self) -> &crate::ui::widgets::atomic::container::tokens::ContainerTokens {
        self.cached(slot::CONTAINER, crate::ui::widgets::atomic::container::tokens::ContainerTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `ScrollChevronTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn scroll_chevron(
        &self,
    ) -> &crate::ui::widgets::atomic::scroll_chevron::tokens::ScrollChevronTokens {
        self.cached(slot::SCROLL_CHEVRON, crate::ui::widgets::atomic::scroll_chevron::tokens::ScrollChevronTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `SeparatorTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn separator(&self) -> &crate::ui::widgets::atomic::separator::tokens::SeparatorTokens {
        self.cached(slot::SEPARATOR, crate::ui::widgets::atomic::separator::tokens::SeparatorTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `TextTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn text(&self) -> &crate::ui::widgets::atomic::text::tokens::TextTokens {
        self.cached(slot::TEXT, crate::ui::widgets::atomic::text::tokens::TextTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `TabTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn tab(&self) -> &crate::ui::widgets::atomic::tab::tokens::TabTokens {
        self.cached(slot::TAB, crate::ui::widgets::atomic::tab::tokens::TabTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `SliderTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn slider(&self) -> &crate::ui::widgets::atomic::slider::tokens::SliderTokens {
        self.cached(slot::SLIDER, crate::ui::widgets::atomic::slider::tokens::SliderTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `TextInputTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn text_input(&self) -> &crate::ui::widgets::atomic::text_input::tokens::TextInputTokens {
        self.cached(slot::TEXT_INPUT, crate::ui::widgets::atomic::text_input::tokens::TextInputTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `ToggleTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn toggle(&self) -> &crate::ui::widgets::atomic::toggle::tokens::ToggleTokens {
        self.cached(slot::TOGGLE, crate::ui::widgets::atomic::toggle::tokens::ToggleTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `ToastTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn toast(&self) -> &crate::ui::widgets::atomic::toast::tokens::ToastTokens {
        self.cached(slot::TOAST, crate::ui::widgets::atomic::toast::tokens::ToastTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `TooltipTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn tooltip(&self) -> &crate::ui::widgets::atomic::tooltip::tokens::TooltipTokens {
        self.cached(slot::TOOLTIP, crate::ui::widgets::atomic::tooltip::tokens::TooltipTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `BlackboxTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn blackbox_panel(
        &self,
    ) -> &crate::ui::widgets::composite::blackbox_panel::tokens::BlackboxPanelTokens {
        self.cached(slot::BLACKBOX_PANEL, crate::ui::widgets::composite::blackbox_panel::tokens::BlackboxPanelTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `ChromeTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn chrome(&self) -> &crate::ui::widgets::composite::chrome::tokens::ChromeTokens {
        self.cached(slot::CHROME, crate::ui::widgets::composite::chrome::tokens::ChromeTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `ContextMenuTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn context_menu(
        &self,
    ) -> &crate::ui::widgets::composite::context_menu::tokens::ContextMenuTokens {
        self.cached(slot::CONTEXT_MENU, crate::ui::widgets::composite::context_menu::tokens::ContextMenuTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `DropdownTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn dropdown(&self) -> &crate::ui::widgets::composite::dropdown::tokens::DropdownTokens {
        self.cached(slot::DROPDOWN, crate::ui::widgets::composite::dropdown::tokens::DropdownTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `ModalTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn modal(&self) -> &crate::ui::widgets::composite::modal::tokens::ModalTokens {
        self.cached(slot::MODAL, crate::ui::widgets::composite::modal::tokens::ModalTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `PanelTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn panel(&self) -> &crate::ui::widgets::composite::panel::tokens::PanelTokens {
        self.cached(slot::PANEL, crate::ui::widgets::composite::panel::tokens::PanelTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `PopupTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn popup(&self) -> &crate::ui::widgets::composite::popup::tokens::PopupTokens {
        self.cached(slot::POPUP, crate::ui::widgets::composite::popup::tokens::PopupTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `SidebarTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn sidebar(&self) -> &crate::ui::widgets::composite::sidebar::tokens::SidebarTokens {
        self.cached(slot::SIDEBAR, crate::ui::widgets::composite::sidebar::tokens::SidebarTokens::resolve)
    }

    /// This widget's resolved component tokens (H1 §3). `pub(crate)` — read
    /// through `ToolbarTheme`'s trait methods via
    /// [`crate::tokens::theme::TokenTheme`], not directly.
    pub(crate) fn toolbar(&self) -> &crate::ui::widgets::composite::toolbar::tokens::ToolbarTokens {
        self.cached(slot::TOOLBAR, crate::ui::widgets::composite::toolbar::tokens::ToolbarTokens::resolve)
    }
}

/// A resolved `app.*` value — MLC's escape hatch namespace (D4). uzor
/// resolves aliases/modifiers into one of these three kinds but never
/// interprets the path itself.
#[derive(Clone, Debug, PartialEq)]
pub enum TokenValue {
    Color(ColorValue),
    /// A `$type: "dimension"` numeric value (e.g. a spacing/size token).
    Dimension(f32),
    /// A plain `$type: "number"` (or type-less) numeric value.
    Number(f64),
}

impl TokenValue {
    fn kind(&self) -> &'static str {
        match self {
            TokenValue::Color(_) => "color",
            TokenValue::Dimension(_) => "dimension",
            TokenValue::Number(_) => "number",
        }
    }
}

/// A widget's resolved tokens, behind `dyn Any` (see [`Tokens::cached`]).
type Cache = OnceLock<Box<dyn std::any::Any + Send + Sync>>;

/// The index of each widget's cache in `Tokens::caches`.
mod slot {
    pub(super) const BUTTON: usize = 0;
    pub(super) const CHECKBOX: usize = 1;
    pub(super) const CHEVRON: usize = 2;
    pub(super) const CLOCK: usize = 3;
    pub(super) const CLOSE_BUTTON: usize = 4;
    pub(super) const COLOR_SWATCH: usize = 5;
    pub(super) const DRAG_HANDLE: usize = 6;
    pub(super) const DROPDOWN_TRIGGER: usize = 7;
    pub(super) const ITEM: usize = 8;
    pub(super) const RADIO: usize = 9;
    pub(super) const SCROLLBAR: usize = 10;
    pub(super) const CONTAINER: usize = 11;
    pub(super) const SCROLL_CHEVRON: usize = 12;
    pub(super) const SEPARATOR: usize = 13;
    pub(super) const TEXT: usize = 14;
    pub(super) const TAB: usize = 15;
    pub(super) const SLIDER: usize = 16;
    pub(super) const TEXT_INPUT: usize = 17;
    pub(super) const TOGGLE: usize = 18;
    pub(super) const TOAST: usize = 19;
    pub(super) const TOOLTIP: usize = 20;
    pub(super) const BLACKBOX_PANEL: usize = 21;
    pub(super) const CHROME: usize = 22;
    pub(super) const CONTEXT_MENU: usize = 23;
    pub(super) const DROPDOWN: usize = 24;
    pub(super) const MODAL: usize = 25;
    pub(super) const PANEL: usize = 26;
    pub(super) const POPUP: usize = 27;
    pub(super) const SIDEBAR: usize = 28;
    pub(super) const TOOLBAR: usize = 29;
    pub(super) const COUNT: usize = 30;
}

impl Tokens {
    /// The tokens in cache `i`, resolved by `resolve` on first read. Each slot only ever holds
    /// the type its accessor resolves, so the downcast cannot fail.
    fn cached<T: std::any::Any + Send + Sync>(&self, i: usize, resolve: fn(&SemanticRoles, &ComponentOverrides) -> T) -> &T {
        let v = self.caches[i].get_or_init(|| Box::new(resolve(&self.semantic, &self.components)));
        match v.downcast_ref() {
            Some(t) => t,
            None => unreachable!("token cache {i} holds another widget's tokens"),
        }
    }
}

impl Clone for Tokens {
    fn clone(&self) -> Self {
        Tokens::assemble(
            self.semantic.clone(),
            self.geometry.clone(),
            self.name.clone(),
            self.app.clone(),
            self.components.clone(),
        )
    }
}

impl Tokens {
    /// Builds the per-widget component tokens from resolved semantic roles and
    /// the file's `component.*` overrides. Shared by [`TokenSet::resolve`] and
    /// the precompiled built-in sets, so both produce identical [`Tokens`].
    pub(crate) fn assemble(
        semantic: SemanticRoles,
        geometry: GeometryRoles,
        name: Option<String>,
        app: AppTokens,
        components: ComponentOverrides,
    ) -> Tokens {
        Tokens {
            semantic,
            geometry,
            name,
            app,
            components,
            caches: [const { OnceLock::new() }; slot::COUNT],
        }
    }
}

/// The resolved `app.*` namespace: sub-path (with the leading `app.`
/// stripped, e.g. `"chart.candle_up"`) to its resolved [`TokenValue`].
/// Typed getters name the offending path on a miss — no
/// `serde_json::Value` anywhere in this type (coordinator amendment A1).
#[derive(Clone, Debug, Default)]
pub struct AppTokens {
    pub(crate) values: HashMap<String, TokenValue>,
}

impl AppTokens {
    /// Looks up `path` (without the `app.` prefix) expecting a colour.
    pub fn color(&self, path: &str) -> Result<&ColorValue, TokenLoadError> {
        match self.values.get(path) {
            Some(TokenValue::Color(c)) => Ok(c),
            Some(other) => Err(TokenLoadError::WrongType {
                path: format!("app.{path}"),
                expected: "color",
                found: other.kind().to_string(),
            }),
            None => Err(TokenLoadError::AppTokenMissing { path: format!("app.{path}") }),
        }
    }

    /// Looks up `path` (without the `app.` prefix) expecting a dimension.
    pub fn dimension(&self, path: &str) -> Result<f32, TokenLoadError> {
        match self.values.get(path) {
            Some(TokenValue::Dimension(d)) => Ok(*d),
            Some(other) => Err(TokenLoadError::WrongType {
                path: format!("app.{path}"),
                expected: "dimension",
                found: other.kind().to_string(),
            }),
            None => Err(TokenLoadError::AppTokenMissing { path: format!("app.{path}") }),
        }
    }

    /// Looks up `path` (without the `app.` prefix) expecting a plain number.
    pub fn number(&self, path: &str) -> Result<f64, TokenLoadError> {
        match self.values.get(path) {
            Some(TokenValue::Number(n)) => Ok(*n),
            Some(other) => Err(TokenLoadError::WrongType {
                path: format!("app.{path}"),
                expected: "number",
                found: other.kind().to_string(),
            }),
            None => Err(TokenLoadError::AppTokenMissing { path: format!("app.{path}") }),
        }
    }
}

/// An override value as authored for one `component.<widget>.<key>` entry:
/// either a semantic-role alias (with an optional [`Modifier`]) or a
/// hardcoded literal colour — the same two shapes the DTCG `$value`/
/// `$extensions` pair can express (H1 §3). Stays symbolic (not resolved to
/// a final [`ColorValue`]) until a widget's own default-alias table exists
/// to resolve it against ([`Self::resolve`], consumed from H1 Brief 4).
#[derive(Clone, Debug, PartialEq)]
pub enum ColorSpec {
    Alias(Role, Option<Modifier>),
    Literal(ColorValue),
}

/// A modifier applied to an aliased [`ColorSpec`] once its target role is
/// known.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Modifier {
    Alpha(f32),
    Mix(Role, f32),
}

impl ColorSpec {
    /// Resolves this override against a fully-resolved semantic-role set.
    pub fn resolve(&self, roles: &SemanticRoles) -> ColorValue {
        match self {
            ColorSpec::Literal(v) => v.clone(),
            ColorSpec::Alias(role, modifier) => {
                let base = role_value(roles, *role);
                match modifier {
                    Some(Modifier::Alpha(a)) => base.with_alpha(*a),
                    Some(Modifier::Mix(other, t)) => base.mix(&role_value(roles, *other), *t),
                    None => base,
                }
            }
        }
    }
}

/// [`Role`] to [`SemanticRoles`] field lookup used only by [`ColorSpec`]
/// resolution. Deliberately not a method on `SemanticRoles` itself — that
/// type's own doc comment fixes it as never exposing a runtime `get(Role)`
/// lookup surface (H1 Brief 1); callers of `SemanticRoles` still always use
/// the named field directly.
fn role_value(roles: &SemanticRoles, role: Role) -> ColorValue {
    match role {
        Role::SurfaceAppChrome => roles.surface_app_chrome.clone(),
        Role::SurfaceControlIdle => roles.surface_control_idle.clone(),
        Role::SurfaceControlHover => roles.surface_control_hover.clone(),
        Role::SurfaceControlActive => roles.surface_control_active.clone(),
        Role::SurfaceFloating => roles.surface_floating.clone(),
        Role::SurfaceHeader => roles.surface_header.clone(),
        Role::SurfacePanel => roles.surface_panel.clone(),
        Role::SurfaceRowAlt => roles.surface_row_alt.clone().unwrap_or(ColorValue::Transparent),
        Role::TextPrimary => roles.text_primary.clone(),
        Role::TextSecondary => roles.text_secondary.clone(),
        Role::TextMuted => roles.text_muted.clone(),
        Role::TextDisabled => roles.text_disabled.clone(),
        Role::TextOnAccent => roles.text_on_accent.clone(),
        Role::BorderSubtle => roles.border_subtle.clone(),
        Role::BorderDefault => roles.border_default.clone(),
        Role::BorderStrong => roles.border_strong.clone(),
        Role::AccentDefault => roles.accent_default.clone(),
        Role::AccentHover => roles.accent_hover.clone(),
        Role::AccentPressed => roles.accent_pressed.clone(),
        Role::StatusSuccess => roles.status_success.clone(),
        Role::StatusSuccessBg => roles.status_success_bg.clone(),
        Role::StatusDanger => roles.status_danger.clone(),
        Role::StatusDangerBg => roles.status_danger_bg.clone(),
        Role::StatusWarning => roles.status_warning.clone(),
        Role::StatusWarningBg => roles.status_warning_bg.clone(),
        Role::StatusInfo => roles.status_info.clone(),
        Role::StatusInfoBg => roles.status_info_bg.clone(),
        Role::Selection => roles.selection.clone(),
        Role::FocusRing => roles.focus_ring.clone(),
        Role::BackdropDim => roles.backdrop_dim.clone(),
        Role::BackdropFull => roles.backdrop_full.clone(),
        Role::ShadowDefault => roles.shadow_default.clone(),
    }
}

/// `component.*` overrides from one loaded token file: widget name to its
/// key-to-[`ColorSpec`] map. Only ever contains entries for keys a widget
/// has registered via [`crate::tokens::component_keys`] — the loader
/// rejects anything else as [`TokenLoadError::UnknownPath`], so this type
/// never needs to represent a rejected/unknown entry.
#[derive(Clone, Debug, Default)]
pub struct ComponentOverrides {
    pub(crate) widgets: HashMap<String, HashMap<String, ColorSpec>>,
}

impl ComponentOverrides {
    /// The override authored for `widget`'s `key` component token, if the
    /// file defined one.
    pub fn get(&self, widget: &str, key: &str) -> Option<&ColorSpec> {
        self.widgets.get(widget)?.get(key)
    }
}
