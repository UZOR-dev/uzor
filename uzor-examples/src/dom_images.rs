//! Raster images in the DOM ([`uzor::ui::dom::El::image`]): a round avatar
//! (`border-radius: 50%`, `object-fit: cover`), a cover image under `cover` and
//! `contain`, and an image the backend has not loaded yet (its box background shows).
//! The pictures are made up in code (greys only). [`ImagesDemo::tree`] is pure: the
//! same page renders in tests and in the `dom-images-frames` PNG bin.

use std::sync::Arc;

use uzor::render::RenderContext;
use uzor::ui::dom::{El, ImageRef};
use uzor::ui::style::Sheet;

/// Page rules on top of the kit sheet.
pub const IMAGES_CSS: &str = r#"
div.di-page { display: flex; flex-direction: column; gap: 14px; padding: 20px; background: var(--k-bg); color: var(--k-fg) }
div.di-page.light { --k-bg: #f6f7f9; --k-panel: #ffffff; --k-fg: #1d1f24; --k-muted: #6b6f78; --k-line: #dfe1e6; --k-hover: #eef0f4 }
div.di-h1 { font-size: 15px; font-weight: 600 }
div.di-row { display: flex; gap: 14px; align-items: flex-start }
div.di-card { display: flex; flex-direction: column; gap: 8px; padding: 12px 14px; border-radius: 10px; border: 1px solid var(--k-line); background: var(--k-panel) }
div.di-cap { font-size: 12px; font-weight: 600 }
div.di-note { font-size: 11px; color: var(--k-muted) }
div.di-who { display: flex; gap: 10px; align-items: center }
img.di-avatar { width: 64px; height: 64px; border-radius: 999px; object-fit: cover; background: var(--k-hover) }
img.di-cover { width: 220px; height: 120px; border-radius: 8px; object-fit: cover; background: var(--k-hover) }
img.di-cover.contain { object-fit: contain }
"#;

fn tx(ru: bool, r: &'static str, e: &'static str) -> &'static str {
    if ru {
        r
    } else {
        e
    }
}

/// A 96×64 grey portrait: a light disc on a dark field.
pub fn avatar_image() -> ImageRef {
    let (w, h) = (96u32, 64u32);
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let (dx, dy) = (x as f64 - 48.0, y as f64 - 32.0);
            let d = (dx * dx + dy * dy).sqrt();
            let v = if d < 22.0 { 210 } else { 70 + (y * 40 / h) as u8 };
            px.extend_from_slice(&[v, v, v, 255]);
        }
    }
    ImageRef::rgba(Arc::<[u8]>::from(px), w, h).expect("avatar pixels")
}

/// A 320×240 grey cover: a left-to-right ramp with a dark horizon band.
pub fn cover_image() -> ImageRef {
    let (w, h) = (320u32, 240u32);
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let v = if (150..170).contains(&y) { 30 } else { (60 + x * 160 / w) as u8 };
            px.extend_from_slice(&[v, v, v, 255]);
        }
    }
    ImageRef::rgba(Arc::<[u8]>::from(px), w, h).expect("cover pixels")
}

/// Demo state.
#[derive(Clone, Debug, PartialEq)]
pub struct ImagesDemo {
    pub ru: bool,
    pub dark: bool,
}

impl Default for ImagesDemo {
    fn default() -> Self {
        ImagesDemo { ru: true, dark: true }
    }
}

impl ImagesDemo {
    pub fn sheet() -> Sheet {
        Sheet::parse(&[uzor::ui::domkit::KIT_CSS, IMAGES_CSS].concat()).expect("images demo css")
    }

    pub fn tree(&self, w: f64) -> El {
        let ru = self.ru;
        let card = |cap: &str, img: El, note: &str| El::new("div.di-card").child(El::new("div.di-cap").text(cap)).child(img).child(El::new("div.di-note").text(note));
        El::new(if self.dark { "div.di-page" } else { "div.di-page.light" })
            .css(&format!("width: {w}px"))
            .child(El::new("div.di-h1").text(tx(ru, "Картинки", "Images")))
            .child(
                El::new("div.di-row")
                    .child(
                        El::new("div.di-card").child(El::new("div.di-cap").text(tx(ru, "Аватар", "Avatar"))).child(
                            El::new("div.di-who")
                                .child(El::new("img.di-avatar").id("avatar").name(tx(ru, "Аватар: князь", "Avatar: knyaz")).image(avatar_image()))
                                .child(El::new("div.di-note").text("cover, 999px")),
                        ),
                    )
                    .child(card(tx(ru, "Обложка", "Cover"), El::new("img.di-cover").id("cover").image(cover_image()), "object-fit: cover"))
                    .child(card(tx(ru, "Целиком", "Whole"), El::new("img.di-cover.contain").id("contain").image(cover_image()), "object-fit: contain"))
                    .child(card(tx(ru, "Загружается", "Loading"), El::new("img.di-cover").id("pending").image(ImageRef::id_sized("https://example.invalid/cover.png", 320, 240)), tx(ru, "фон, пока нет картинки", "background until it loads"))),
            )
    }

    /// Fill the background, lay the page out `w` wide and paint it.
    pub fn render(&self, ctx: &mut dyn RenderContext, w: f64, h: f64) -> uzor::ui::dom::Dom {
        ctx.set_fill_color(if self.dark { "#17181b" } else { "#f6f7f9" });
        ctx.fill_rect(0.0, 0.0, w, h);
        let d = uzor::ui::dom::lay_out_in(ctx, &Self::sheet(), &self.tree(w), 0.0, 0.0, w, Some(uzor::Rect::new(0.0, 0.0, w, h)), &|_| uzor::ui::style::State::NONE);
        uzor::ui::dom::paint(ctx, &d);
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uzor::testing::NullRenderContext;
    use uzor::ui::dom;
    use uzor::ui::style::State;

    #[test]
    fn images_are_img_elements_with_names_and_fit() {
        let mut ctx = NullRenderContext::default();
        let d = dom::lay_out(&mut ctx, &ImagesDemo::sheet(), &ImagesDemo::default().tree(1000.0), 0.0, 0.0, 1000.0, &|_| State::NONE);
        let a11y = d.a11y();
        let av = a11y.iter().find(|n| n.id == "avatar").unwrap();
        assert_eq!((av.role, av.name.as_str()), ("img", "Аватар: князь"));
        assert_eq!(d.get("cover").unwrap().style.object_fit.as_deref(), Some("cover"));
        assert_eq!(d.get("contain").unwrap().style.object_fit.as_deref(), Some("contain"));
        assert_eq!(d.rect("avatar").map(|r| (r.width, r.height)), Some((64.0, 64.0)));
    }
}
