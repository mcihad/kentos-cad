//! KentOS's model viewer component (docs/adr/8061 §3, docs/adr/8073 §2), over the viewer core (`kentos-model-view`):
//!
//! - [`scene`]: the 3D scene on Iced's shader widget, every 3D view's (the building's and the road's 3B görünüm, the
//!   IFC viewer's): tiles, picking, the clicked point; gestures back to the host.
//! - [`look`]: what every 3D view wears around its scene: the sky, the north arrow, the legend, the views' menu.
//! - [`viewer`]: the model viewer: the tree, the scene, the details, what shows, the section; its look in
//!   viewer_view.rs, its host's part in [`Host`].
//! - [`measuring`]: the viewer's measuring: a distance, an angle, a flat face's area, on points snapped to the bodies.
//!
//! No application, file, command or thread: a host (the application's adapter, the single-window example) fills the
//! core's input and draws the component where it wants it.

pub mod look;
pub mod measuring;
pub mod scene;
pub mod viewer;
mod viewer_view;

pub use measuring::{Measured, Tool};
pub use viewer::{Message, Out, Panel, Side, Stage, Viewer};
pub use viewer_view::Host;
