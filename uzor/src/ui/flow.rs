//! Flex and grid track solvers (DOM-like roadmap item 2): the arithmetic behind
//! `display:flex` / `display:grid` so screens stop hand-computing x offsets.
//!
//! All functions are pure and return `(offset, size)` pairs along one axis.
//!
//! ```
//! use uzor::ui::flow::{grid_tracks, Track};
//! // grid-template-columns: 34px 1fr 76px 190px 64px; gap: 10px
//! let cols = grid_tracks(600.0, &[Track::Px(34.0), Track::Fr(1.0), Track::Px(76.0), Track::Px(190.0), Track::Px(64.0)], 10.0);
//! assert_eq!(cols[1], (44.0, 196.0));
//! ```

/// `justify-content` along the main axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Justify {
    #[default]
    Start,
    End,
    Center,
    SpaceBetween,
}

/// One flex item: `flex: grow shrink basis` with min/max clamps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlexItem {
    pub basis: f64,
    pub grow: f64,
    pub shrink: f64,
    pub min: f64,
    pub max: f64,
}

impl FlexItem {
    /// A fixed-size item (`flex: none`).
    pub fn fixed(w: f64) -> Self {
        FlexItem { basis: w, grow: 0.0, shrink: 0.0, min: w, max: w }
    }
    /// `flex: 1` (basis 0, grows to fill).
    pub fn fill() -> Self {
        FlexItem { basis: 0.0, grow: 1.0, shrink: 1.0, min: 0.0, max: f64::INFINITY }
    }
    /// Content-sized item that may shrink (`flex: 0 1 auto`).
    pub fn auto(w: f64) -> Self {
        FlexItem { basis: w, grow: 0.0, shrink: 1.0, min: 0.0, max: f64::INFINITY }
    }
}

/// Lay items out on one line of `avail` pixels (a single-line flex container).
pub fn flex_row(avail: f64, items: &[FlexItem], gap: f64, justify: Justify) -> Vec<(f64, f64)> {
    if items.is_empty() {
        return vec![];
    }
    let gaps = gap * (items.len() - 1) as f64;
    let mut size: Vec<f64> = items.iter().map(|i| i.basis.clamp(i.min, i.max)).collect();
    let mut frozen = vec![false; items.len()];
    // Iterative resolution (CSS flexible lengths): distribute, clamp, freeze violators, repeat.
    for _ in 0..items.len() + 1 {
        let free = avail - gaps - size.iter().sum::<f64>();
        let grow = free > 0.0;
        let weight = |k: usize| if grow { items[k].grow } else { items[k].shrink * items[k].basis.max(1e-9) };
        let total: f64 = (0..items.len()).filter(|&k| !frozen[k]).map(weight).sum();
        if free.abs() < 1e-9 || total <= 0.0 {
            break;
        }
        let mut clamped = false;
        for k in 0..items.len() {
            if frozen[k] {
                continue;
            }
            let want = size[k] + free * weight(k) / total;
            let got = want.clamp(items[k].min, items[k].max);
            if (got - want).abs() > 1e-9 {
                frozen[k] = true;
                clamped = true;
            }
            size[k] = got;
        }
        if !clamped {
            break;
        }
    }
    let used = size.iter().sum::<f64>() + gaps;
    let rest = (avail - used).max(0.0);
    let (mut x, step) = match justify {
        Justify::Start => (0.0, gap),
        Justify::End => (rest, gap),
        Justify::Center => (rest / 2.0, gap),
        Justify::SpaceBetween if items.len() > 1 => (0.0, gap + rest / (items.len() - 1) as f64),
        Justify::SpaceBetween => (0.0, gap),
    };
    size.iter()
        .map(|&w| {
            let r = (x, w);
            x += w + step;
            r
        })
        .collect()
}

/// `flex-wrap: wrap`: place fixed-size boxes left to right, breaking lines at `avail`.
/// Returns `(x, y)` per box and the total height.
pub fn flex_wrap(avail: f64, sizes: &[(f64, f64)], gap_x: f64, gap_y: f64) -> (Vec<(f64, f64)>, f64) {
    let (mut x, mut y, mut line_h) = (0.0, 0.0, 0.0f64);
    let mut out = Vec::with_capacity(sizes.len());
    for &(w, h) in sizes {
        if x > 0.0 && x + w > avail + 1e-9 {
            y += line_h + gap_y;
            x = 0.0;
            line_h = 0.0;
        }
        out.push((x, y));
        x += w + gap_x;
        line_h = line_h.max(h);
    }
    (out, if sizes.is_empty() { 0.0 } else { y + line_h })
}

/// A grid track size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Track {
    Px(f64),
    Fr(f64),
    /// `minmax(min, Nfr)`
    MinMax(f64, f64),
}

