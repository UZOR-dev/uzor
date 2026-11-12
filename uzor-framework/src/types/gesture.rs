//! Touch gestures: one recogniser for one-finger tap / long-press / pan / swipe and
//! two-finger pinch, a release-velocity tracker, and the fling (inertia) that follows
//! a pan. Pure state over `(contact id, position, time)`; the kernel's touch route
//! feeds it and turns its answers into pointer, wheel and gesture events.
//!
//! Positions are window-local logical pixels, times are seconds.

use crate::types::window::Point;

/// Thresholds. The defaults follow the platform feel (UIKit / Android `ViewConfiguration`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GestureConfig {
    /// Movement a finger may make and still tap (Android touch slop is 8 dp).
    pub slop: f64,
    /// Hold time for a long-press, s.
    pub long_press: f64,
    /// A pan that starts this close to a window edge is an edge pan (back-swipe, drawer).
    pub edge: f64,
    /// A released pan is a swipe when it travelled at least this far…
    pub swipe_min_dist: f64,
    /// …and left the screen at least this fast, px/s.
    pub swipe_min_speed: f64,
    /// Release speed below which a pan does not fling, px/s.
    pub fling_min_speed: f64,
    /// Release speeds are capped here, px/s.
    pub fling_max_speed: f64,
    /// Exponential deceleration of a fling, 1/s (`v(t) = v0 * e^(-k t)`; 2.0 is close to
    /// UIKit's normal deceleration rate of 0.998 per ms).
    pub fling_decel: f64,
    /// A fling ends below this speed, px/s.
    pub fling_stop_speed: f64,
    /// Velocity is measured over this much history before the release, s.
    pub velocity_window: f64,
    /// A finger that rested this long before lifting leaves with zero velocity, s.
    pub velocity_rest: f64,
    /// A second tap within this long after the first is a double-tap, s.
    pub double_tap: f64,
    /// …and within this distance of it, px.
    pub double_tap_slop: f64,
}

impl Default for GestureConfig {
    fn default() -> Self {
        Self {
            slop: 8.0,
            long_press: 0.5,
            edge: 20.0,
            swipe_min_dist: 48.0,
            swipe_min_speed: 400.0,
            fling_min_speed: 150.0,
            fling_max_speed: 8000.0,
            fling_decel: 2.0,
            fling_stop_speed: 20.0,
            velocity_window: 0.1,
            velocity_rest: 0.05,
            double_tap: 0.3,
            double_tap_slop: 24.0,
        }
    }
}

/// The dominant direction of a pan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    /// Left / right.
    Horizontal,
    /// Up / down.
    Vertical,
}

/// A window edge a pan started at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Edge {
    /// Left edge (iOS back-swipe, a left drawer).
    Left,
    /// Right edge (a right drawer).
    Right,
    /// Top edge (a pull-down shade).
    Top,
    /// Bottom edge (a bottom sheet).
    Bottom,
}

/// Where a swipe went.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SwipeDir {
    /// Towards the left edge.
    Left,
    /// Towards the right edge.
    Right,
    /// Towards the top.
    Up,
    /// Towards the bottom.
    Down,
}

