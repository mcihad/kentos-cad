//! KentOS CAD's native drawing pipeline on wgpu (docs/adr/0019; TODOS.md
//! REN-01..07). The desktop app draws the open drawing with it; the web keeps
//! its own WebGPU and WebGL2 renderers and shares only the WGSL
//! (`shaders/wgsl`, ADR 0010).
//!
//! What it owns:
//! - [`Camera`]: the view in float64 world units (x east, y north) and logical
//!   pixels: pan, zoom at a point, fit, screen ↔ world.
//! - [`scene`]: the drawing as the GPU draws it, built from a `.kcad` v1
//!   document on the CPU. Straight geometry, points and fills form one part
//!   that changes only with the document; curves, tessellated by the shared
//!   geometry core within an on-screen tolerance, form another that also
//!   follows the zoom band. Nothing here changes the document.
//! - [`Renderer`]: the device resources: the pipelines of the shared WGSL, a
//!   uniform per view and the scene cache, GPU buffers uploaded once per part
//!   and never per frame.
//! - [`RenderSettings`] and [`FrameStats`].
//!
//! It knows no Iced. A host hands it the device, queue and render pass it
//! draws with; the desktop app does that from Iced's shader widget, so the
//! drawing shares Iced's device and frame with no copy between GPU and CPU.
//!
//! Precision (REN-07): coordinates stay float64 on the CPU. The GPU gets the
//! float32 high and low parts of each point's offset from a local origin, and
//! the camera centre the same way; the vertex shader subtracts part by part,
//! so the float32 it turns into pixels is the offset from the camera. See
//! [`precision`].
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod camera;
pub mod color;
pub mod layout;
pub mod precision;
pub mod renderer;
pub mod scene;
pub mod settings;
pub mod shader;
pub mod stats;

pub use camera::Camera;
pub use color::{Palette, Rgba8};
pub use renderer::{FrameInput, RenderError, Renderer, ViewId};
pub use scene::{LayerRanges, ScenePart};
pub use settings::RenderSettings;
pub use stats::FrameStats;

/// World points and boxes, as the shared geometry core has them.
pub use kentos_geometry_core::Vec2;
pub use kentos_geometry_core::geometry::Bounds;
