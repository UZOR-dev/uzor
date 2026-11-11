//! Phase 3 — Route (design §4.3): one pass per event, in arrival order,
//! per-kind precedence. The route phase owns the `RouteTrace` (§4.4): hop
//! names appended as the event crosses module boundaries, asserted
//! `len() <= 3` by the hop-count tests.

use uzor::CursorIcon;
use uzor::render::InvalidateBits;
use uzor::input::{KeyboardShortcut, MouseButton};

use crate::types::bus::{
    ClipboardResult, DropInput, ImeInput, InputEnvelope, InputEvent, KeyInput, KeyState,
    PointerInput, WheelInput,
};
use crate::types::command::LayoutHit;
use crate::types::ids::{Seconds, WindowId};
use crate::types::intent::{DropIntent, Intent, UnhandledInput};
use crate::types::ops::{
    Capture, DockPointer, EngineTarget, InputEffect, InputOp, Intercepted, KeyRoute, LayoutOp,
    OverlayEffect, OverlayOp, PointerRoute, PointerTarget, WindowOp,
};
use crate::types::spec::Spec;
use crate::types::window::WindowCommand;

use super::{Kernel, TouchMode};

impl<S: Spec> Kernel<S> {
    /// Phase 3 — route every non-lifecycle event of the tick, in order.
    /// Each event starts a fresh `RouteTrace`; the longest one is kept for
    /// the hop-count assertion (design §4.4).
    pub(super) fn route(&mut self, now: Seconds, inputs: &mut Vec<InputEnvelope>) {
        self.touch_tick(now);
        for env in inputs.drain(..) {
            self.trace.clear();
            let win = env.window;
            match &env.event {
                InputEvent::Touch(_) if !self.coarse_pointer => {
                    self.coarse_pointer = true;
                    self.invalidate(win, None, InvalidateBits::ALL);
                }
                InputEvent::Pointer(PointerInput::Moved { .. }) if self.coarse_pointer => {
                    self.coarse_pointer = false;
                    self.invalidate(win, None, InvalidateBits::ALL);
                }
                _ => {}
            }
            match env.event {
                InputEvent::Pointer(input) => self.route_pointer(now, win, input),
                InputEvent::Wheel(input) => self.route_wheel(now, win, input),
                InputEvent::Key(input) => self.route_key(now, win, input),
                InputEvent::Ime(input) => self.route_ime(now, win, input),
                InputEvent::Clipboard(result) => self.route_clipboard(now, win, result),
                InputEvent::Paste(text) => {
                    self.trace_push("route::paste");
                    let fx = self.input.apply(InputOp::Paste { win, now, text });
                    self.conduct_input(now, fx);
                }
                InputEvent::ClipboardImage(image) => {
                    self.outbox.intents.push(Intent::PasteImage(crate::types::intent::PastedImage { win, image }));
                }
                InputEvent::Drop(input) => self.route_drop(now, win, input),
                // Lifecycle and host facts are phase 2's; the wheel drains
                // by clock in phase 4a regardless of the wake echo.
                InputEvent::Window(_) | InputEvent::Host(_) | InputEvent::Timer(_) => {}
                InputEvent::Touch(t) => self.route_touch(now, win, t),
            }
            self.trace_max = self.trace_max.max(self.trace.len());
        }
    }

