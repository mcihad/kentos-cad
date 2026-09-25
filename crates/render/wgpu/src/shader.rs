//! The shared WGSL (`shaders/wgsl`, TODOS.md REN-04/05) as this crate compiles
//! it. WGSL has no includes: a module is its source files joined in the order
//! the layout file lists them (`cad2d.layout.json` → `sources`). The common
//! parts (frame, colour, marks) come first, the pipelines' entry points after;
//! the browser check (`scripts/wgsl/browser-check.mjs`) joins the same files
//! the same way.

/// The module's source files: path under `shaders/wgsl`, and text.
pub const SOURCES: [(&str, &str); 7] = [
    (
        "common/frame.wgsl",
        include_str!("../../../../shaders/wgsl/common/frame.wgsl"),
    ),
    (
        "common/color.wgsl",
        include_str!("../../../../shaders/wgsl/common/color.wgsl"),
    ),
    (
        "common/marks.wgsl",
        include_str!("../../../../shaders/wgsl/common/marks.wgsl"),
    ),
    (
        "cad2d/background.wgsl",
        include_str!("../../../../shaders/wgsl/cad2d/background.wgsl"),
    ),
    (
        "cad2d/fill.wgsl",
        include_str!("../../../../shaders/wgsl/cad2d/fill.wgsl"),
    ),
    (
        "cad2d/line.wgsl",
        include_str!("../../../../shaders/wgsl/cad2d/line.wgsl"),
    ),
    (
        "cad2d/marker.wgsl",
        include_str!("../../../../shaders/wgsl/cad2d/marker.wgsl"),
    ),
];

/// The binding, vertex and uniform layout contract of the module.
pub const LAYOUT_JSON: &str = include_str!("../../../../shaders/wgsl/cad2d.layout.json");

/// The module's source: its files joined, each after a comment naming it.
pub fn source() -> String {
    let mut out = String::with_capacity(SOURCES.iter().map(|(p, s)| p.len() + s.len() + 16).sum());
    for (path, text) in SOURCES {
        out.push_str("// ── ");
        out.push_str(path);
        out.push_str(" ──\n");
        out.push_str(text);
        out.push('\n');
    }
    out
}
