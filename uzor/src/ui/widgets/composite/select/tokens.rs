//! Default look of the open select list (dark, as hq / app.js render it). Apps restyle it
//! from CSS (`SelectStyle::from_sheet`).

use super::SelectStyle;
use crate::ui::widgets::composite::dialog::Shadow;

impl Default for SelectStyle {
    fn default() -> Self {
        Self {
            bg: "#1f1f1f",
            border: Some("#2e2e34"),
            radius: 10.0,
            shadow: Some(Shadow { dy: 8.0, blur: 24.0, color: "rgba(0,0,0,0.5)" }),
            color: "#f5f5f5",
            sub: "#acacac",
            px: 13.0,
            row_h: 30.0,
            row_radius: 7.0,
            row_hover: "#2a2a2a",
            row_hi: "#2f2f2f",
            check: "#3b82f6",
            search_bg: "#1b1b1b",
            rows: 10.0,
            min_w: 160.0,
            search_min_w: 280.0,
            thumb: "#4a4a4a",
            gap: 4.0,
        }
    }
}