    /// Touch (design: one recogniser, platform feel). The first finger presses at
    /// once (pressed look, a tap clicks on lift). When it leaves the 8 px slop:
    /// - on a widget that captured the drag (slider, splitter) or a layout zone, it
    ///   keeps dragging the pointer;
    /// - on a text field it scrolls like anywhere else (the selection drag ends);
    /// - anywhere else the press is cancelled (no click) and the motion scrolls
    ///   1:1 as pixel wheel steps marked as touch; on lift a fast finger flings
    ///   (`types::gesture::Fling`, stepped every tick in `touch_tick`).
    /// Holding still for 0.5 s is a long-press: the press is cancelled and a
    /// secondary click is delivered (context menus open as on a right click).
    /// A second finger cancels the press; the fingers' mean motion scrolls and
    /// the pinch is reported. Every recognised gesture (tap, long-press, pan,
    /// edge pan, swipe, pinch) is in `VisualView::gestures` for this tick.
    fn route_touch(&mut self, now: Seconds, win: WindowId, t: crate::types::bus::TouchInput) {
        use crate::types::bus::TouchInput as T;
        use crate::types::gesture::Gesture;
        let mods = uzor::input::ModifierKeys::default();
        let mean = |v: &[(u64, crate::types::window::Point)]| {
            let n = v.len().max(1) as f64;
            crate::types::window::Point::new(v.iter().map(|c| c.1.x).sum::<f64>() / n, v.iter().map(|c| c.1.y).sum::<f64>() / n)
        };
        let ts = now.get();
        match t {
            T::Start { id, pos } => {
                if self.touches.is_empty() {
                    // A finger on a flinging list stops it (and that touch does not click).
                    let stopped = self.fling.take().is_some();
                    if let Some(vp) = self.windows.view().window(win).map(|w| w.geometry().viewport) {
                        self.gesture.set_viewport(vp.x + vp.width, vp.y + vp.height);
                    }
                    self.touch_win = Some(win);
                    let g = self.gesture.start(id, pos, ts);
                    self.touches.push((id, pos));
                    self.push_gestures(win, g);
                    self.route_pointer(now, win, PointerInput::Moved { pos, mods });
                    if stopped {
                        self.touch_mode = TouchMode::Held;
                    } else {
                        self.touch_mode = TouchMode::Press;
                        self.route_pointer(now, win, PointerInput::Down { pos, button: MouseButton::Left, mods });
                    }
                } else {
                    let g = self.gesture.start(id, pos, ts);
                    self.touches.push((id, pos));
                    self.push_gestures(win, g);
                    if self.touches.len() == 2 {
                        // A pan, not a drag: release the primary without a click.
                        if matches!(self.touch_mode, TouchMode::Press | TouchMode::Drag) {
                            self.route_pointer(now, win, PointerInput::Cancelled);
                        }
                        self.touch_mode = TouchMode::Multi;
                    }
                }
            }
            T::Move { id, pos } => {
                let before = mean(&self.touches);
                let primary = self.touches.first().map(|c| c.0) == Some(id);
                let prev = self.touches.iter().find(|c| c.0 == id).map(|c| c.1);
                if let Some(c) = self.touches.iter_mut().find(|c| c.0 == id) {
                    c.1 = pos;
                }
                let g = self.gesture.moved(id, pos, ts);
                let pan_started = g.iter().any(|e| matches!(e, Gesture::PanStart { .. }));
                self.push_gestures(win, g);
                match self.touch_mode {
                    TouchMode::Multi => {
                        let after = mean(&self.touches);
                        let (dx, dy) = (after.x - before.x, after.y - before.y);
                        if dx != 0.0 || dy != 0.0 {
                            self.touch_scroll(now, win, after, dx, dy);
                        }
                    }
                    TouchMode::Press if primary => {
                        // Inside the slop the press follows the finger (drag-sensitive
                        // widgets arm and may capture here).
                        self.route_pointer(now, win, PointerInput::Moved { pos, mods });
                        if pan_started {
                            let captured = self.input.view().capture(win).is_some();
                            let text_drag = self
                                .input
                                .view()
                                .coordinator(win)
                                .is_some_and(|c| c.text_fields().drag_active());
                            // A finger dragging over a text field scrolls (as on a
                            // phone: selection is the long-press's), it does not select.
                            if captured && !text_drag {
                                self.touch_mode = TouchMode::Drag;
                            } else {
                                self.route_pointer(now, win, PointerInput::Cancelled);
                                self.touch_mode = TouchMode::Scroll;
                            }
                        }
                    }
                    TouchMode::Scroll if primary => {
                        if let Some(p) = prev {
                            let (dx, dy) = (pos.x - p.x, pos.y - p.y);
                            if dx != 0.0 || dy != 0.0 {
                                self.touch_scroll(now, win, pos, dx, dy);
                            }
                        }
                    }
                    TouchMode::Drag if primary => {
                        self.route_pointer(now, win, PointerInput::Moved { pos, mods });
                    }
                    _ => {}
                }
            }
            T::End { id, pos } => {
                let primary = self.touches.first().map(|c| c.0) == Some(id);
                self.touches.retain(|c| c.0 != id);
                let g = self.gesture.end(id, pos, ts);
                let release = g.iter().find_map(|e| match e {
                    Gesture::PanEnd { velocity, .. } => Some(*velocity),
                    _ => None,
                });
                self.push_gestures(win, g);
                match self.touch_mode {
                    TouchMode::Press | TouchMode::Drag if primary => {
                        self.route_pointer(now, win, PointerInput::Moved { pos, mods });
                        self.route_pointer(now, win, PointerInput::Up { pos, button: MouseButton::Left, mods });
                    }
                    TouchMode::Scroll if primary => {
                        if let Some(v) = release {
                            self.fling = crate::types::gesture::Fling::new(v, ts, self.gesture.config()).map(|f| (win, pos, f));
                        }
                    }
                    _ => {}
                }
                if self.touches.is_empty() {
                    self.touch_mode = TouchMode::Idle;
                    self.touch_win = None;
                }
            }
            T::Cancel { id } => {
                self.touches.retain(|c| c.0 != id);
                let g = self.gesture.cancel(id);
                self.push_gestures(win, g);
                if matches!(self.touch_mode, TouchMode::Press | TouchMode::Drag) {
                    self.route_pointer(now, win, PointerInput::Cancelled);
                }
                if self.touches.is_empty() {
                    self.touch_mode = TouchMode::Idle;
                    self.touch_win = None;
                } else {
                    self.touch_mode = TouchMode::Held;
                }
            }
        }
    }

