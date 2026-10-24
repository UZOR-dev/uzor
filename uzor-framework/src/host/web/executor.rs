//! Web `WindowCommand` executor (design §2.4, §5.4).
//!
//! The browser can do title, cursor, pointer lock, fullscreen, focus and
//! clipboard. Everything else is a no-op toward the kernel (single canvas,
//! no OS window chrome). The [`WebSurface`] trait is what unit tests record;
//! the live host applies it to the DOM.

use uzor::CursorIcon;

use crate::types::bus::{ClipboardResult, InputEvent};
use crate::types::ids::Ticket;
use crate::types::window::{CursorMode, FullscreenMode, SizePx, WindowCommand, WindowSpec};

/// Side-effects the DOM surface cannot finish inside the match.
#[derive(Clone, Debug, PartialEq)]
pub enum WebFollowUp {
    /// Write text to the clipboard (async `navigator.clipboard`).
    ClipboardWrite(String),
    /// Keep what the next `copy` / `cut` event writes (`ClipboardCmd::Offer`).
    ClipboardOffer(Option<String>),
    /// Read the clipboard; answer with this ticket.
    ClipboardRead(Ticket),
    /// Bind the single canvas to this window, or ignore if one is bound.
    Spawn(WindowSpec),
    /// Destroy the canvas window and stop the loop.
    Close,
    /// Stop the page loop.
    ExitApp,
    /// Echo of an outer-rect command. The page canvas cannot move; the host
    /// reports the requested rect so the kernel does not wait.
    OuterRectEcho {
        /// Requested outer position.
        position: (i32, i32),
        /// Requested outer size.
        size: SizePx,
    },
}

/// The slice of a browser window the executor can touch.
pub trait WebSurface {
    /// `document.title`.
    fn set_title(&mut self, title: &str);
    /// CSS cursor keyword for the canvas.
    fn set_cursor(&mut self, icon: CursorIcon);
    /// Show or hide the cursor (`cursor: none` while hidden).
    fn set_cursor_visible(&mut self, on: bool);
    /// Pointer lock / free cursor. Confined has no browser equivalent.
    fn set_cursor_mode(&mut self, mode: CursorMode);
    /// Fullscreen the canvas, or leave fullscreen.
    fn set_fullscreen(&mut self, mode: FullscreenMode);
    /// Focus the canvas.
    fn focus(&mut self);
    /// Ask the RAF driver for another frame.
    fn request_redraw(&mut self);
    /// A text field gained / lost focus: route DOM focus to the IME proxy.
    fn set_ime_allowed(&mut self, _on: bool) {}
    /// Caret rect (window-local CSS px) for the IME candidate window.
    fn set_ime_area(&mut self, _r: uzor::Rect) {}
}

