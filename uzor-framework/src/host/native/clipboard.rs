//! System clipboard via arboard (design §5.4 `ClipboardRead` / `ClipboardWrite`).
//! iOS and Android have no arboard backend. The window still runs; clipboard calls no-op.

#[cfg(not(any(target_os = "ios", target_os = "android")))]
use arboard::Clipboard;

use crate::types::bus::ClipboardResult;
use crate::types::ids::Ticket;

/// Host-owned clipboard. Created once; OS errors become `None` / warn logs.
pub struct NativeClipboard {
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    inner: Option<Clipboard>,
}

impl NativeClipboard {
    /// Open the system clipboard; `None` inside when the OS denies access.
    pub fn new() -> Self {
        #[cfg(any(target_os = "ios", target_os = "android"))]
        {
            return Self {};
        }
        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        Self {
            inner: Clipboard::new()
                .map_err(|e| {
                    log::warn!("clipboard open failed: {e}");
                    e
                })
                .ok(),
        }
    }

    /// Write text; OS errors are logged and ignored (executor contract).
    pub fn write(&mut self, text: &str) {
        #[cfg(any(target_os = "ios", target_os = "android"))]
        {
            let _ = text;
            return;
        }
        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        {
            let Some(cb) = self.inner.as_mut() else {
                return;
            };
            if let Err(e) = cb.set_text(text.to_owned()) {
                log::warn!("clipboard write failed: {e}");
            }
        }
    }

    /// Write an image (RGBA8).
    pub fn write_image(&mut self, img: &crate::types::bus::ClipboardImage) {
        #[cfg(any(target_os = "ios", target_os = "android"))]
        {
            let _ = img;
            return;
        }
        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        {
            let Some(cb) = self.inner.as_mut() else {
                return;
            };
            let data = arboard::ImageData {
                width: img.width as usize,
                height: img.height as usize,
                bytes: std::borrow::Cow::Borrowed(&img.rgba),
            };
            if let Err(e) = cb.set_image(data) {
                log::warn!("clipboard image write failed: {e}");
            }
        }
    }

    /// The clipboard image, if it holds one.
    pub fn read_image(&mut self) -> Option<crate::types::bus::ClipboardImage> {
        #[cfg(any(target_os = "ios", target_os = "android"))]
        {
            return None;
        }
        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        {
            let img = self.inner.as_mut()?.get_image().ok()?;
            Some(crate::types::bus::ClipboardImage {
                width: img.width as u32,
                height: img.height as u32,
                rgba: img.bytes.into_owned().into(),
            })
        }
    }

    /// Read the clipboard's text now (synchronously): `None` when it holds no text,
    /// the OS denies access, or the platform has no clipboard. For an app that pastes
    /// from its own key handling on desktop; the web has no synchronous read (there
    /// the paste arrives as `UnhandledInput::Paste`).
    pub fn read_text(&mut self) -> Option<String> {
        self.read(Ticket(0)).text
    }

    /// Read text for a `ClipboardRead` ticket.
    pub fn read(&mut self, ticket: Ticket) -> ClipboardResult {
        #[cfg(any(target_os = "ios", target_os = "android"))]
        {
            return ClipboardResult { ticket, text: None };
        }
        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        {
            let text = self.inner.as_mut().and_then(|cb| match cb.get_text() {
                Ok(t) if t.is_empty() => None,
                Ok(t) => Some(t),
                Err(e) => {
                    log::warn!("clipboard read failed: {e}");
                    None
                }
            });
            ClipboardResult { ticket, text }
        }
    }
}

impl Default for NativeClipboard {
    fn default() -> Self {
        Self::new()
    }
}
