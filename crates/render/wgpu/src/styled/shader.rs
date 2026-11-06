//! The styled drawing's shared WGSL (`shaders/wgsl/styled`, docs/STYLE.md
//! §6): its files joined in the order the contract lists them
//! (`styled.layout.json` → `sources`), as the web's WebGPU backend and the
//! browser check join them.

/// The module's source files: path under `shaders/wgsl`, and text.
pub const SOURCES: [(&str, &str); 5] = [
    (
        "styled/common.wgsl",
        include_str!("../../../../../shaders/wgsl/styled/common.wgsl"),
    ),
    (
        "styled/shapes.wgsl",
        include_str!("../../../../../shaders/wgsl/styled/shapes.wgsl"),
    ),
    (
        "styled/stroke.wgsl",
        include_str!("../../../../../shaders/wgsl/styled/stroke.wgsl"),
    ),
    (
        "styled/fill.wgsl",
        include_str!("../../../../../shaders/wgsl/styled/fill.wgsl"),
    ),
    (
        "styled/marker.wgsl",
        include_str!("../../../../../shaders/wgsl/styled/marker.wgsl"),
    ),
];

/// The binding, vertex and uniform layout contract of the module.
pub const LAYOUT_JSON: &str = include_str!("../../../../../shaders/wgsl/styled.layout.json");

/// The module's source as the contract has it: its files joined, each after a comment naming it.
/// The desktop builds it as it is: since the contract's version 2 the atlas
/// binds in group 0 beside the frame, and Iced's device takes two bind groups
/// (`max_bind_groups: 2`, iced_wgpu's compositor; docs/adr/0090).
pub fn shared_source() -> String {
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

/// Native long-pattern extension, preserving the paused browser's v10 source
/// and its bindings. Short recipes still call the original inline function.
pub fn source() -> String {
    let mut source = shared_source()
        .replacen("fn dashCover(", "fn inlineDashCover(", 1)
        // Round dots of the line's weight (ADR 1124), native only: the paused browser keeps its one-pixel dot.
        .replacen(
            "a *= dashCover(i.s0 + x, unitK(), st.a.y, st.a.z, st.a.w);",
            "a = max(a * strokeDash(i.s0 + x), dotCover(i.s0 + x, x, y, i.len, (i.ends & 2u) != 0u, i.halfW));",
            1,
        );
    source.push_str(include_str!("dashes.wgsl"));
    // Instanced blocks (docs/adr/1063 §2), native only as the long patterns are.
    source.push_str(include_str!("instances.wgsl"));
    source
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_atlas_binds_beside_the_frame_in_two_groups() {
        let shared = shared_source();
        assert_eq!(
            shared.matches("@group(0) @binding(1) var atlasTex").count(),
            1
        );
        assert_eq!(
            shared.matches("@group(0) @binding(2) var atlasSmp").count(),
            1
        );
        assert!(!shared.contains("@group(2)"));
    }

    /// ADR 1124: the native strokes take their round dots; the shared (browser) source is unchanged.
    #[test]
    fn native_strokes_draw_round_dots() {
        let native = source();
        assert_eq!(native.matches("dotCover(i.s0 + x, x, y, i.len").count(), 1);
        assert!(!shared_source().contains("dotCover"));
    }
}
