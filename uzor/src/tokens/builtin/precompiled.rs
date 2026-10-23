//! The `dark` and `light` built-in sets, precompiled: the result of loading and
//! resolving `builtin/dark.json` / `builtin/light.json`, written out as Rust so a
//! binary that only uses these sets does not link the DTCG loader. Generated
//! from the JSON; `precompiled_sets_match_their_json` in `builtin.rs` fails if
//! either drifts from its file.

use crate::tokens::color::{ColorValue, Rgba};
use crate::tokens::geometry::GeometryRoles;
use crate::tokens::semantic::SemanticRoles;

const fn c(r: u8, g: u8, b: u8, a: u8) -> ColorValue {
    ColorValue::Solid(Rgba { r, g, b, a })
}

/// `builtin/dark.json`, resolved: name, geometry, semantic roles.
pub(super) fn dark() -> (&'static str, GeometryRoles, SemanticRoles) {
    ("Dark", GeometryRoles { radius_sm: 2.0, radius_md: 4.0, radius_lg: 8.0 }, SemanticRoles {
        surface_app_chrome: c(19, 23, 34, 255),
        surface_control_idle: c(30, 34, 45, 255),
        surface_control_hover: c(42, 46, 57, 255),
        surface_control_active: c(41, 98, 255, 255),
        surface_floating: c(30, 34, 45, 255),
        surface_header: c(30, 34, 45, 255),
        surface_panel: c(30, 34, 45, 255),
        surface_row_alt: None,
        text_primary: c(209, 212, 220, 255),
        text_secondary: c(178, 181, 190, 255),
        text_muted: c(120, 123, 134, 255),
        text_disabled: c(106, 109, 120, 255),
        text_on_accent: c(209, 212, 220, 255),
        border_subtle: c(42, 46, 57, 255),
        border_default: c(54, 58, 69, 255),
        border_strong: c(54, 58, 69, 255),
        accent_default: c(41, 98, 255, 255),
        accent_hover: c(30, 83, 228, 255),
        accent_pressed: c(30, 83, 228, 255),
        status_success: c(38, 166, 154, 255),
        status_success_bg: c(38, 166, 154, 38),
        status_danger: c(242, 54, 69, 255),
        status_danger_bg: c(242, 54, 69, 38),
        status_warning: c(255, 152, 0, 255),
        status_warning_bg: c(255, 152, 0, 38),
        status_info: c(41, 98, 255, 255),
        status_info_bg: c(41, 98, 255, 38),
        selection: c(41, 98, 255, 85),
        focus_ring: c(41, 98, 255, 255),
        backdrop_dim: c(0, 0, 0, 114),
        backdrop_full: c(19, 23, 34, 255),
        shadow_default: c(0, 0, 0, 102),
    })
}

/// `builtin/light.json`, resolved: name, geometry, semantic roles.
pub(super) fn light() -> (&'static str, GeometryRoles, SemanticRoles) {
    ("Light", GeometryRoles { radius_sm: 2.0, radius_md: 4.0, radius_lg: 8.0 }, SemanticRoles {
        surface_app_chrome: c(248, 249, 250, 255),
        surface_control_idle: c(233, 236, 239, 255),
        surface_control_hover: c(222, 226, 230, 255),
        surface_control_active: c(74, 144, 217, 255),
        surface_floating: c(255, 255, 255, 255),
        surface_header: c(255, 255, 255, 255),
        surface_panel: c(255, 255, 255, 255),
        surface_row_alt: None,
        text_primary: c(19, 23, 34, 255),
        text_secondary: c(67, 70, 81, 255),
        text_muted: c(120, 123, 134, 255),
        text_disabled: c(120, 123, 134, 255),
        text_on_accent: c(19, 23, 34, 255),
        border_subtle: c(233, 236, 239, 255),
        border_default: c(222, 226, 230, 255),
        border_strong: c(222, 226, 230, 255),
        accent_default: c(74, 144, 217, 255),
        accent_hover: c(58, 123, 200, 255),
        accent_pressed: c(58, 123, 200, 255),
        status_success: c(38, 166, 154, 255),
        status_success_bg: c(38, 166, 154, 38),
        status_danger: c(242, 54, 69, 255),
        status_danger_bg: c(242, 54, 69, 38),
        status_warning: c(255, 152, 0, 255),
        status_warning_bg: c(255, 152, 0, 38),
        status_info: c(74, 144, 217, 255),
        status_info_bg: c(74, 144, 217, 38),
        selection: c(74, 144, 217, 64),
        focus_ring: c(74, 144, 217, 255),
        backdrop_dim: c(0, 0, 0, 114),
        backdrop_full: c(248, 249, 250, 255),
        shadow_default: c(0, 0, 0, 102),
    })
}
