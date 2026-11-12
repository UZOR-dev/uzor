//! L4 dashboard on [`uzor-framework`] (brief E1).
//!
//! Older layered demos (L1–L3, force-graph, figures, typography,
//! text-kinetics, the urx smoke bin, the parity harness) are frozen in
//! `_deprecated/uzor-examples`. They are not a workspace member.

pub mod dashboard;
pub mod kit_gallery;
pub mod look;
pub mod page_app;
pub mod catalog;

pub use dashboard::DashboardApp;
pub use kit_gallery::Gallery;
pub mod dom_gallery;
pub mod dom_widgets;
pub mod mobile_gallery;
pub mod tray_widgets;
pub mod dom_slots;
pub mod dom_images;