/// `grid-template-columns` resolution: fixed tracks first, the rest shared by `fr`,
/// never below a `minmax` minimum.
pub fn grid_tracks(avail: f64, tracks: &[Track], gap: f64) -> Vec<(f64, f64)> {
    let fr: f64 = tracks.iter().map(|t| match *t { Track::Px(_) => 0.0, Track::Fr(f) | Track::MinMax(_, f) => f }).sum();
    // Each fr track gets max(min, its fr share of the space left after fixed tracks and gaps).
    let fixed: f64 = tracks.iter().filter_map(|t| if let Track::Px(w) = t { Some(*w) } else { None }).sum::<f64>()
        + gap * tracks.len().saturating_sub(1) as f64;
    let flex_space = (avail - fixed).max(0.0);
    let mut x = 0.0;
    tracks
        .iter()
        .map(|t| {
            let w = match *t {
                Track::Px(w) => w,
                Track::Fr(f) => flex_space * f / fr.max(1e-9),
                Track::MinMax(m, f) => (flex_space * f / fr.max(1e-9)).max(m),
            };
            let r = (x, w);
            x += w + gap;
            r
        })
        .collect()
}

/// `repeat(auto-fill, minmax(min, 1fr))`: column count and column width.
pub fn auto_fill(avail: f64, min: f64, gap: f64) -> (usize, f64) {
    let n = (((avail + gap) / (min + gap)).floor() as usize).max(1);
    (n, ((avail - gap * (n - 1) as f64) / n as f64).max(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: &[(f64, f64)], b: &[(f64, f64)]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(p, q)| (p.0 - q.0).abs() < 1e-6 && (p.1 - q.1).abs() < 1e-6)
    }

    #[test]
    fn flex_grow_and_justify() {
        let r = flex_row(300.0, &[FlexItem::fixed(50.0), FlexItem::fill(), FlexItem::fixed(70.0)], 10.0, Justify::Start);
        assert!(close(&r, &[(0.0, 50.0), (60.0, 160.0), (230.0, 70.0)]));
        let r = flex_row(100.0, &[FlexItem::fixed(20.0), FlexItem::fixed(20.0)], 0.0, Justify::End);
        assert!(close(&r, &[(60.0, 20.0), (80.0, 20.0)]));
        let r = flex_row(100.0, &[FlexItem::fixed(20.0), FlexItem::fixed(20.0)], 0.0, Justify::Center);
        assert!(close(&r, &[(30.0, 20.0), (50.0, 20.0)]));
        let r = flex_row(100.0, &[FlexItem::fixed(20.0), FlexItem::fixed(20.0), FlexItem::fixed(20.0)], 0.0, Justify::SpaceBetween);
        assert!(close(&r, &[(0.0, 20.0), (40.0, 20.0), (80.0, 20.0)]));
    }

    #[test]
    fn flex_shrink_respects_min_and_max() {
        // two auto items overflow by 50: shrink proportional to basis
        let r = flex_row(150.0, &[FlexItem::auto(100.0), FlexItem::auto(100.0)], 0.0, Justify::Start);
        assert!(close(&r, &[(0.0, 75.0), (75.0, 75.0)]));
        // one item with a min freezes, the other absorbs the rest
        let a = FlexItem { min: 90.0, ..FlexItem::auto(100.0) };
        let r = flex_row(150.0, &[a, FlexItem::auto(100.0)], 0.0, Justify::Start);
        assert!(close(&r, &[(0.0, 90.0), (90.0, 60.0)]));
        // max clamps growth; the leftover goes to the other grower
        let capped = FlexItem { max: 40.0, ..FlexItem::fill() };
        let r = flex_row(200.0, &[capped, FlexItem::fill()], 0.0, Justify::Start);
        assert!(close(&r, &[(0.0, 40.0), (40.0, 160.0)]));
    }

    #[test]
    fn wrap_breaks_lines() {
        let (pos, h) = flex_wrap(100.0, &[(40.0, 24.0), (40.0, 24.0), (40.0, 24.0)], 5.0, 5.0);
        assert_eq!(pos, vec![(0.0, 0.0), (45.0, 0.0), (0.0, 29.0)]);
        assert_eq!(h, 53.0);
        // an oversize first box still sits on line one
        assert_eq!(flex_wrap(10.0, &[(40.0, 20.0)], 5.0, 5.0), (vec![(0.0, 0.0)], 20.0));
    }

    #[test]
    fn grid_fr_and_minmax() {
        let r = grid_tracks(310.0, &[Track::Fr(2.0), Track::Fr(1.0)], 10.0);
        assert!(close(&r, &[(0.0, 200.0), (210.0, 100.0)]));
        let r = grid_tracks(100.0, &[Track::MinMax(140.0, 1.0), Track::Px(50.0)], 0.0);
        assert!(close(&r, &[(0.0, 140.0), (140.0, 50.0)]));
        assert_eq!(auto_fill(800.0, 380.0, 10.0), (2, 395.0));
        assert_eq!(auto_fill(300.0, 380.0, 10.0), (1, 300.0));
    }
}