/// One recognised gesture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gesture {
    /// A contact lifted within the slop and before the long-press time.
    Tap {
        /// Where.
        pos: Point,
    },
    /// A contact held still for the long-press time (fires while still down).
    LongPress {
        /// Where.
        pos: Point,
    },
    /// A second tap close to the first one in time and place (sent after its `Tap`).
    DoubleTap {
        /// Where the second tap lifted.
        pos: Point,
    },
    /// The long-pressed finger moved (a chart crosshair, a drag-to-reorder).
    LongPressDrag {
        /// Where the finger is.
        pos: Point,
        /// Motion since the last event.
        delta: (f64, f64),
    },
    /// The long-pressed finger lifted.
    LongPressEnd {
        /// Where.
        pos: Point,
    },
    /// One finger left the slop: a pan begins.
    PanStart {
        /// Where the finger went down.
        origin: Point,
        /// Where it is now.
        pos: Point,
        /// Dominant direction when it left the slop.
        axis: Axis,
        /// The window edge it started at, if any.
        edge: Option<Edge>,
    },
    /// The pan moved.
    Pan {
        /// Where the finger is.
        pos: Point,
        /// Motion since the last event.
        delta: (f64, f64),
        /// Total motion since the finger went down.
        total: (f64, f64),
        /// The edge the pan started at.
        edge: Option<Edge>,
    },
    /// The pan's finger lifted.
    PanEnd {
        /// Where.
        pos: Point,
        /// Release velocity, px/s.
        velocity: (f64, f64),
        /// The edge the pan started at.
        edge: Option<Edge>,
    },
    /// A fast, long enough pan (sent after its `PanEnd`).
    Swipe {
        /// Direction.
        dir: SwipeDir,
        /// The edge the pan started at (an edge swipe: back, open drawer).
        edge: Option<Edge>,
        /// Release speed along `dir`, px/s.
        speed: f64,
    },
    /// Two fingers: the pinch changed. The midpoint moving is the two-finger pan
    /// ([`Gesture::PinchPan`] carries its step).
    Pinch {
        /// Midpoint between the fingers.
        center: Point,
        /// Distance now / distance when the second finger went down.
        scale: f64,
    },
    /// Two fingers: the midpoint moved (sent with the `Pinch` of the same motion).
    PinchPan {
        /// Midpoint between the fingers.
        center: Point,
        /// Midpoint motion since the last event.
        delta: (f64, f64),
        /// Midpoint motion since the second finger went down.
        total: (f64, f64),
    },
    /// The second finger lifted (or the gesture was cancelled).
    PinchEnd {
        /// Final scale.
        scale: f64,
    },
}

/// The recogniser's phase.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// No contact.
    Idle,
    /// One contact inside the slop: may still tap, long-press or pan.
    Pending,
    /// The long-press fired; the contact is still down.
    LongPressed,
    /// One contact panning.
    Pan(Axis),
    /// Two or more contacts.
    Pinch,
}

/// Release velocity from the last [`GestureConfig::velocity_window`] of samples.
#[derive(Clone, Debug, Default)]
pub struct VelocityTracker {
    samples: Vec<(f64, Point)>,
}

impl VelocityTracker {
    /// Forget every sample.
    pub fn reset(&mut self) {
        self.samples.clear();
    }

    /// Record a position at time `t`.
    pub fn add(&mut self, t: f64, pos: Point, cfg: &GestureConfig) {
        self.samples.push((t, pos));
        let keep = cfg.velocity_window * 2.0;
        self.samples.retain(|(s, _)| t - s <= keep);
    }

    /// Velocity at `now`, px/s, capped at [`GestureConfig::fling_max_speed`]. Zero when
    /// the finger rested for [`GestureConfig::velocity_rest`] before `now`.
    pub fn velocity(&self, now: f64, cfg: &GestureConfig) -> (f64, f64) {
        let Some(&(t1, p1)) = self.samples.last() else { return (0.0, 0.0) };
        if now - t1 > cfg.velocity_rest {
            return (0.0, 0.0);
        }
        let Some(&(t0, p0)) = self.samples.iter().find(|(t, _)| t1 - t <= cfg.velocity_window) else {
            return (0.0, 0.0);
        };
        let dt = t1 - t0;
        if dt <= 1e-6 {
            return (0.0, 0.0);
        }
        let (vx, vy) = ((p1.x - p0.x) / dt, (p1.y - p0.y) / dt);
        let speed = vx.hypot(vy);
        if speed > cfg.fling_max_speed {
            let k = cfg.fling_max_speed / speed;
            (vx * k, vy * k)
        } else {
            (vx, vy)
        }
    }
}

/// Inertia after a pan: exponential deceleration, stepped by time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fling {
    v0: (f64, f64),
    t0: f64,
    last: f64,
    decel: f64,
    stop: f64,
}

impl Fling {
    /// A fling leaving at `velocity` (px/s) at time `t`; `None` below the fling threshold.
    pub fn new(velocity: (f64, f64), t: f64, cfg: &GestureConfig) -> Option<Self> {
        let speed = velocity.0.hypot(velocity.1);
        if speed < cfg.fling_min_speed {
            return None;
        }
        Some(Self { v0: velocity, t0: t, last: t, decel: cfg.fling_decel.max(0.1), stop: cfg.fling_stop_speed })
    }

