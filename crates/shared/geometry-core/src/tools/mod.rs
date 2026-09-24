//! What the drawing tools compute from their points and typed values
//! (docs/adr/0008, S5): typed point input, the ortho and polar cursor,
//! object tracking, the point calculator's own arithmetic, and each tool's
//! constructions (directions, typed-radius polygons, arc bulges, corners,
//! transforms, dimension arms). The tools in `apps/web/src/tools` and
//! `apps/web/src/viewport` only pick, preview and record; camera and screen pixels
//! stay there.

pub mod drawing;
pub mod editing;
pub mod object_tracking;
pub mod point_input;
