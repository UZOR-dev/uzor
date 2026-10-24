//! Matrix text: a line that decodes out of flickering glyphs, flickers a few cells now
//! and then, and scrambles under the pointer, as a pure function of time.
//!
//! - Decode: from `start`, cells resolve left to right over `decode` seconds; until a cell
//!   resolves it shows a new random glyph every `step` seconds.
//! - Idle: every `idle_every` seconds a few cells (2..4) flicker for `idle_for`.
//! - Brush: cells within `brush` cells of the pointer swap glyphs, always inside `core`,
//!   with a chance falling linearly to the edge; a new roll every `brush_step` seconds.
//!   A swapped cell never shows its own character again.
//!
//! Only letters and digits ever swap; spaces and punctuation hold. There is no state:
//! [`MatrixText::glyph`] gives the character of cell `i` at `t`, so two frames, two
//! threads or a page script and a wasm build agree when they share the clock and seed.

/// Timing and glyphs of the effect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MatrixText {
    /// Glyphs to scramble through (ASCII, so any font has them).
    pub glyphs: &'static [u8],
    /// Seconds between flicker frames.
    pub step: f64,
    /// Seconds for the whole line to decode.
    pub decode: f64,
    /// Seconds between idle flickers (0 = none).
    pub idle_every: f64,
    /// How long an idle flicker lasts.
    pub idle_for: f64,
    /// Pointer brush radius, cells (0 = no brush).
    pub brush: f64,
    /// Inner radius where cells always swap.
    pub core: f64,
    /// Seconds between brush rolls.
    pub brush_step: f64,
    /// Decorrelates lines shown together.
    pub seed: u32,
}

impl Default for MatrixText {
    fn default() -> Self {
        Self {
            glyphs: b"0123456789ABCDEFGHJKLMNPQRSTUVWXYZ#@%&$*/|=<>?+",
            step: 0.06,
            decode: 0.7,
            idle_every: 1.25,
            idle_for: 0.15,
            brush: 3.0,
            core: 1.3,
            brush_step: 0.2,
            seed: 1,
        }
    }
}

fn mix(a: u32, b: u32) -> u32 {
    crate::ui::animation::math::cycle::slot_hash(a, b)
}

impl MatrixText {
    fn pick(&self, orig: char, h: u32) -> char {
        let n = self.glyphs.len().max(1);
        let mut g = self.glyphs[h as usize % n] as char;
        if g.eq_ignore_ascii_case(&orig) {
            g = self.glyphs[(h as usize + 1) % n] as char;
        }
        g
    }

    /// Character of cell `i` (of `n`, original `orig`) at `t` seconds; `start`: when the
    /// decode began; `pointer`: the pointer's position in cells along the line (fractional,
    /// cell `i` spans i..i+1), if it is over the line.
    pub fn glyph(&self, orig: char, i: usize, n: usize, t: f64, start: f64, pointer: Option<f64>) -> char {
        if !orig.is_alphanumeric() {
            return orig;
        }
        let s = self.seed;
        let i32_ = i as u32;
        let tick = (t / self.step.max(1e-3)).floor() as i64 as u32;
        let since = t - start;
        // decode: cell i resolves at (i + 1) / n of the way, with a little jitter
        let jitter = (mix(s ^ 0x5eed, i32_) % 1000) as f64 / 1000.0 * 0.5;
        let at = self.decode * ((i as f64 + 0.5 + jitter) / n.max(1) as f64).min(1.0);
        if since < at {
            return self.pick(orig, mix(s.wrapping_add(i32_), tick));
        }
        // pointer brush
        if let Some(p) = pointer {
            let d = (i as f64 + 0.5 - p).abs();
            if self.brush > 0.0 && d < self.brush {
                let chance = if d <= self.core { 1.0 } else { 1.0 - (d - self.core) / (self.brush - self.core).max(1e-6) };
                let roll = (t / self.brush_step.max(1e-3)).floor() as i64 as u32;
                let h = mix(s ^ 0xb7a5, mix(i32_, roll));
                if ((h >> 8) as f64 / 16_777_216.0) < chance {
                    return self.pick(orig, h);
                }
            }
        }
        // idle flicker: 2..4 cells per window, for the first `idle_for` of it
        if self.idle_every > 0.0 && since > self.decode {
            let w = (t / self.idle_every).floor();
            if t - w * self.idle_every < self.idle_for {
                let wh = mix(s ^ 0x1d1e, w as i64 as u32);
                for j in 0..2 + wh % 3 {
                    if mix(wh, j) as usize % n.max(1) == i {
                        return self.pick(orig, mix(s.wrapping_add(i32_), tick));
                    }
                }
            }
        }
        orig
    }

    /// The whole line at `t` (see [`MatrixText::glyph`]).
    pub fn line(&self, text: &str, t: f64, start: f64, pointer: Option<f64>) -> String {
        let n = text.chars().count();
        text.chars().enumerate().map(|(i, c)| self.glyph(c, i, n, t, start, pointer)).collect()
    }

    /// Whether the line still changes by itself after `t` (decoding, or idle flicker on).
    pub fn moving(&self, t: f64, start: f64) -> bool {
        t - start < self.decode || self.idle_every > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_then_holds() {
        let m = MatrixText { idle_every: 0.0, ..Default::default() };
        let s = "Connecting to HQ";
        assert_ne!(m.line(s, 0.01, 0.0, None), s);
        assert_eq!(m.line(s, 0.01, 0.0, None).chars().nth(10), Some(' '), "spaces hold");
        assert_eq!(m.line(s, 0.8, 0.0, None), s);
        assert_eq!(m.line(s, 0.8, 0.0, None), m.line(s, 0.8, 0.0, None), "pure");
    }

    #[test]
    fn brush_swaps_near_the_pointer_only() {
        let m = MatrixText { idle_every: 0.0, ..Default::default() };
        let s = "abcdefghijklmnopqrstuvwxyz";
        let l: Vec<char> = m.line(s, 5.0, 0.0, Some(13.0)).chars().collect();
        let o: Vec<char> = s.chars().collect();
        assert!(l[12] != o[12] && l[13] != o[13], "core always swaps");
        assert_eq!(l[0], o[0]);
        assert_eq!(l[25], o[25]);
    }

    #[test]
    fn idle_flicker_touches_a_few_cells_briefly() {
        let m = MatrixText::default();
        let s = "loading the engine now";
        let w = 8.0 * m.idle_every;
        let diff = |t: f64| m.line(s, t, 0.0, None).chars().zip(s.chars()).filter(|(a, b)| a != b).count();
        assert!((1..=4).contains(&diff(w + 0.05)));
        assert_eq!(diff(w + 0.5), 0);
    }
}
