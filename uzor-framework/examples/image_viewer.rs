//! Image viewer over a gallery of PNGs: a DOM library and a DOM lightbox, the
//! pictures painted into embed slots.
//!
//! ```text
//! cargo run -p uzor-framework --example image_viewer --features native -- a.png b.png …
//! ```
//!
//! Without arguments it shows eight generated samples. Click a tile (or Enter) to
//! open; Left / Right, the arrows or a thumbnail switch; Esc, X, back or the backdrop
//! close; the download button prints the file path.

#[path = "viewer/mod.rs"]
mod viewer;

use uzor_framework::host::native::run_native_default;

fn main() -> Result<(), uzor_framework::FrameworkError> {
    run_native_default(viewer::from_args(std::env::args().skip(1)), viewer::Viewer::config())
}