    /// Distance travelled from `t0` to `t0 + dt`.
    fn travelled(&self, dt: f64) -> (f64, f64) {
        let f = (1.0 - (-self.decel * dt).exp()) / self.decel;
        (self.v0.0 * f, self.v0.1 * f)
    }

    /// Current velocity, px/s.
    pub fn velocity(&self, now: f64) -> (f64, f64) {
        let e = (-self.decel * (now - self.t0).max(0.0)).exp();
        (self.v0.0 * e, self.v0.1 * e)
    }

    /// Total distance the fling will travel until it stops.
    pub fn distance(&self) -> (f64, f64) {
        (self.v0.0 / self.decel, self.v0.1 / self.decel)
    }

    /// The motion since the last step, or `None` once the fling has slowed below
    /// the stop speed (the fling is over; drop it).
    pub fn step(&mut self, now: f64) -> Option<(f64, f64)> {
        if now <= self.last {
            return Some((0.0, 0.0));
        }
        let (vx, vy) = self.velocity(self.last);
        if vx.hypot(vy) < self.stop {
            return None;
        }
        let a = self.travelled(self.last - self.t0);
        let b = self.travelled(now - self.t0);
        self.last = now;
        Some((b.0 - a.0, b.1 - a.1))
    }
}

/// iOS-style rubber band: how far content visibly moves for `overshoot` px of finger
/// travel past an edge, in a viewport `dim` px long (always less than both).
pub fn rubber_band(overshoot: f64, dim: f64) -> f64 {
    if overshoot <= 0.0 || dim <= 0.0 {
        return 0.0;
    }
    (1.0 - 1.0 / (overshoot * 0.55 / dim + 1.0)) * dim
}

/// The tap / long-press / pan / swipe / pinch recogniser.
#[derive(Clone, Debug)]
pub struct GestureRecognizer {
    cfg: GestureConfig,
    viewport: (f64, f64),
    contacts: Vec<(u64, Point)>,
    phase: Phase,
    origin: Point,
    down_at: f64,
    last: Point,
    edge: Option<Edge>,
    pinch0: f64,
    scale: f64,
    /// Midpoint when the second finger went down, and at the last event.
    center0: Point,
    center: Point,
    vel: VelocityTracker,
    /// Time and place of the last tap that may still become a double-tap.
    last_tap: Option<(f64, Point)>,
    /// The current long-press is live (its finger may drag and lift).
    held: bool,
}

impl Default for GestureRecognizer {
    fn default() -> Self {
        Self::new(GestureConfig::default())
    }
}

fn dist(a: Point, b: Point) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

fn mid(a: Point, b: Point) -> Point {
    Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)
}

impl GestureRecognizer {
    /// A recogniser with these thresholds.
    pub fn new(cfg: GestureConfig) -> Self {
        Self {
            cfg,
            viewport: (0.0, 0.0),
            contacts: Vec::new(),
            phase: Phase::Idle,
            origin: Point::new(0.0, 0.0),
            down_at: 0.0,
            last: Point::new(0.0, 0.0),
            edge: None,
            pinch0: 1.0,
            scale: 1.0,
            center0: Point::new(0.0, 0.0),
            center: Point::new(0.0, 0.0),
            vel: VelocityTracker::default(),
            last_tap: None,
            held: false,
        }
    }

    /// Thresholds in use.
    pub fn config(&self) -> &GestureConfig {
        &self.cfg
    }

    /// The window's logical size (edge pans need it for the right and bottom edges).
    pub fn set_viewport(&mut self, w: f64, h: f64) {
        self.viewport = (w, h);
    }

    /// Current phase.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Contacts down.
    pub fn contacts(&self) -> usize {
        self.contacts.len()
    }

    /// The primary (first) contact id.
    pub fn primary(&self) -> Option<u64> {
        self.contacts.first().map(|c| c.0)
    }

