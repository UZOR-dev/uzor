//! Web IME proxy. A canvas is not editable, so the browser never starts an
//! input-method composition on it. While a text field is focused
//! (`SetImeAllowed(true)`) DOM focus moves to one hidden `<textarea>` placed at
//! the caret (`SetImeCursorArea`), so the candidate window opens there; its
//! key and composition events bubble to the window listeners as before. The
//! textarea is emptied after every input: the text store owns the text.

use std::cell::Cell;
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Document, Event, HtmlCanvasElement, HtmlElement};

use uzor::Rect;

const ID: &str = "uzor-ime";

/// The hidden textarea and whether a field wants the IME.
pub(super) struct Ime {
    el: HtmlElement,
    canvas: HtmlCanvasElement,
    on: Rc<Cell<bool>>,
}

impl Ime {
    /// Insert the textarea after the canvas and wire its listeners. `None`
    /// when the DOM refuses (the canvas then keeps focus, as before).
    pub(super) fn install(document: &Document, canvas: &HtmlCanvasElement) -> Option<Self> {
        canvas
            .insert_adjacent_html(
                "afterend",
                &format!(
                    "<textarea id=\"{ID}\" aria-hidden=\"true\" tabindex=\"-1\" autocomplete=\"off\" spellcheck=\"false\" style=\"position:fixed;left:0;top:0;width:1px;height:16px;opacity:0;border:0;padding:0;margin:0;resize:none;overflow:hidden;pointer-events:none;caret-color:transparent\"></textarea>"
                ),
            )
            .ok()?;
        let el: HtmlElement = document.get_element_by_id(ID)?.dyn_into().ok()?;
        let on = Rc::new(Cell::new(false));
        // A click on the canvas takes DOM focus; while a field is focused, hand
        // it straight back so the next keystroke can compose.
        let (bounce_el, bounce_on) = (el.clone(), Rc::clone(&on));
        let bounce = Closure::wrap(Box::new(move |_: Event| {
            if bounce_on.get() {
                let _ = bounce_el.focus();
            }
        }) as Box<dyn FnMut(Event)>);
        let _ = canvas.add_event_listener_with_callback("focus", bounce.as_ref().unchecked_ref());
        bounce.forget();
        // Keep the textarea empty outside a composition.
        let clear_el = el.clone();
        let clear = Closure::wrap(Box::new(move |ev: Event| {
            let composing = js_sys::Reflect::get(ev.as_ref(), &JsValue::from_str("isComposing"))
                .ok()
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !composing {
                let _ = js_sys::Reflect::set(clear_el.as_ref(), &JsValue::from_str("value"), &JsValue::from_str(""));
            }
        }) as Box<dyn FnMut(Event)>);
        for ty in ["input", "compositionend"] {
            let _ = el.add_event_listener_with_callback(ty, clear.as_ref().unchecked_ref());
        }
        clear.forget();
        Some(Self { el, canvas: canvas.clone(), on })
    }

    /// A text field gained (`true`) or lost focus.
    pub(super) fn set_allowed(&self, on: bool) {
        self.on.set(on);
        let target: &HtmlElement = if on { &self.el } else { self.canvas.unchecked_ref() };
        let _ = target.focus();
    }

    /// Move the textarea over the caret (window-local CSS px).
    pub(super) fn set_area(&self, r: Rect) {
        let b = self.canvas.get_bounding_client_rect();
        let s = self.el.style();
        let _ = s.set_property("left", &format!("{}px", b.left() + r.x));
        let _ = s.set_property("top", &format!("{}px", b.top() + r.y));
        let _ = s.set_property("height", &format!("{}px", r.height.max(1.0)));
    }
}
