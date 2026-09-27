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

/// The contract's atlas bindings (group 2) and where the desktop binds them:
/// beside the frame in group 0. Iced's device takes two bind groups at most
/// (`max_bind_groups: 2`, iced_wgpu's compositor), the contract uses three;
/// the shader code is the shared one, only these two numbers move (docs/adr/0090).
pub const ATLAS_REMAP: [(&str, &str); 2] = [
    (
        "@group(2) @binding(0) var atlasTex",
        "@group(0) @binding(1) var atlasTex",
    ),
    (
        "@group(2) @binding(1) var atlasSmp",
        "@group(0) @binding(2) var atlasSmp",
    ),
];

/// The module's source as the contract has it: its files joined, each after a comment naming it.
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

/// The module the desktop builds: the shared source with the atlas bound in group 0 ([`ATLAS_REMAP`]).
pub fn source() -> String {
    ATLAS_REMAP
        .iter()
        .fold(shared_source(), |text, (from, to)| text.replace(from, to))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_atlas_moves_to_the_frame_s_group_once() {
        let shared = shared_source();
        for (from, _) in ATLAS_REMAP {
            assert_eq!(shared.matches(from).count(), 1, "{from}");
        }
        let desktop = source();
        assert!(!desktop.contains("@group(2)"));
    }
}