    /// When [`Self::poll`] next has something to say (the long-press deadline).
    pub fn deadline(&self) -> Option<f64> {
        (self.phase == Phase::Pending).then_some(self.down_at + self.cfg.long_press)
    }

    fn edge_at(&self, p: Point) -> Option<Edge> {
        let e = self.cfg.edge;
        let (w, h) = self.viewport;
        if p.x <= e {
            Some(Edge::Left)
        } else if w > 0.0 && p.x >= w - e {
            Some(Edge::Right)
        } else if p.y <= e {
            Some(Edge::Top)
        } else if h > 0.0 && p.y >= h - e {
            Some(Edge::Bottom)
        } else {
            None
        }
    }

    /// A contact went down.
    pub fn start(&mut self, id: u64, pos: Point, t: f64) -> Vec<Gesture> {
        self.contacts.retain(|c| c.0 != id);
        self.contacts.push((id, pos));
        let mut out = Vec::new();
        match self.contacts.len() {
            1 => {
                self.phase = Phase::Pending;
                self.origin = pos;
                self.last = pos;
                self.down_at = t;
                self.edge = self.edge_at(pos);
                self.vel.reset();
                self.vel.add(t, pos, &self.cfg);
            }
            2 => {
                if let Phase::Pan(_) = self.phase {
                    out.push(Gesture::PanEnd { pos: self.last, velocity: (0.0, 0.0), edge: self.edge });
                }
                if self.held {
                    out.push(Gesture::LongPressEnd { pos: self.last });
                    self.held = false;
                }
                self.phase = Phase::Pinch;
                self.pinch0 = dist(self.contacts[0].1, self.contacts[1].1).max(1.0);
                self.scale = 1.0;
                self.center0 = mid(self.contacts[0].1, self.contacts[1].1);
                self.center = self.center0;
                self.last_tap = None;
            }
            _ => {}
        }
        out
    }

    /// The long-press check; call on every tick while a contact is down.
    pub fn poll(&mut self, now: f64) -> Option<Gesture> {
        if self.phase == Phase::Pending && now - self.down_at >= self.cfg.long_press {
            self.phase = Phase::LongPressed;
            self.held = true;
            self.last_tap = None;
            return Some(Gesture::LongPress { pos: self.last });
        }
        None
    }

    /// A contact moved.
    pub fn moved(&mut self, id: u64, pos: Point, t: f64) -> Vec<Gesture> {
        let Some(i) = self.contacts.iter().position(|c| c.0 == id) else { return Vec::new() };
        self.contacts[i].1 = pos;
        let mut out = Vec::new();
        if self.phase == Phase::Pinch {
            if self.contacts.len() >= 2 {
                let (a, b) = (self.contacts[0].1, self.contacts[1].1);
                self.scale = dist(a, b) / self.pinch0;
                let c = mid(a, b);
                out.push(Gesture::Pinch { center: c, scale: self.scale });
                let delta = (c.x - self.center.x, c.y - self.center.y);
                if delta != (0.0, 0.0) {
                    out.push(Gesture::PinchPan { center: c, delta, total: (c.x - self.center0.x, c.y - self.center0.y) });
                }
                self.center = c;
            }
            return out;
        }
        if i != 0 {
            return out;
        }
        self.vel.add(t, pos, &self.cfg);
        let (dx, dy) = (pos.x - self.origin.x, pos.y - self.origin.y);
        match self.phase {
            Phase::Pending => {
                if dx.hypot(dy) > self.cfg.slop {
                    // Too late to long-press; a pan of the stronger axis.
                    let axis = if dx.abs() > dy.abs() { Axis::Horizontal } else { Axis::Vertical };
                    self.phase = Phase::Pan(axis);
                    self.last_tap = None;
                    out.push(Gesture::PanStart { origin: self.origin, pos, axis, edge: self.edge });
                    self.last = pos;
                } else {
                    self.last = pos;
                }
            }
            Phase::Pan(_) => {
                let delta = (pos.x - self.last.x, pos.y - self.last.y);
                self.last = pos;
                out.push(Gesture::Pan { pos, delta, total: (dx, dy), edge: self.edge });
            }
            Phase::LongPressed if self.held => {
                let delta = (pos.x - self.last.x, pos.y - self.last.y);
                self.last = pos;
                if delta != (0.0, 0.0) {
                    out.push(Gesture::LongPressDrag { pos, delta });
                }
            }
            Phase::LongPressed | Phase::Idle | Phase::Pinch => self.last = pos,
        }
        out
    }

