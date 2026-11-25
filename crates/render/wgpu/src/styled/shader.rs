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
        )
        // A stroke's quad three quarters of a pixel past its edge's smoothing, not a whole one (docs/adr/9247): no sample
        // of any pattern lies farther than half a diagonal (0.71 px) from its pixel's middle, so a drawn pixel still
        // covers all its samples on every sample count, and every view draws the same quad. The joins' test keeps `ext`.
        .replacen(
            "  let along = mix(-ext, len + ext, c.x);\n  let across = mix(-ext, ext, c.y);",
            "  let quad = halfW + 0.5 * blur + 0.75;\n  let along = mix(-quad, len + quad, c.x);\n  let across = mix(-quad, quad, c.y);",
            1,
        );
    source.push_str(include_str!("dashes.wgsl"));
    // Instanced blocks (docs/adr/1063 §2), native only as the long patterns are.
    source.push_str(include_str!("instances.wgsl"));
    // Pattern tiles drawn from their SVG marks' triangles (plan/render.md RM-11), reading the long-pattern page.
    source.push_str(include_str!("vector_tiles.wgsl"));
    // A stroke's quad as a strip of four corners (docs/adr/9247).
    source.push_str(include_str!("strip.wgsl"));
    source
}

#[cfg(test)]
mod tests {
    use super::*;

    /// docs/adr/9247: the native source's stroke quad reaches three quarters of a pixel past its smoothing (a shared
    /// source changed under the patch would leave the quad as it was, silently), and its strip's vertex stage is there.
    #[test]
    fn the_stroke_quad_reaches_three_quarters_of_a_pixel_past_its_smoothing() {
        let native = source();
        assert!(native.contains("let quad = halfW + 0.5 * blur + 0.75;"), "the corners' patch applies");
        assert!(!native.contains("let along = mix(-ext, len + ext, c.x);"));
        assert!(native.contains("fn strokeVsStrip("));
    }

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