    /// Clock-driven touch work, run at the start of every route phase: the
    /// long-press deadline and one fling step.
    pub(super) fn touch_tick(&mut self, now: Seconds) {
        // Settle passes of the same tick re-run the route phase with the same
        // clock: the fling steps once per tick.
        if self.touch_clock == Some(now.get()) {
            return;
        }
        self.touch_clock = Some(now.get());
        if let (Some(win), Some(g)) = (self.touch_win, self.gesture.poll(now.get())) {
            self.push_gestures(win, vec![g]);
            if self.touch_mode == TouchMode::Press {
                // A long-press is the touch context request: drop the press (no
                // click) and report a context point where the finger rests, as a
                // secondary click does, so context menus open.
                let pos = match g {
                    crate::types::gesture::Gesture::LongPress { pos } => pos,
                    _ => return,
                };
                self.route_pointer(now, win, PointerInput::Cancelled);
                let fx = self.input.apply(InputOp::Context { win, pos });
                self.conduct_input(now, fx);
                self.invalidate(win, None, InvalidateBits::MATERIAL);
                self.touch_mode = TouchMode::Held;
            }
        }
        if let Some((win, pos, mut f)) = self.fling.take() {
            if let Some((dx, dy)) = f.step(now.get()) {
                if dx != 0.0 || dy != 0.0 {
                    self.touch_scroll(now, win, pos, dx, dy);
                }
                self.fling = Some((win, pos, f));
            }
        }
    }

    /// One finger-driven scroll step: content follows the finger (finger down =
    /// content down = positive wheel y, winit's sign).
    fn touch_scroll(&mut self, now: Seconds, win: WindowId, pos: crate::types::window::Point, dx: f64, dy: f64) {
        use crate::types::bus::{WheelDelta, WheelInput};
        self.touch_wheel = true;
        self.route_wheel(now, win, WheelInput { pos, delta: WheelDelta::Pixels { x: dx, y: dy }, mods: Default::default() });
        self.touch_wheel = false;
    }

    fn push_gestures(&mut self, win: WindowId, g: Vec<crate::types::gesture::Gesture>) {
        if g.is_empty() {
            return;
        }
        match self.gestures.get_mut(&win) {
            Some(v) => v.extend(g),
            None => {
                self.gestures.insert(win, g);
            }
        }
        self.invalidate(win, None, InvalidateBits::MATERIAL);
    }