    /// A contact lifted.
    pub fn end(&mut self, id: u64, pos: Point, t: f64) -> Vec<Gesture> {
        let Some(i) = self.contacts.iter().position(|c| c.0 == id) else { return Vec::new() };
        self.contacts.remove(i);
        let mut out = Vec::new();
        match self.phase {
            Phase::Pinch => {
                if self.contacts.len() < 2 {
                    out.push(Gesture::PinchEnd { scale: self.scale });
                    // The finger left down neither taps nor pans; it waits to lift.
                    self.phase = if self.contacts.is_empty() { Phase::Idle } else { Phase::LongPressed };
                }
            }
            Phase::Pending if i == 0 => {
                out.push(Gesture::Tap { pos });
                match self.last_tap {
                    Some((t0, p0)) if t - t0 <= self.cfg.double_tap && dist(p0, pos) <= self.cfg.double_tap_slop => {
                        out.push(Gesture::DoubleTap { pos });
                        self.last_tap = None;
                    }
                    _ => self.last_tap = Some((t, pos)),
                }
                self.phase = Phase::Idle;
            }
            Phase::LongPressed if i == 0 && self.held => {
                out.push(Gesture::LongPressEnd { pos });
                self.held = false;
                self.phase = Phase::Idle;
            }
            Phase::Pan(axis) if i == 0 => {
                if pos != self.last {
                    self.vel.add(t, pos, &self.cfg);
                }
                let v = self.vel.velocity(t, &self.cfg);
                out.push(Gesture::PanEnd { pos, velocity: v, edge: self.edge });
                let (dx, dy) = (pos.x - self.origin.x, pos.y - self.origin.y);
                let (d, s, dir) = match axis {
                    Axis::Horizontal => (dx, v.0, if v.0 < 0.0 { SwipeDir::Left } else { SwipeDir::Right }),
                    Axis::Vertical => (dy, v.1, if v.1 < 0.0 { SwipeDir::Up } else { SwipeDir::Down }),
                };
                if d.abs() >= self.cfg.swipe_min_dist && s.abs() >= self.cfg.swipe_min_speed && d.signum() == s.signum() {
                    out.push(Gesture::Swipe { dir, edge: self.edge, speed: s.abs() });
                }
                self.phase = Phase::Idle;
            }
            _ => {
                if self.contacts.is_empty() {
                    self.phase = Phase::Idle;
                }
            }
        }
        if self.contacts.is_empty() {
            self.phase = Phase::Idle;
        }
        out
    }