/// Apply one [`WindowCommand`]. OS errors are not reported: the executor
/// never fails toward the kernel.
pub fn execute(window: &mut dyn WebSurface, cmd: WindowCommand) -> Option<WebFollowUp> {
    match cmd {
        WindowCommand::RequestRedraw => {
            window.request_redraw();
            None
        }
        WindowCommand::SetTitle(title) => {
            window.set_title(&title);
            None
        }
        WindowCommand::SetIcon(_)
        | WindowCommand::SetDecorations(_)
        | WindowCommand::SetCornerStyle(_)
        | WindowCommand::SetBorderColor(_)
        | WindowCommand::SetTheme(_)
        | WindowCommand::SetVisible(_)
        | WindowCommand::SetMinimized(_)
        | WindowCommand::SetMaximized(_)
        | WindowCommand::SetResizable(_)
        | WindowCommand::SetMinInnerSize(_)
        | WindowCommand::SetMaxInnerSize(_)
        | WindowCommand::RequestInnerSize(_)
        | WindowCommand::DragWindow
        | WindowCommand::DragResizeWindow(_)
        | WindowCommand::ShowWindowMenu { .. }
        | WindowCommand::SetCursorPosition(_)
        | WindowCommand::SetImePurpose(_)
        | WindowCommand::RequestAttention(_)
        | WindowCommand::Render(_)
        | WindowCommand::Screenshot(_) => None,
        WindowCommand::SetImeAllowed(on) => {
            window.set_ime_allowed(on);
            None
        }
        WindowCommand::SetImeCursorArea(r) => {
            window.set_ime_area(r);
            None
        }
        WindowCommand::SetFullscreen(mode) => {
            window.set_fullscreen(mode);
            None
        }
        WindowCommand::SetCursor(icon) => {
            window.set_cursor(icon);
            None
        }
        WindowCommand::SetCursorVisible(on) => {
            window.set_cursor_visible(on);
            None
        }
        WindowCommand::SetCursorMode(mode) => {
            window.set_cursor_mode(mode);
            None
        }
        WindowCommand::SetOuterRect { position, size } => {
            Some(WebFollowUp::OuterRectEcho { position, size })
        }
        WindowCommand::ClipboardWrite(text) => Some(WebFollowUp::ClipboardWrite(text)),
        // Image writes need the async Clipboard API; not wired on the web yet.
        WindowCommand::ClipboardWriteImage(_) => None,
        WindowCommand::ClipboardRead(ticket) => Some(WebFollowUp::ClipboardRead(ticket)),
        WindowCommand::ClipboardOffer(text) => Some(WebFollowUp::ClipboardOffer(text)),
        WindowCommand::FocusWindow => {
            window.focus();
            None
        }
        WindowCommand::Spawn(spec) => Some(WebFollowUp::Spawn(spec)),
        WindowCommand::Close => Some(WebFollowUp::Close),
        WindowCommand::ExitApp => Some(WebFollowUp::ExitApp),
    }
}

/// Where the DOM `paste` event's text goes: an app read waiting on the
/// clipboard (`ClipboardRead` with an app ticket) gets it as its answer;
/// otherwise it is the platform paste ([`InputEvent::Paste`]): the focused
/// text field, else the app's focused widget.
pub(super) fn paste_input(pending_app_read: Option<Ticket>, text: String) -> InputEvent {
    match pending_app_read {
        Some(ticket) => InputEvent::Clipboard(ClipboardResult { ticket, text: Some(text) }),
        None => InputEvent::Paste(text),
    }
}

/// A clipboard read the host performs with `navigator.clipboard.readText`.
/// Engine-minted reads (Ctrl/Cmd+V) are not: the browser's own `paste`
/// event, fired in the same gesture, answers them — no permission prompt,
/// no second insert.
pub(super) fn reads_async(ticket: Ticket) -> bool {
    ticket.0 < crate::engine::input::INPUT_TICKET_BASE
}

/// The `copy` / `cut` event's text: the focused widget's current offer.
/// Without one the event keeps the browser default (a text field's copy is
/// written by `ClipboardWrite` from the same key).
pub(super) fn copy_event_text(offer: Option<&str>) -> Option<&str> {
    offer.filter(|t| !t.is_empty())
}

/// Recording fake for executor unit tests.
#[cfg(test)]
#[derive(Default, Debug)]
struct RecordingSurface {
    /// Method log.
    pub log: Vec<String>,
}