    /// Pointer precedence (§4.3): cook -> capture -> overlay intercept ->
    /// layout zones -> content -> unhandled.
    fn route_pointer(&mut self, now: Seconds, win: WindowId, input: PointerInput) {
        self.trace_push("route::pointer");
        #[cfg(feature = "inspector")]
        if self.inspector.enabled {
            // The overlay follows the pointer: repaint on every move.
            self.invalidate(win, None, InvalidateBits::ALL);
        }
        // Step 1: cook (click count, drag arm).
        let fx = self.input.apply(InputOp::Pointer { win, now, input });
        let Some(cooked) = fx.iter().find_map(|e| match e {
            InputEffect::Pointer(c) => Some(c.clone()),
            _ => None,
        }) else {
            self.conduct_input(now, fx);
            return;
        };
        self.conduct_input(
            now,
            fx.into_iter()
                .filter(|e| !matches!(e, InputEffect::Pointer(_)))
                .collect(),
        );

        // Step 2: a capture owns the event regardless of position.
        match cooked.target.clone() {
            PointerTarget::Captured(Capture::Widget(_)) => {
                self.trace_push("capture:widget");
                self.content_cursor_at(now, win, input);
                let fx = self.input.apply(InputOp::Deliver { win, input });
                self.conduct_routed_content(now, win, input, fx);
                return;
            }
            PointerTarget::Captured(Capture::Engine(EngineTarget::Layout)) => {
                self.trace_push("capture:layout");
                if let Some(event) = dock_pointer(input) {
                    let fx = self.layout.apply(LayoutOp::Pointer { win, now, event });
                    self.conduct_layout(now, fx);
                }
                return;
            }
            PointerTarget::Captured(Capture::Engine(EngineTarget::Overlay(slot))) => {
                self.trace_push("capture:overlay");
                let fx = self
                    .overlays
                    .apply(OverlayOp::Captured { win, slot, input });
                self.conduct_overlay(now, fx);
                return;
            }
            PointerTarget::HitTest => {}
        }

        // Step 3: overlay intercept (topmost first; the modal shield and
        // outside-dismiss live in the engine).
        let fx = self.overlays.apply(OverlayOp::Intercept {
            win,
            event: Intercepted::Pointer(input),
        });
        let route = fx.iter().find_map(|e| match e {
            OverlayEffect::Pointer { route, .. } => Some(*route),
            _ => None,
        });
        self.conduct_overlay(
            now,
            fx.into_iter()
                .filter(|e| !matches!(e, OverlayEffect::Pointer { .. } | OverlayEffect::Key { .. }))
                .collect(),
        );
        match route {
            Some(PointerRoute::Consumed) => {
                self.trace_push("overlay:consumed");
                self.content_cursor_at(now, win, input);
                return;
            }
            Some(PointerRoute::Overlay { .. }) => {
                self.trace_push("overlay:body");
                self.content_cursor_at(now, win, input);
                let fx = self.input.apply(InputOp::Deliver { win, input });
                self.conduct_routed_content(now, win, input, fx);
                return;
            }
            Some(PointerRoute::Pass) | None => {}
        }

        // Step 4: window-level zones. A claimed press starts a layout
        // session (and may set the capture); hover over a zone sets the
        // suggested cursor.
        if let Some(pos) = pointer_pos(input) {
            let mut hit = self.layout.hit(win, pos);
            let floating = self
                .layout
                .view()
                .window(win)
                .and_then(|wv| wv.chrome_model().floating.clone());
            if let Some(fc) = floating {
                // The top drag band yields to content widgets under the pointer
                // (the app's own buttons there stay clickable).
                if hit == LayoutHit::Chrome(uzor::widgets::composite::chrome::ChromeHit::Drag)
                    && self
                        .input
                        .view()
                        .coordinator(win)
                        .and_then(|c| c.hit_test_now(pos.x, pos.y))
                        .is_some()
                {
                    hit = LayoutHit::None;
                }
                // Button hover paints without waiting for other input.
                let vp = self.windows.view().window(win).map(|w| w.geometry().viewport);
                if vp.is_some_and(|vp| pos.y < vp.y + fc.style.button_h.max(fc.style.dot_inset * 2.0 + fc.style.dot) + 4.0) {
                    self.invalidate(win, None, InvalidateBits::ALL);
                }
            }
            self.update_cursor(now, win, &hit, pos);
            let primary_down = matches!(
                input,
                PointerInput::Down {
                    button: MouseButton::Left,
                    ..
                }
            );
            // A press on a zone the layout engine does not claim (panel
            // body, gutter, empty dock) falls through to content.
            if primary_down && hit.claims_press() {
                self.trace_push("layout:zone");
                let fx = self.layout.apply(LayoutOp::Pointer {
                    win,
                    now,
                    event: DockPointer::Down {
                        pos,
                        button: MouseButton::Left,
                    },
                });
                self.conduct_layout(now, fx);
                return;
            }
        }

        // Step 5: content widgets (last frame's registrations).
        self.trace_push("content");
        let fx = self.input.apply(InputOp::Deliver { win, input });
        self.conduct_routed_content(now, win, input, fx);
    }

