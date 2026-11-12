//! Raster images in the tree: [`ImageRef`] and CSS `object-fit` ([`El::image`](super::El::image)).

use std::sync::Arc;

use crate::types::Rect;

/// What a raster element ([`El::image`](super::El::image)) shows.
#[derive(Clone, PartialEq)]
pub enum ImageRef {
    /// An image the backend caches by id (a URL or an asset key), drawn with
    /// [`ImagePainter::draw_image`](crate::render::ImagePainter::draw_image). `size` is
    /// its intrinsic size in pixels, when known: `object-fit: contain | cover | none |
    /// scale-down` need it; without it the image fills the box.
    Id { id: String, size: Option<(u32, u32)> },
    /// Decoded RGBA pixels (4 bytes a pixel, row-major), shared between frames.
    Rgba { data: Arc<[u8]>, width: u32, height: u32 },
}

impl ImageRef {
    /// A cached image by id, intrinsic size unknown.
    pub fn id(id: impl Into<String>) -> Self {
        ImageRef::Id { id: id.into(), size: None }
    }
    /// A cached image by id with its intrinsic size.
    pub fn id_sized(id: impl Into<String>, width: u32, height: u32) -> Self {
        ImageRef::Id { id: id.into(), size: Some((width, height)) }
    }
    /// RGBA pixels; `None` when `data` is not `width * height * 4` bytes.
    pub fn rgba(data: impl Into<Arc<[u8]>>, width: u32, height: u32) -> Option<Self> {
        let data = data.into();
        (data.len() == width as usize * height as usize * 4 && width > 0 && height > 0).then_some(ImageRef::Rgba { data, width, height })
    }
    /// The intrinsic size, when known.
    pub fn size(&self) -> Option<(u32, u32)> {
        match self {
            ImageRef::Id { size, .. } => *size,
            ImageRef::Rgba { width, height, .. } => Some((*width, *height)),
        }
    }
}

impl std::fmt::Debug for ImageRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageRef::Id { id, size } => f.debug_struct("Id").field("id", id).field("size", size).finish(),
            ImageRef::Rgba { width, height, .. } => write!(f, "Rgba({width}x{height})"),
        }
    }
}

/// Where an image of intrinsic size `size` goes in `content` under CSS `object-fit`
/// (`fill` default, `contain`, `cover`, `none`, `scale-down`), centred as
/// `object-position: 50% 50%`. The result may overflow `content` (`cover`, `none`);
/// the painter clips it. Without a size the image fills the box.
pub fn object_fit_rect(content: Rect, size: Option<(u32, u32)>, fit: Option<&str>) -> Rect {
    let Some((iw, ih)) = size.filter(|(w, h)| *w > 0 && *h > 0) else { return content };
    let (iw, ih) = (iw as f64, ih as f64);
    let scale = match fit.map(str::trim) {
        Some("contain") => (content.width / iw).min(content.height / ih),
        Some("cover") => (content.width / iw).max(content.height / ih),
        Some("none") => 1.0,
        Some("scale-down") => (content.width / iw).min(content.height / ih).min(1.0),
        _ => return content,
    };
    let (w, h) = (iw * scale, ih * scale);
    Rect::new(content.x + (content.width - w) / 2.0, content.y + (content.height - h) / 2.0, w, h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_fit_places_like_css() {
        let c = Rect::new(10.0, 20.0, 200.0, 100.0);
        let s = Some((100, 100));
        assert_eq!(object_fit_rect(c, s, None), c);
        assert_eq!(object_fit_rect(c, s, Some("fill")), c);
        assert_eq!(object_fit_rect(c, s, Some("contain")), Rect::new(60.0, 20.0, 100.0, 100.0));
        assert_eq!(object_fit_rect(c, s, Some("cover")), Rect::new(10.0, -30.0, 200.0, 200.0));
        assert_eq!(object_fit_rect(c, Some((40, 20)), Some("none")), Rect::new(90.0, 60.0, 40.0, 20.0));
        assert_eq!(object_fit_rect(c, Some((40, 20)), Some("scale-down")), Rect::new(90.0, 60.0, 40.0, 20.0));
        assert_eq!(object_fit_rect(c, Some((400, 200)), Some("scale-down")), c);
        assert_eq!(object_fit_rect(c, None, Some("cover")), c, "no size: fill");
    }

    #[test]
    fn rgba_checks_its_length() {
        assert!(ImageRef::rgba(vec![0u8; 16], 2, 2).is_some());
        assert!(ImageRef::rgba(vec![0u8; 15], 2, 2).is_none());
        assert!(ImageRef::rgba(Vec::<u8>::new(), 0, 0).is_none());
        assert_eq!(format!("{:?}", ImageRef::rgba(vec![0u8; 16], 2, 2).unwrap()), "Rgba(2x2)");
    }
}
