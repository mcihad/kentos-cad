//! The styled drawing's shared WGSL and its layout contract
//! (`shaders/wgsl/styled.layout.json`, version 5, docs/STYLE.md §6,
//! docs/adr/0090): naga, the compiler wgpu runs, parses and validates the
//! shared module, which the desktop builds as it is (two bind groups: the
//! frame with the atlas, then the style; Iced's device takes two); the
//! uniforms, entry points, pipelines and vertex layouts the desktop builds are
//! the contract's. The browser side is `scripts/wgsl/browser-check.mjs`.

use std::collections::BTreeMap;
use std::mem::size_of;
use std::path::Path;

use kentos_render_wgpu::styled::gpu::STYLED_PIPELINES;
use kentos_render_wgpu::styled::shader;
use kentos_render_wgpu::styled::uniform::{STYLE_BYTES, StyledFrameUniform};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Contract {
    format: String,
    version: u32,
    module: String,
    sources: Vec<String>,
    bind_groups: Vec<BindGroup>,
    structs: BTreeMap<String, StructDef>,
    pipelines: Vec<Pipeline>,
    blends: BTreeMap<String, Blend>,
}

#[derive(Deserialize)]
struct BindGroup {
    group: u32,
    bindings: Vec<Binding>,
}

#[derive(Deserialize)]
struct Binding {
    binding: u32,
    name: String,
}

#[derive(Deserialize)]
struct StructDef {
    size: u32,
    fields: Vec<Field>,
}