    /// Content delivery answer: `None` on a press is routing step 6
    /// (reported, never dropped silently); anything else is quiet
    /// (a `Moved` over nothing is not an event worth an intent).
    fn conduct_routed_content(
        &mut self,
        now: Seconds,
        win: WindowId,
        input: PointerInput,
        fx: crate::engine::input::InputEffects,
    ) {
        let target = fx.iter().find_map(|e| match e {
            InputEffect::Content { target, .. } => Some(target.clone()),
            _ => None,
        });
        // Hover moved, or a wheel delta waits in the hook view (scroll
        // containers read it at compose): content hooks must see it now,
        // also on `TickRate::Dirty` windows.
        let _ = &input;
        self.invalidate(win, None, InvalidateBits::MATERIAL);
        self.conduct_input(
            now,
            fx.into_iter()
                .filter(|e| !matches!(e, InputEffect::Content { .. }))
                .collect(),
        );
        // `Some(None)` = the engine answered "delivered to no widget"
        // (routing step 6); a missing Content effect is not a report.
        if matches!(target, Some(None)) && matches!(input, PointerInput::Down { .. }) {
            self.outbox
                .intents
                .push(Intent::Unhandled(UnhandledInput::Pointer {
                    win,
                    pointer: input,
                }));
        }
    }

    /// Wheel precedence (§4.3): capture -> overlay (scrollable body) ->
    /// content. The layout panel-scroll row of the design has no engine
    /// door yet (`DockPointer` carries no wheel): recorded gap.
    fn route_wheel(&mut self, now: Seconds, win: WindowId, input: WheelInput) {
        self.trace_push("route::wheel");
        match self.input.view().capture(win).cloned() {
            Some(Capture::Widget(_)) => {
                self.trace_push("capture:widget");
                let fx = self.input.apply(InputOp::Wheel { win, input, touch: self.touch_wheel });
                self.conduct_wheel_content(now, win, input, fx);
            }
            Some(Capture::Engine(EngineTarget::Layout)) => {
                // A layout drag ignores the wheel.
                self.trace_push("capture:layout");
            }
            Some(Capture::Engine(EngineTarget::Overlay(_))) | None => {
                let fx = self.overlays.apply(OverlayOp::Intercept {
                    win,
                    event: Intercepted::Wheel(input),
                });
                let route = fx.iter().find_map(|e| match e {
                    OverlayEffect::Pointer { route, .. } => Some(*route),
                    _ => None,
                });
                self.conduct_overlay(
                    now,
                    fx.into_iter()
                        .filter(|e| {
                            !matches!(e, OverlayEffect::Pointer { .. } | OverlayEffect::Key { .. })
                        })
                        .collect(),
                );
                match route {
                    Some(PointerRoute::Consumed) => self.trace_push("overlay:consumed"),
                    Some(PointerRoute::Overlay { .. }) | Some(PointerRoute::Pass) | None => {
                        self.trace_push("content");
                        let fx = self.input.apply(InputOp::Wheel { win, input, touch: self.touch_wheel });
                        self.conduct_wheel_content(now, win, input, fx);
                    }
                }
            }
        }
    }

    fn conduct_wheel_content(
        &mut self,
        now: Seconds,
        win: WindowId,
        input: WheelInput,
        fx: crate::engine::input::InputEffects,
    ) {
        let target = fx.iter().find_map(|e| match e {
            InputEffect::Content { target, .. } => Some(target.clone()),
            _ => None,
        });
        // Hover moved, or a wheel delta waits in the hook view (scroll
        // containers read it at compose): content hooks must see it now,
        // also on `TickRate::Dirty` windows.
        let _ = &input;
        self.invalidate(win, None, InvalidateBits::MATERIAL);
        self.conduct_input(
            now,
            fx.into_iter()
                .filter(|e| !matches!(e, InputEffect::Content { .. }))
                .collect(),
        );
        if matches!(target, Some(None)) {
            self.outbox
                .intents
                .push(Intent::Unhandled(UnhandledInput::Wheel {
                    win,
                    wheel: input,
                }));
        }
    }