    /// The OS took a contact away: no tap, no swipe.
    pub fn cancel(&mut self, id: u64) -> Vec<Gesture> {
        let before = self.contacts.len();
        self.contacts.retain(|c| c.0 != id);
        let mut out = Vec::new();
        if before != self.contacts.len() {
            match self.phase {
                Phase::Pinch if self.contacts.len() < 2 => out.push(Gesture::PinchEnd { scale: self.scale }),
                Phase::Pan(_) => out.push(Gesture::PanEnd { pos: self.last, velocity: (0.0, 0.0), edge: self.edge }),
                Phase::LongPressed if self.held => {
                    out.push(Gesture::LongPressEnd { pos: self.last });
                    self.held = false;
                }
                _ => {}
            }
            self.phase = if self.contacts.is_empty() { Phase::Idle } else { Phase::LongPressed };
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn small_motion_still_taps() {
        let mut g = GestureRecognizer::default();
        assert!(g.start(1, p(100.0, 100.0), 0.0).is_empty());
        assert!(g.moved(1, p(104.0, 103.0), 0.05).is_empty());
        assert_eq!(g.end(1, p(104.0, 103.0), 0.1), vec![Gesture::Tap { pos: p(104.0, 103.0) }]);
        assert_eq!(g.phase(), Phase::Idle);
    }

    #[test]
    fn leaving_the_slop_starts_a_pan_on_the_stronger_axis() {
        let mut g = GestureRecognizer::default();
        g.start(1, p(100.0, 300.0), 0.0);
        let out = g.moved(1, p(102.0, 290.0), 0.02);
        assert!(matches!(out[..], [Gesture::PanStart { axis: Axis::Vertical, edge: None, .. }]));
        let out = g.moved(1, p(102.0, 280.0), 0.04);
        assert_eq!(out, vec![Gesture::Pan { pos: p(102.0, 280.0), delta: (0.0, -10.0), total: (2.0, -20.0), edge: None }]);
        // No tap after a pan.
        let out = g.end(1, p(102.0, 280.0), 0.3);
        assert!(!out.iter().any(|e| matches!(e, Gesture::Tap { .. })));
    }

    #[test]
    fn holding_still_long_presses_and_suppresses_the_tap() {
        let mut g = GestureRecognizer::default();
        g.start(1, p(50.0, 50.0), 1.0);
        assert_eq!(g.deadline(), Some(1.5));
        assert_eq!(g.poll(1.3), None);
        assert_eq!(g.poll(1.5), Some(Gesture::LongPress { pos: p(50.0, 50.0) }));
        assert_eq!(g.poll(1.6), None, "fires once");
        let out = g.moved(1, p(80.0, 50.0), 1.7);
        assert_eq!(out, vec![Gesture::LongPressDrag { pos: p(80.0, 50.0), delta: (30.0, 0.0) }], "no pan after a long-press: the held finger drags");
        assert_eq!(g.end(1, p(80.0, 50.0), 1.8), vec![Gesture::LongPressEnd { pos: p(80.0, 50.0) }], "no tap after a long-press");
    }

    #[test]
    fn a_pan_cancels_the_long_press() {
        let mut g = GestureRecognizer::default();
        g.start(1, p(50.0, 50.0), 0.0);
        g.moved(1, p(50.0, 80.0), 0.1);
        assert_eq!(g.deadline(), None);
        assert_eq!(g.poll(1.0), None);
    }

    #[test]
    fn fast_horizontal_pan_from_the_left_edge_is_a_back_swipe() {
        let mut g = GestureRecognizer::default();
        g.set_viewport(390.0, 844.0);
        g.start(1, p(6.0, 400.0), 0.0);
        let mut t = 0.0;
        let mut x = 6.0;
        let mut first = None;
        for _ in 0..10 {
            t += 0.01;
            x += 12.0;
            for e in g.moved(1, p(x, 402.0), t) {
                first.get_or_insert(e);
            }
        }
        assert!(matches!(first, Some(Gesture::PanStart { axis: Axis::Horizontal, edge: Some(Edge::Left), .. })));
        let out = g.end(1, p(x, 402.0), t + 0.005);
        let swipe = out.iter().find_map(|e| match e {
            Gesture::Swipe { dir, edge, speed } => Some((*dir, *edge, *speed)),
            _ => None,
        });
        let (dir, edge, speed) = swipe.expect("a swipe");
        assert_eq!((dir, edge), (SwipeDir::Right, Some(Edge::Left)));
        assert!((speed - 1200.0).abs() < 1.0, "speed {speed}");
    }

    #[test]
    fn right_edge_is_found_with_a_viewport() {
        let mut g = GestureRecognizer::default();
        g.set_viewport(390.0, 844.0);
        g.start(1, p(385.0, 400.0), 0.0);
        let out = g.moved(1, p(360.0, 400.0), 0.02);
        assert!(matches!(out[..], [Gesture::PanStart { edge: Some(Edge::Right), .. }]));
    }

    #[test]
    fn slow_pan_is_not_a_swipe() {
        let mut g = GestureRecognizer::default();
        g.start(1, p(100.0, 100.0), 0.0);
        let mut t = 0.0;
        for i in 1..=50 {
            t += 0.02;
            g.moved(1, p(100.0 + i as f64 * 2.0, 100.0), t);
        }
        let out = g.end(1, p(200.0, 100.0), t + 0.01);
        assert!(!out.iter().any(|e| matches!(e, Gesture::Swipe { .. })), "{out:?}");
    }

    #[test]
    fn a_finger_that_rested_before_lifting_does_not_fling() {
        let cfg = GestureConfig::default();
        let mut v = VelocityTracker::default();
        for i in 0..10 {
            v.add(i as f64 * 0.01, p(0.0, i as f64 * 20.0), &cfg);
        }
        let moving = v.velocity(0.095, &cfg);
        assert!((moving.1 - 2000.0).abs() < 1.0, "{moving:?}");
        assert_eq!(v.velocity(0.3, &cfg), (0.0, 0.0));
    }

    #[test]
    fn velocity_is_capped() {
        let cfg = GestureConfig::default();
        let mut v = VelocityTracker::default();
        v.add(0.0, p(0.0, 0.0), &cfg);
        v.add(0.01, p(0.0, 1000.0), &cfg);
        let (_, vy) = v.velocity(0.01, &cfg);
        assert!((vy - cfg.fling_max_speed).abs() < 1e-6);
    }

    #[test]
    fn two_fingers_pinch() {
        let mut g = GestureRecognizer::default();
        g.start(1, p(100.0, 100.0), 0.0);
        g.start(2, p(200.0, 100.0), 0.01);
        assert_eq!(g.phase(), Phase::Pinch);
        let out = g.moved(2, p(300.0, 100.0), 0.02);
        assert_eq!(out[0], Gesture::Pinch { center: p(200.0, 100.0), scale: 2.0 });
        assert_eq!(g.end(2, p(300.0, 100.0), 0.03), vec![Gesture::PinchEnd { scale: 2.0 }]);
        assert!(g.end(1, p(100.0, 100.0), 0.04).is_empty(), "the remaining finger does not tap");
        assert_eq!(g.phase(), Phase::Idle);
    }

    #[test]
    fn fling_decelerates_to_its_total_distance_and_stops() {
        let cfg = GestureConfig::default();
        assert!(Fling::new((0.0, 100.0), 0.0, &cfg).is_none(), "below the threshold");
        let mut f = Fling::new((0.0, 2000.0), 0.0, &cfg).unwrap();
        let total = f.distance().1;
        let mut moved = 0.0;
        let mut t = 0.0;
        let mut last_step = f64::MAX;
        let mut steps = 0;
        while let Some((_, dy)) = f.step(t + 1.0 / 60.0) {
            t += 1.0 / 60.0;
            assert!(dy <= last_step + 1e-9, "steps shrink");
            last_step = dy;
            moved += dy;
            steps += 1;
            assert!(steps < 1000, "fling must end");
        }
        // Stops when speed < 20 px/s: the tail left is 20 / k = 10 px.
        assert!(moved < total && total - moved <= 10.5, "moved {moved} of {total}");
        assert!(moved > 900.0);
    }

    #[test]
    fn rubber_band_resists_and_is_bounded() {
        assert_eq!(rubber_band(0.0, 800.0), 0.0);
        let a = rubber_band(100.0, 800.0);
        let b = rubber_band(200.0, 800.0);
        assert!(a > 0.0 && a < 100.0);
        assert!(b > a && b - a < a, "diminishing");
        assert!(rubber_band(1e9, 800.0) < 800.0);
    }

    #[test]
    fn cancel_never_taps() {
        let mut g = GestureRecognizer::default();
        g.start(1, p(10.0, 10.0), 0.0);
        assert!(g.cancel(1).is_empty());
        assert_eq!(g.phase(), Phase::Idle);
        assert!(g.end(1, p(10.0, 10.0), 0.1).is_empty());
    }

    #[test]
    fn two_quick_taps_close_together_double_tap() {
        let mut g = GestureRecognizer::default();
        g.start(1, p(100.0, 100.0), 0.0);
        assert_eq!(g.end(1, p(100.0, 100.0), 0.05), vec![Gesture::Tap { pos: p(100.0, 100.0) }]);
        g.start(2, p(110.0, 104.0), 0.2);
        assert_eq!(g.end(2, p(110.0, 104.0), 0.25), vec![Gesture::Tap { pos: p(110.0, 104.0) }, Gesture::DoubleTap { pos: p(110.0, 104.0) }]);
        // A third tap starts a new pair: no triple.
        g.start(3, p(110.0, 104.0), 0.4);
        assert_eq!(g.end(3, p(110.0, 104.0), 0.45), vec![Gesture::Tap { pos: p(110.0, 104.0) }]);
    }

    #[test]
    fn slow_or_far_taps_stay_single() {
        let mut g = GestureRecognizer::default();
        g.start(1, p(100.0, 100.0), 0.0);
        g.end(1, p(100.0, 100.0), 0.05);
        g.start(2, p(100.0, 100.0), 0.5);
        assert_eq!(g.end(2, p(100.0, 100.0), 0.55).len(), 1, "too slow");
        g.start(3, p(200.0, 100.0), 0.6);
        assert_eq!(g.end(3, p(200.0, 100.0), 0.65).len(), 1, "too far");
        // A pan between taps breaks the pair.
        g.start(4, p(200.0, 100.0), 0.7);
        g.moved(4, p(200.0, 140.0), 0.72);
        g.end(4, p(200.0, 140.0), 0.74);
        g.start(5, p(200.0, 140.0), 0.8);
        assert_eq!(g.end(5, p(200.0, 140.0), 0.82).len(), 1, "pan in between");
    }

    #[test]
    fn long_press_drag_follows_the_finger_and_ends_on_lift() {
        let mut g = GestureRecognizer::default();
        g.start(1, p(50.0, 50.0), 0.0);
        g.poll(0.6);
        assert_eq!(g.moved(1, p(50.0, 50.0), 0.65), vec![], "no motion, no drag");
        assert_eq!(g.moved(1, p(60.0, 45.0), 0.7), vec![Gesture::LongPressDrag { pos: p(60.0, 45.0), delta: (10.0, -5.0) }]);
        assert_eq!(g.cancel(1), vec![Gesture::LongPressEnd { pos: p(60.0, 45.0) }], "a cancel ends it too");
        assert_eq!(g.phase(), Phase::Idle);
    }

    #[test]
    fn pinch_reports_scale_and_the_midpoint_pan() {
        let mut g = GestureRecognizer::default();
        g.start(1, p(100.0, 100.0), 0.0);
        g.start(2, p(200.0, 100.0), 0.01);
        assert_eq!(g.phase(), Phase::Pinch);
        // Spread to twice the distance, midpoint unchanged: scale only.
        assert_eq!(g.moved(2, p(250.0, 100.0), 0.02), vec![Gesture::Pinch { center: p(175.0, 100.0), scale: 1.5 }, Gesture::PinchPan { center: p(175.0, 100.0), delta: (25.0, 0.0), total: (25.0, 0.0) }]);
        // Both fingers move up 20 px: a two-finger pan at the same scale.
        g.moved(1, p(100.0, 80.0), 0.03);
        let out = g.moved(2, p(250.0, 80.0), 0.04);
        assert_eq!(out[0], Gesture::Pinch { center: p(175.0, 80.0), scale: 1.5 });
        assert_eq!(out[1], Gesture::PinchPan { center: p(175.0, 80.0), delta: (0.0, -10.0), total: (25.0, -20.0) });
        assert_eq!(g.end(2, p(250.0, 80.0), 0.05), vec![Gesture::PinchEnd { scale: 1.5 }]);
        // The finger left down neither taps nor long-press-drags.
        assert!(g.moved(1, p(120.0, 80.0), 0.06).is_empty());
        assert!(g.end(1, p(120.0, 80.0), 0.07).is_empty());
    }

    #[test]
    fn a_second_finger_ends_a_long_press() {
        let mut g = GestureRecognizer::default();
        g.start(1, p(50.0, 50.0), 0.0);
        g.poll(0.6);
        assert_eq!(g.start(2, p(150.0, 50.0), 0.7), vec![Gesture::LongPressEnd { pos: p(50.0, 50.0) }]);
        assert_eq!(g.phase(), Phase::Pinch);
    }
}
