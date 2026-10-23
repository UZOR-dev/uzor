//! Anchored placement for portal-like layers (menus, dropdowns, submenus,
//! tooltips): the box opens on one side of its anchor, flips to the
//! opposite side when it does not fit there and the opposite side has more
//! room, then shifts along the cross axis to stay inside the viewport. The
//! Floating UI / Popper default (`flip` + `shift`), so a dropdown whose
//! trigger sits inside a clipped, scrolled container still lands fully on
//! screen: the overlay composes in its own layer, outside every app clip.

use crate::core::types::Rect;

/// The side of the anchor a placed box opens on. The cross axis aligns to
/// the anchor's start edge (left for `Below` / `Above`, top for `Right` /
/// `Left`): `bottom-start`, `top-start`, `right-start`, `left-start`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Side {
    /// Under the anchor (`bottom-start`), the dropdown default.
    #[default]
    Below,
    /// Over the anchor (`top-start`).
    Above,
    /// Right of the anchor (`right-start`), the submenu default.
    Right,
    /// Left of the anchor (`left-start`).
    Left,
}

impl Side {
    fn flipped(self) -> Self {
        match self {
            Self::Below => Self::Above,
            Self::Above => Self::Below,
            Self::Right => Self::Left,
            Self::Left => Self::Right,
        }
    }
}

/// Place a `w` × `h` box on `side` of `anchor`, `gap` px away, inside
/// `viewport` minus `margin` on every edge. Flips to the opposite side when
/// the box overflows on `side` and the opposite side has more room; then
/// clamps (shifts) both axes into the viewport. `offset` moves the cross
/// axis start (a submenu's `-6` so its first row lines up with the row).
pub fn place_anchored(anchor: Rect, w: f64, h: f64, viewport: Rect, side: Side, gap: f64, margin: f64, offset: f64) -> Rect {
    let (lo_x, hi_x) = (viewport.x + margin, viewport.x + viewport.width - margin);
    let (lo_y, hi_y) = (viewport.y + margin, viewport.y + viewport.height - margin);
    // Room on each side of the anchor, along the main axis.
    let room = |s: Side| match s {
        Side::Below => hi_y - (anchor.y + anchor.height + gap),
        Side::Above => (anchor.y - gap) - lo_y,
        Side::Right => hi_x - (anchor.x + anchor.width + gap),
        Side::Left => (anchor.x - gap) - lo_x,
    };
    let need = |s: Side| if matches!(s, Side::Below | Side::Above) { h } else { w };
    let side = if room(side) < need(side) && room(side.flipped()) > room(side) { side.flipped() } else { side };
    let (x, y) = match side {
        Side::Below => (anchor.x + offset, anchor.y + anchor.height + gap),
        Side::Above => (anchor.x + offset, anchor.y - gap - h),
        Side::Right => (anchor.x + anchor.width + gap, anchor.y + offset),
        Side::Left => (anchor.x - gap - w, anchor.y + offset),
    };
    // Shift into the viewport; a box larger than it keeps its start edge in.
    let clamp = |v: f64, size: f64, lo: f64, hi: f64| v.min(hi - size).max(lo);
    Rect::new(clamp(x, w, lo_x, hi_x), clamp(y, h, lo_y, hi_y), w, h)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VP: Rect = Rect { x: 0.0, y: 0.0, width: 800.0, height: 600.0 };

    #[test]
    fn opens_on_its_side_then_flips_and_shifts() {
        let trigger = Rect::new(100.0, 100.0, 120.0, 30.0);
        assert_eq!(place_anchored(trigger, 200.0, 150.0, VP, Side::Below, 4.0, 8.0, 0.0), Rect::new(100.0, 134.0, 200.0, 150.0));
        // Near the bottom: no room below, more above, so it flips.
        let low = Rect::new(100.0, 520.0, 120.0, 30.0);
        assert_eq!(place_anchored(low, 200.0, 150.0, VP, Side::Below, 4.0, 8.0, 0.0), Rect::new(100.0, 366.0, 200.0, 150.0));
        // Near the right edge: shifted left to stay on screen.
        let right = Rect::new(700.0, 100.0, 60.0, 30.0);
        assert_eq!(place_anchored(right, 200.0, 150.0, VP, Side::Below, 0.0, 8.0, 0.0).x, 592.0);
        // A submenu flips to the left of its menu at the right edge, and
        // its offset top shifts down to the margin at the top.
        let menu = Rect::new(560.0, 2.0, 200.0, 32.0);
        assert_eq!(place_anchored(menu, 180.0, 100.0, VP, Side::Right, 4.0, 8.0, -6.0), Rect::new(376.0, 8.0, 180.0, 100.0));
        // No room on either side: it keeps the side with more room and
        // shifts in rather than leaving the viewport.
        let tall = place_anchored(Rect::new(10.0, 290.0, 50.0, 20.0), 100.0, 700.0, VP, Side::Below, 0.0, 0.0, 0.0);
        assert_eq!((tall.y, tall.height), (0.0, 700.0));
    }
}