    /// Key precedence (§4.3): text / IME and Tab -> overlay Escape ->
    /// keymap -> unhandled. Releases that pass text and overlays are
    /// dropped quietly (a release is not a binding).
    fn route_key(&mut self, now: Seconds, win: WindowId, input: KeyInput) {
        self.trace_push("route::key");
        // F12 toggles the inspector, Shift+F12 pins the box under the pointer.
        #[cfg(feature = "inspector")]
        if input.code == uzor::input::KeyCode::F12 {
            if input.state == KeyState::Down {
                if input.mods.shift {
                    self.inspector.pin_pointer();
                } else {
                    self.inspector.toggle();
                }
                self.invalidate(win, None, InvalidateBits::ALL);
            }
            return;
        }
        // Step 1: the focused text field's editing chords and text, and
        // Tab / Shift+Tab focus cycling.
        let focus_before = self.input.view().window(win).map(|v| (v.focused.clone(), v.focus_visible));
        let fx = self.input.apply(InputOp::Key {
            win,
            now,
            input: input.clone(),
        });
        // A Tab that moved focus repaints now (focus ring, active descendant),
        // not on the next unrelated input.
        if self.input.view().window(win).map(|v| (v.focused.clone(), v.focus_visible)) != focus_before {
            self.invalidate(win, None, InvalidateBits::ALL);
        }
        let passed = fx.iter().find_map(|e| match e {
            InputEffect::KeyPassed { key, .. } => Some(key.clone()),
            _ => None,
        });
        self.conduct_input(
            now,
            fx.into_iter()
                .filter(|e| !matches!(e, InputEffect::KeyPassed { .. }))
                .collect(),
        );
        let Some(key) = passed else {
            self.trace_push("text");
            return;
        };

        // Step 2: overlay intercept (Escape closes the top overlay that
        // wants it; the verdict also carries the keymap context).
        let fx = self.overlays.apply(OverlayOp::Intercept {
            win,
            event: Intercepted::Key(key.clone()),
        });
        let route = fx.iter().find_map(|e| match e {
            OverlayEffect::Key { route, .. } => Some(*route),
            _ => None,
        });
        self.conduct_overlay(
            now,
            fx.into_iter()
                .filter(|e| !matches!(e, OverlayEffect::Pointer { .. } | OverlayEffect::Key { .. }))
                .collect(),
        );
        let (top_overlay, modal_open) = match route {
            Some(KeyRoute::Consumed) => {
                self.trace_push("overlay:escape");
                return;
            }
            Some(KeyRoute::Pass {
                top_overlay,
                modal_open,
            }) => (top_overlay, modal_open),
            None => (None, false),
        };

        // Step 3: keymap (focused > overlay > global; globals shielded by
        // an open modal unless the binding opts out). Presses and repeats
        // only.
        if key.state != KeyState::Up {
            let chord = KeyboardShortcut::new(key.mods, key.code);
            let focused = self.input.view().focused(win).cloned();
            if let Some(action) =
                self.keymap
                    .resolve(&chord, focused.as_ref(), top_overlay.as_ref(), modal_open)
            {
                self.trace_push("keymap");
                self.outbox.intents.push(Intent::Action(action));
                return;
            }
            // Step 4 (the focused widget's raw key) has no widget-facing
            // door: the driver method is the input engine's alone (ban
            // F7). What passes everything is reported.
            self.outbox
                .intents
                .push(Intent::Unhandled(UnhandledInput::Key { win, key }));
        }
    }

    /// IME events go to the focused text field (through the engine; the
    /// preedit never enters a buffer before commit).
    fn route_ime(&mut self, now: Seconds, win: WindowId, input: ImeInput) {
        self.trace_push("route::ime");
        let fx = self.input.apply(InputOp::Ime { win, now, input });
        self.conduct_input(now, fx);
    }

    /// The host's clipboard answer returns to the engine that asked.
    fn route_clipboard(&mut self, now: Seconds, win: WindowId, result: ClipboardResult) {
        self.trace_push("route::clipboard");
        let fx = self.input.apply(InputOp::Clipboard { win, now, result });
        self.conduct_input(now, fx);
    }