#[derive(Deserialize)]
struct Field {
    name: String,
    offset: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pipeline {
    name: String,
    vertex: String,
    fragment: String,
    vertex_count: Option<u32>,
    blend: String,
    buffers: Vec<Buffer>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Buffer {
    step_mode: String,
    array_stride: u64,
    attributes: Vec<Attribute>,
}

#[derive(Deserialize)]
struct Attribute {
    location: u32,
    format: String,
    offset: u64,
}

#[derive(Deserialize)]
struct Blend {
    color: Component,
    alpha: Component,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Component {
    src_factor: String,
    dst_factor: String,
    operation: String,
}

fn contract() -> Contract {
    serde_json::from_str(shader::LAYOUT_JSON).expect("styled.layout.json reads")
}

fn validate(source: &str) -> naga::Module {
    let module = naga::front::wgsl::parse_str(source)
        .unwrap_or_else(|e| panic!("WGSL does not parse:\n{}", e.emit_to_string(source)));
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap_or_else(|e| panic!("WGSL does not validate:\n{}", e.emit_to_string(source)));
    module
}

#[test]
fn the_shared_module_validates_and_the_desktop_builds_it_as_it_is() {
    let c = contract();
    assert_eq!(
        (c.format.as_str(), c.version, c.module.as_str()),
        ("kentos.wgsl-layout", 5, "styled")
    );
    assert_eq!(shader::source(), shader::shared_source());
    let module = validate(&shader::shared_source());
    for p in &c.pipelines {
        for (name, stage) in [
            (&p.vertex, naga::ShaderStage::Vertex),
            (&p.fragment, naga::ShaderStage::Fragment),
        ] {
            assert!(
                module
                    .entry_points
                    .iter()
                    .any(|e| &e.name == name && e.stage == stage),
                "{}: {name} ({stage:?})",
                p.name
            );
        }
    }
}

#[test]
fn the_sources_are_the_contract_s_and_current() {
    let c = contract();
    let rust: Vec<&str> = shader::SOURCES.iter().map(|(p, _)| *p).collect();
    assert_eq!(rust, c.sources);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../shaders/wgsl");
    for (path, text) in shader::SOURCES {
        let disk = std::fs::read_to_string(root.join(path)).expect("source reads");
        assert_eq!(disk, text, "{path}: the compiled-in copy is current");
    }
}

/// Where a global is bound in a module.
fn binding(module: &naga::Module, name: &str) -> Option<naga::ResourceBinding> {
    module
        .global_variables
        .iter()
        .find(|(_, g)| g.name.as_deref() == Some(name))
        .and_then(|(_, g)| g.binding)
}

#[test]
fn the_bindings_are_the_contract_s_with_the_atlas_beside_the_frame() {
    let c = contract();
    let shared = validate(&shader::shared_source());
    for group in &c.bind_groups {
        for b in &group.bindings {
            assert_eq!(
                binding(&shared, &b.name),
                Some(naga::ResourceBinding {
                    group: group.group,
                    binding: b.binding
                }),
                "{}",
                b.name
            );
        }
    }
    // The frame, the atlas texture and its sampler in group 0; the style in group 1:
    // two groups, all Iced's device takes.
    let at = |name: &str| binding(&shared, name).map(|b| (b.group, b.binding));
    assert_eq!(at("frame"), Some((0, 0)));
    assert_eq!(at("atlasTex"), Some((0, 1)));
    assert_eq!(at("atlasSmp"), Some((0, 2)));
    assert_eq!(at("st"), Some((1, 0)));
    assert!(
        shared
            .global_variables
            .iter()
            .all(|(_, g)| g.binding.as_ref().is_none_or(|b| b.group < 2))
    );
}

#[test]
fn the_uniforms_are_laid_out_as_the_contract_says() {
    let c = contract();
    let module = validate(&shader::shared_source());
    let mut layouter = naga::proc::Layouter::default();
    layouter
        .update(module.to_ctx())
        .expect("naga lays out the types");
    for (name, rust) in [
        ("Frame", size_of::<StyledFrameUniform>()),
        ("SStyle", STYLE_BYTES),
    ] {
        let def = &c.structs[name];
        assert_eq!(rust as u32, def.size, "{name}: Rust size");
        let (handle, ty) = module
            .types
            .iter()
            .find(|(_, t)| t.name.as_deref() == Some(name))
            .expect("WGSL declares the struct");
        assert_eq!(layouter[handle].size, def.size, "{name}: WGSL size");
        let naga::TypeInner::Struct { members, .. } = &ty.inner else {
            panic!("{name} is a struct");
        };
        let offsets: Vec<(Option<&str>, u32)> = members
            .iter()
            .map(|m| (m.name.as_deref(), m.offset))
            .collect();
        let want: Vec<(Option<&str>, u32)> = def
            .fields
            .iter()
            .map(|f| (Some(f.name.as_str()), f.offset))
            .collect();
        assert_eq!(offsets, want, "{name}: fields");
    }
}

fn format_name(format: wgpu::VertexFormat) -> &'static str {
    match format {
        wgpu::VertexFormat::Float32 => "float32",
        wgpu::VertexFormat::Float32x2 => "float32x2",
        wgpu::VertexFormat::Float32x4 => "float32x4",
        _ => "?",
    }
}

fn factor(f: wgpu::BlendFactor) -> &'static str {
    match f {
        wgpu::BlendFactor::One => "one",
        wgpu::BlendFactor::SrcAlpha => "src-alpha",
        wgpu::BlendFactor::OneMinusSrcAlpha => "one-minus-src-alpha",
        _ => "?",
    }
}

#[test]
fn the_pipelines_are_the_contract_s() {
    let c = contract();
    let names: Vec<&str> = STYLED_PIPELINES.iter().map(|p| p.name).collect();
    let want: Vec<&str> = c.pipelines.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, want);
    for (spec, p) in STYLED_PIPELINES.iter().zip(&c.pipelines) {
        assert_eq!(
            (spec.vertex, spec.fragment),
            (p.vertex.as_str(), p.fragment.as_str()),
            "{}",
            p.name
        );
        assert_eq!(spec.vertex_count, p.vertex_count, "{}: vertices", p.name);
        let [buffer] = p.buffers.as_slice() else {
            panic!("{}: one buffer", p.name);
        };
        assert_eq!(
            spec.buffer.array_stride, buffer.array_stride,
            "{}: stride",
            p.name
        );
        let step = match spec.buffer.step_mode {
            wgpu::VertexStepMode::Vertex => "vertex",
            wgpu::VertexStepMode::Instance => "instance",
        };
        assert_eq!(step, buffer.step_mode, "{}: step mode", p.name);
        let got: Vec<(u32, &str, u64)> = spec
            .buffer
            .attributes
            .iter()
            .map(|a| (a.shader_location, format_name(a.format), a.offset))
            .collect();
        let want: Vec<(u32, &str, u64)> = buffer
            .attributes
            .iter()
            .map(|a| (a.location, a.format.as_str(), a.offset))
            .collect();
        assert_eq!(got, want, "{}: attributes", p.name);
        let blend = &c.blends[&p.blend];
        for (have, want) in [
            (spec.blend.color, &blend.color),
            (spec.blend.alpha, &blend.alpha),
        ] {
            assert_eq!(factor(have.src_factor), want.src_factor, "{}", p.name);
            assert_eq!(factor(have.dst_factor), want.dst_factor, "{}", p.name);
            assert_eq!(want.operation, "add", "{}", p.name);
            assert_eq!(have.operation, wgpu::BlendOperation::Add, "{}", p.name);
        }
    }
}