#[cfg(test)]
impl WebSurface for RecordingSurface {
    fn set_title(&mut self, title: &str) {
        self.log.push(format!("set_title:{title}"));
    }
    fn set_cursor(&mut self, icon: CursorIcon) {
        self.log.push(format!("set_cursor:{}", icon.css_name()));
    }
    fn set_cursor_visible(&mut self, on: bool) {
        self.log.push(format!("set_cursor_visible:{on}"));
    }
    fn set_cursor_mode(&mut self, mode: CursorMode) {
        self.log.push(format!("set_cursor_mode:{mode:?}"));
    }
    fn set_fullscreen(&mut self, mode: FullscreenMode) {
        self.log.push(format!("set_fullscreen:{mode:?}"));
    }
    fn focus(&mut self) {
        self.log.push("focus".into());
    }
    fn request_redraw(&mut self) {
        self.log.push("request_redraw".into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ids::Ticket;

    #[test]
    fn dom_paste_maps_to_a_waiting_app_read_or_the_paste_event() {
        assert_eq!(paste_input(None, "hi".into()), InputEvent::Paste("hi".into()));
        assert_eq!(
            paste_input(Some(Ticket(3)), "hi".into()),
            InputEvent::Clipboard(ClipboardResult { ticket: Ticket(3), text: Some("hi".into()) })
        );
        // Ctrl/Cmd+V reads are answered by the paste event, app reads by readText.
        assert!(reads_async(Ticket(3)));
        assert!(!reads_async(Ticket(crate::engine::input::INPUT_TICKET_BASE + 1)));
        assert_eq!(copy_event_text(Some("sel")), Some("sel"));
        assert_eq!(copy_event_text(Some("")), None);
        assert_eq!(copy_event_text(None), None);
        let mut w = RecordingSurface::default();
        assert_eq!(
            execute(&mut w, WindowCommand::ClipboardOffer(Some("x".into()))),
            Some(WebFollowUp::ClipboardOffer(Some("x".into())))
        );
    }

    #[test]
    fn title_cursor_fullscreen_focus() {
        let mut w = RecordingSurface::default();
        assert!(execute(&mut w, WindowCommand::SetTitle("Uzor".into())).is_none());
        assert!(execute(&mut w, WindowCommand::SetCursor(CursorIcon::Text)).is_none());
        assert!(execute(
            &mut w,
            WindowCommand::SetFullscreen(FullscreenMode::Borderless)
        )
        .is_none());
        assert!(execute(&mut w, WindowCommand::FocusWindow).is_none());
        assert_eq!(
            w.log,
            [
                "set_title:Uzor",
                "set_cursor:text",
                "set_fullscreen:Borderless",
                "focus",
            ]
        );
    }

    #[test]
    fn clipboard_spawn_close_are_follow_ups() {
        let mut w = RecordingSurface::default();
        let ticket = Ticket(3);
        assert_eq!(
            execute(&mut w, WindowCommand::ClipboardRead(ticket)),
            Some(WebFollowUp::ClipboardRead(ticket))
        );
        assert_eq!(
            execute(&mut w, WindowCommand::ClipboardWrite("x".into())),
            Some(WebFollowUp::ClipboardWrite("x".into()))
        );
        let spec = WindowSpec::new("main", "Demo", SizePx::new(800, 600));
        assert!(matches!(
            execute(&mut w, WindowCommand::Spawn(spec)),
            Some(WebFollowUp::Spawn(_))
        ));
        assert_eq!(
            execute(&mut w, WindowCommand::Close),
            Some(WebFollowUp::Close)
        );
        assert_eq!(
            execute(&mut w, WindowCommand::ExitApp),
            Some(WebFollowUp::ExitApp)
        );
        assert!(w.log.is_empty());
    }

    #[test]
    fn outer_rect_echoes_and_icon_is_ignored() {
        let mut w = RecordingSurface::default();
        assert!(execute(&mut w, WindowCommand::SetIcon(None)).is_none());
        assert_eq!(
            execute(
                &mut w,
                WindowCommand::SetOuterRect {
                    position: (1, 2),
                    size: SizePx::new(30, 40),
                }
            ),
            Some(WebFollowUp::OuterRectEcho {
                position: (1, 2),
                size: SizePx::new(30, 40),
            })
        );
        assert!(w.log.is_empty());
    }

    #[test]
    fn cursor_mode_and_redraw() {
        let mut w = RecordingSurface::default();
        assert!(execute(
            &mut w,
            WindowCommand::SetCursorMode(CursorMode::LockedHidden)
        )
        .is_none());
        assert!(execute(&mut w, WindowCommand::SetCursorVisible(false)).is_none());
        assert!(execute(&mut w, WindowCommand::RequestRedraw).is_none());
        assert_eq!(
            w.log,
            [
                "set_cursor_mode:LockedHidden",
                "set_cursor_visible:false",
                "request_redraw",
            ]
        );
    }
}