    /// A dropped file targets the widget under the pointer (design §4.3:
    /// "target by `Sense::drop` from the coordinator" — the bus's
    /// `DropInput` carries no position, so the target is the hit at the
    /// last known pointer position; sense filtering lands with the touch /
    /// gesture brief). No target, no intent: a drop on empty space is not
    /// an error.
    fn route_drop(&mut self, now: Seconds, win: WindowId, input: DropInput) {
        let _ = now;
        self.trace_push("route::drop");
        let file = match input {
            DropInput::Dropped(f) => f,
            DropInput::Hovered(file) => {
                self.outbox.intents.push(Intent::Drop(DropIntent::Hovering { win, file }));
                return;
            }
            DropInput::Cancelled => {
                self.outbox.intents.push(Intent::Drop(DropIntent::Left { win }));
                return;
            }
        };
        let pos = self.input.view().coordinator(win).and_then(|c| c.pointer_pos());
        let target = pos
            .and_then(|(x, y)| {
                self.input
                    .view()
                    .coordinator(win)
                    .and_then(|c| c.hit_test_now(x, y))
            });
        match target {
            Some(target) => self.outbox.intents.push(Intent::Drop(DropIntent::Files { win, target, files: vec![file], pos })),
            None => self.outbox.intents.push(Intent::Drop(DropIntent::Window { win, files: vec![file], pos })),
        }
    }

    /// The cursor a layout hit suggests, applied when it changes (design
    /// §3.8's cursor row; the engine-only `CursorChanged` effect does not
    /// exist, so the route phase maps `LayoutHit::cursor` itself).
    /// The cursor over content when no layout zone is consulted: an
    /// overlay (modal, menu, dropdown) or a widget capture owns the event,
    /// and the widget under the pointer (or the captured one) still
    /// declares the cursor, the arrow when it declares none. Without this
    /// the cursor froze at its pre-overlay value (the hand of the button
    /// that opened the modal stayed over the whole modal).
    fn content_cursor_at(&mut self, now: Seconds, win: WindowId, input: PointerInput) {
        if let Some(pos) = pointer_pos(input) {
            self.update_cursor(now, win, &LayoutHit::None, pos);
        }
    }

    fn update_cursor(&mut self, now: Seconds, win: WindowId, hit: &LayoutHit, pos: crate::types::window::Point) {
        // Layout zones decide first; over content the widget under the pointer does.
        let icon = match hit.cursor() {
            Some(i) => i,
            None => self.input.view().coordinator(win).map_or(CursorIcon::Default, |c| content_cursor(c, pos)),
        };
        let current = self.windows.view().window(win).map(|w| w.cursor());
        if current != Some(icon) {
            let fx = self.windows.apply(WindowOp::Enqueue {
                win,
                cmd: WindowCommand::SetCursor(icon),
            });
            self.conduct_window_all(now, fx);
        }
    }

    fn trace_push(&mut self, hop: &'static str) {
        self.trace.push(hop);
    }
}

/// The cursor a content widget implies, resolved like the DOM's `cursor` property: the
/// pointer-captured widget keeps its own for the whole drag, else the topmost widget under
/// the modal barrier declares it (its own `set_widget_cursor`, a text field's I-beam, else
/// its role: controls and links the hand, splitters resize, text the I-beam, backdrops,
/// surfaces and everything else the arrow).
pub(crate) fn content_cursor(c: &uzor::input::InputCoordinator, pos: crate::types::window::Point) -> CursorIcon {
    c.cursor_at(pos.x, pos.y)
}

/// The position a pointer event carries, if it is a positioned one.
fn pointer_pos(input: PointerInput) -> Option<crate::types::window::Point> {
    match input {
        PointerInput::Moved { pos, .. }
        | PointerInput::Down { pos, .. }
        | PointerInput::Up { pos, .. } => Some(pos),
        PointerInput::Entered
        | PointerInput::Left
        | PointerInput::Cancelled
        | PointerInput::RawDelta { .. } => None,
    }
}

/// The layout-session view of a pointer event (`Entered` / `Left` /
/// `RawDelta` are not layout gestures; `Cancelled` maps to `Cancel`).
fn dock_pointer(input: PointerInput) -> Option<DockPointer> {
    match input {
        PointerInput::Down { pos, button, .. } => Some(DockPointer::Down { pos, button }),
        PointerInput::Moved { pos, .. } => Some(DockPointer::Move(pos)),
        PointerInput::Up { pos, .. } => Some(DockPointer::Up(pos)),
        PointerInput::Cancelled => Some(DockPointer::Cancel),
        PointerInput::Entered | PointerInput::Left | PointerInput::RawDelta { .. } => None,
    }
}
