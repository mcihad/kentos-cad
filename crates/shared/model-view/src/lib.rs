//! The model viewer's core (docs/adr/8061 §3, docs/adr/8073 §2): what a 3D model viewer knows of a model, whatever it
//! comes from. An adapter fills its [`Input`] (an IFC file read: kentos-ifc's `view` feature); the viewer's component
//! (crates/model-view-ui) draws what it gives and sends back what the user does.
//!
//! - [`Input`]: the model's objects (a key, the class as the source names it, a label, a category, the tree parent, the
//!   storey) and the bodies of those that have one, with what could not be shown and why.
//! - [`View`]: what shows (hidden, isolated, the spaces) and what is chosen; the tree's rows by structure or by class;
//!   the scene's source meshes in their parts' colours ([`PARTS`]), the chosen one and the found ones painted over
//!   their category's.
//! - [`OrbitCamera`]: the camera about a point (orbit, pan, dolly, fit, the four views), the 3D views' one.
//! - [`Section`]: a plane across the model or a box about the chosen objects; the planes the renderer clips with, and
//!   [`caps`]: the cut faces filled (docs/adr/8131).
//! - [`Details`]: the chosen object's named values, as the source writes them, for the properties panel.
//! - [`measure`]: a point snapped to a body's corner, an edge's middle, an edge or its face; a distance and its
//!   components, an angle, a face's area (docs/adr/8121).
//!
//! Pure: no window, GPU, file, clock or thread; native and wasm32.

pub mod camera;
pub mod caps;
pub mod details;
pub mod input;
pub mod measure;
pub mod parts;
pub mod section;
pub mod tree;
pub mod view;

pub use camera::{OrbitCamera, VIEWS};
pub use details::{Details, Group, Line};
pub use input::{BodyIn, Category, Input, ObjectIn};
pub use parts::{PARTS, Part};
pub use section::{Cut, Section};
pub use tree::{Row, TreeMode};
pub use view::{Source, View};
