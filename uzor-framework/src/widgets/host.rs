//! The paint door shared by [`PanelCx`] and [`OverlayCx`].

use uzor::render::RenderContext;
use uzor::tokens::Tokens;

use crate::handle::{FrameTime, OverlayCx, PanelCx, VisualView, Widgets};
use crate::types::spec::Spec;

/// A content hook context a widget convenience function can paint into.
///
/// Implemented for [`PanelCx`] and [`OverlayCx`]. The closure receives the
/// disjoint pieces of one frame (paint target, registration face, this
/// frame's input truth, tokens) so a widget can register and draw without
/// reaching an engine.
pub trait ContentCx<'a> {
    /// The app vocabulary of the context.
    type Spec: Spec;

    /// Run `f` against this frame's paint target and registration face.
    fn with_paint<R>(
        &mut self,
        f: impl FnOnce(&mut dyn RenderContext, &mut Widgets<'a>, &VisualView<Self::Spec>, &Tokens) -> R,
    ) -> R;

    /// Report the focused field's caret rect so the OS IME window opens there.
    fn ime_area(&mut self, _rect: uzor::Rect) {}

    /// Host clock in whole milliseconds. Same unit as
    /// `TextFieldStore::cursor_visible`.
    fn time_ms(&self) -> u64;

    /// The frame clock in milliseconds, unrounded: the stamp scroll containers share
    /// within one frame.
    fn frame_now_ms(&self) -> f64 {
        self.time_ms() as f64
    }

    /// Ask for another frame (an animation or a scroll is still moving).
    fn request_frame(&mut self) {}

    /// Painting into an overlay (a modal makes the page's scroll containers inert to keys).
    fn is_overlay(&self) -> bool {
        false
    }
}

impl<'a, S: Spec> ContentCx<'a> for PanelCx<'a, S> {
    type Spec = S;

    fn frame_now_ms(&self) -> f64 {
        self.time.now.0 * 1000.0
    }

    fn request_frame(&mut self) {
        self.out.request_frame();
    }

    fn is_overlay(&self) -> bool {
        false
    }

    fn ime_area(&mut self, rect: uzor::Rect) {
        self.out.ime_area(rect);
    }

    fn time_ms(&self) -> u64 {
        frame_ms(self.time)
    }

    fn with_paint<R>(
        &mut self,
        f: impl FnOnce(&mut dyn RenderContext, &mut Widgets<'a>, &VisualView<S>, &Tokens) -> R,
    ) -> R {
        let mut f = Some(f);
        with_paint_dyn(self.render, &mut self.widgets, self.view, self.tokens, &mut |r, w, v, tok| {
            f.take().expect("with_paint once")(r, w, v, tok)
        })
    }
}

impl<'a, S: Spec> ContentCx<'a> for OverlayCx<'a, S> {
    type Spec = S;

    fn frame_now_ms(&self) -> f64 {
        self.time.now.0 * 1000.0
    }

    fn request_frame(&mut self) {
        self.out.request_frame();
    }

    fn is_overlay(&self) -> bool {
        true
    }

    fn ime_area(&mut self, rect: uzor::Rect) {
        self.out.ime_area(rect);
    }

    fn time_ms(&self) -> u64 {
        frame_ms(self.time)
    }

    fn with_paint<R>(
        &mut self,
        f: impl FnOnce(&mut dyn RenderContext, &mut Widgets<'a>, &VisualView<S>, &Tokens) -> R,
    ) -> R {
        let mut f = Some(f);
        with_paint_dyn(self.render, &mut self.widgets, self.view, self.tokens, &mut |r, w, v, tok| {
            f.take().expect("with_paint once")(r, w, v, tok)
        })
    }
}

fn with_paint_dyn<'a, S: Spec, R>(
    render: &mut dyn RenderContext,
    widgets: &mut Widgets<'a>,
    view: &VisualView<S>,
    tokens: &Tokens,
    f: &mut dyn FnMut(&mut dyn RenderContext, &mut Widgets<'a>, &VisualView<S>, &Tokens) -> R,
) -> R {
    f(render, widgets, view, tokens)
}

fn frame_ms(time: FrameTime) -> u64 {
    (time.now.get() * 1000.0).round().max(0.0) as u64
}
