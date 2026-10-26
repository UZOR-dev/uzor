//! [`ImagePainter`] — opt-in image rendering capability.
//!
//! Backends that cannot render images do not implement this trait.
//! Callers check capability explicitly instead of silently skipping.

/// Image rendering — opt-in.
///
/// Backends that cannot render images do not implement this trait.
pub trait ImagePainter {
    /// Draw a cached image by URL/id.
    ///
    /// Returns `false` if the image is not yet loaded or cached.
    fn draw_image(
        &mut self,
        image_id: &str,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> bool;

    /// Draw raw RGBA pixel data.
    ///
    /// # Arguments
    /// * `data` — RGBA pixel data (4 bytes per pixel, row-major, top-to-bottom)
    /// * `img_width`, `img_height` — source image dimensions in pixels
    /// * `x`, `y` — top-left corner position on canvas
    /// * `width`, `height` — target draw dimensions (stretched/shrunk to fit)
    #[allow(clippy::too_many_arguments)]
    fn draw_image_rgba(
        &mut self,
        data: &[u8],
        img_width: u32,
        img_height: u32,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    );

    /// Draw a source rectangle of `data` into the dest rectangle.
    ///
    /// `sx, sy, sw, sh` are pixels of the full image. Return `false` to
    /// keep the caller's extract-and-blit path (the default). A backend
    /// that can sample a crop from one shared texture returns `true`.
    #[allow(clippy::too_many_arguments)]
    fn draw_image_rgba_src(
        &mut self,
        data: &[u8],
        img_width: u32,
        img_height: u32,
        sx: f64,
        sy: f64,
        sw: f64,
        sh: f64,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> bool {
        let _ = (data, img_width, img_height, sx, sy, sw, sh, x, y, width, height);
        false
    }
}
