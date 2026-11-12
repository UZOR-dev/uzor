//! # uzor-plot
//!
//! Plot math for uzor apps: the numbers behind axes and charts, with no
//! rendering and no DOM. Widgets and renderers on top decide how things look.
//!
//! - [`ticks`]: nice-step ladders, tick density, label precision, label culling.
//! - [`value_scale`]: linear / log / percent / indexed value axes, auto-range, zoom and pan.
//!
//! Every module is plain functions and small `Copy` models, so anything an
//! app does not call is dropped by the linker.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod ticks;
pub mod value_scale;
