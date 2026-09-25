//! The shared WGSL and its layout contract (TODOS.md REN-04, REN-05): naga,
//! the same compiler wgpu runs, parses and validates the module; the Rust
//! structs, attribute lists and pipelines agree with
//! `shaders/wgsl/cad2d.layout.json`; and the WGSL agrees with both, as naga
//! lays out its structs and types its entry points. The browser side of the
//! check is `scripts/wgsl/browser-check.mjs`.

use std::collections::BTreeMap;
use std::mem::{offset_of, size_of};
use std::path::Path;

use kentos_render_wgpu::layout::{
    self, FillVertex, FrameUniform, LAYOUT_VERSION, MarkerInstance, PIPELINES, SegmentInstance,
};
use kentos_render_wgpu::shader;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Contract {
    format: String,
    version: u32,
    sources: Vec<String>,
    bind_groups: Vec<BindGroup>,
    structs: BTreeMap<String, StructDef>,
    pipelines: Vec<Pipeline>,
    blends: BTreeMap<String, Option<Blend>>,
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
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "struct")]
    struct_name: String,
    visibility: Vec<String>,
}

#[derive(Deserialize)]
struct StructDef {
    size: u32,
    align: u32,
    fields: Vec<Field>,
}

#[derive(Deserialize)]
struct Field {
    name: String,
    #[serde(rename = "type")]
    ty: String,
    offset: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pipeline {
    name: String,
    vertex: String,
    fragment: String,
    topology: String,
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
    name: String,
    format: String,
    offset: u64,
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Blend {
    color: BlendComponent,
    alpha: BlendComponent,
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct BlendComponent {
    src_factor: String,
    dst_factor: String,
    operation: String,
}

fn contract() -> Contract {
    serde_json::from_str(shader::LAYOUT_JSON).expect("cad2d.layout.json reads")
}

fn module() -> (naga::Module, naga::valid::ModuleInfo) {
    let source = shader::source();
    let module = naga::front::wgsl::parse_str(&source)
        .unwrap_or_else(|e| panic!("WGSL does not parse:\n{}", e.emit_to_string(&source)));
    // No capability beyond core WebGPU: what compiles here must compile in a browser too.
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap_or_else(|e| panic!("WGSL does not validate:\n{}", e.emit_to_string(&source)));
    (module, info)
}

#[test]
fn the_module_parses_and_validates_with_naga() {
    let (module, _) = module();
    let mut stages: Vec<(String, naga::ShaderStage)> = module
        .entry_points
        .iter()
        .map(|e| (e.name.clone(), e.stage))
        .collect();
    stages.sort_by(|a, b| a.0.cmp(&b.0));
    for pipeline in contract().pipelines {
        for (name, stage) in [
            (&pipeline.vertex, naga::ShaderStage::Vertex),
            (&pipeline.fragment, naga::ShaderStage::Fragment),
        ] {
            assert!(
                stages.iter().any(|(n, s)| n == name && *s == stage),
                "{}: entry point {name} ({stage:?}) is missing",
                pipeline.name
            );
        }
    }
}

#[test]
fn the_sources_are_the_ones_the_contract_lists_and_nothing_is_left_out() {
    let contract = contract();
    assert_eq!(contract.format, "kentos.wgsl-layout");
    let rust: Vec<&str> = shader::SOURCES.iter().map(|(p, _)| *p).collect();
    assert_eq!(
        rust, contract.sources,
        "shader::SOURCES follows the contract's order"
    );

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../shaders/wgsl");
    let mut on_disk = Vec::new();
    for dir in ["common", "cad2d"] {
        for entry in std::fs::read_dir(root.join(dir)).expect("shaders/wgsl reads") {
            let path = entry.expect("entry").path();
            if path.extension().is_some_and(|e| e == "wgsl") {
                let name = path.file_name().and_then(|n| n.to_str()).expect("name");
                on_disk.push(format!("{dir}/{name}"));
            }
        }
    }
    on_disk.sort();
    let mut listed = contract.sources.clone();
    listed.sort();
    assert_eq!(on_disk, listed, "every WGSL file belongs to the module");
    for (path, text) in shader::SOURCES {
        let disk = std::fs::read_to_string(root.join(path)).expect("source reads");
        assert_eq!(disk, text, "{path}: the compiled-in copy is current");
    }
}

#[test]
fn the_version_is_one_number_in_rust_json_and_wgsl() {
    let (module, _) = module();
    let version = module
        .constants
        .iter()
        .find(|(_, c)| c.name.as_deref() == Some("LAYOUT_VERSION"))
        .map(|(_, c)| &module.global_expressions[c.init])
        .expect("WGSL declares LAYOUT_VERSION");
    assert!(
        matches!(version, naga::Expression::Literal(naga::Literal::U32(v)) if *v == LAYOUT_VERSION),
        "WGSL LAYOUT_VERSION: {version:?}"
    );
    assert_eq!(contract().version, LAYOUT_VERSION);
}

fn frame_offset(name: &str) -> Option<usize> {
    Some(match name {
        "center_hi" => offset_of!(FrameUniform, center_hi),
        "center_lo" => offset_of!(FrameUniform, center_lo),
        "viewport" => offset_of!(FrameUniform, viewport),
        "px_per_unit" => offset_of!(FrameUniform, px_per_unit),
        "dpi" => offset_of!(FrameUniform, dpi),
        "background" => offset_of!(FrameUniform, background),
        "line_width" => offset_of!(FrameUniform, line_width),
        "srgb_target" => offset_of!(FrameUniform, srgb_target),
        _ => return None,
    })
}

fn wgsl_type_name(module: &naga::Module, ty: naga::Handle<naga::Type>) -> String {
    use naga::{ScalarKind, TypeInner, VectorSize};
    let scalar = |s: naga::Scalar| match (s.kind, s.width) {
        (ScalarKind::Float, 4) => "f32",
        (ScalarKind::Uint, 4) => "u32",
        (ScalarKind::Sint, 4) => "i32",
        _ => "?",
    };
    match module.types[ty].inner {
        TypeInner::Scalar(s) => scalar(s).to_owned(),
        TypeInner::Vector { size, scalar: s } => {
            let n = match size {
                VectorSize::Bi => 2,
                VectorSize::Tri => 3,
                VectorSize::Quad => 4,
            };
            format!("vec{n}<{}>", scalar(s))
        }
        ref other => format!("{other:?}"),
    }
}

#[test]
fn the_frame_uniform_is_laid_out_alike_in_rust_json_and_wgsl() {
    let contract = contract();
    let def = &contract.structs["Frame"];
    assert_eq!(size_of::<FrameUniform>() as u32, def.size, "Rust size");

    let (module, _) = module();
    let mut layouter = naga::proc::Layouter::default();
    layouter
        .update(module.to_ctx())
        .expect("naga lays out the types");
    let (handle, ty) = module
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some("Frame"))
        .expect("WGSL declares Frame");
    let naga::TypeInner::Struct { members, span } = &ty.inner else {
        panic!("Frame is a struct");
    };
    assert_eq!(*span, def.size, "WGSL size");
    assert_eq!(layouter[handle].size, def.size);
    assert_eq!(
        layouter[handle].alignment.round_up(1),
        def.align,
        "WGSL alignment"
    );

    assert_eq!(members.len(), def.fields.len(), "the same fields");
    for (member, field) in members.iter().zip(&def.fields) {
        assert_eq!(
            member.name.as_deref(),
            Some(field.name.as_str()),
            "field order"
        );
        assert_eq!(member.offset, field.offset, "{}: WGSL offset", field.name);
        assert_eq!(
            frame_offset(&field.name),
            Some(field.offset as usize),
            "{}: Rust offset",
            field.name
        );
        assert_eq!(
            wgsl_type_name(&module, member.ty),
            field.ty,
            "{}: type",
            field.name
        );
    }

    let [group] = contract.bind_groups.as_slice() else {
        panic!("one bind group");
    };
    let [binding] = group.bindings.as_slice() else {
        panic!("one binding");
    };
    assert_eq!(binding.kind, "uniform");
    assert_eq!(binding.struct_name, "Frame");
    assert_eq!(binding.visibility, ["vertex", "fragment"]);
    let (_, global) = module
        .global_variables
        .iter()
        .find(|(_, g)| g.name.as_deref() == Some(binding.name.as_str()))
        .expect("WGSL declares the frame uniform");
    assert_eq!(global.space, naga::AddressSpace::Uniform);
    assert_eq!(
        global.binding,
        Some(naga::ResourceBinding {
            group: group.group,
            binding: binding.binding
        })
    );
    assert_eq!(global.ty, handle);
}

fn format_name(format: wgpu::VertexFormat) -> &'static str {
    match format {
        wgpu::VertexFormat::Float32 => "float32",
        wgpu::VertexFormat::Float32x2 => "float32x2",
        wgpu::VertexFormat::Unorm8x4 => "unorm8x4",
        wgpu::VertexFormat::Uint32 => "uint32",
        _ => "?",
    }
}

/// The WGSL type an attribute of this format arrives as.
fn shader_type(format: &str) -> &'static str {
    match format {
        "float32" => "f32",
        "float32x2" => "vec2<f32>",
        "unorm8x4" => "vec4<f32>",
        "uint32" => "u32",
        _ => "?",
    }
}

/// Offsets of the named fields of each pipeline's vertex struct.
fn attribute_offset(pipeline: &str, name: &str) -> Option<usize> {
    Some(match (pipeline, name) {
        ("fill", "hi") => offset_of!(FillVertex, hi),
        ("fill", "lo") => offset_of!(FillVertex, lo),
        ("fill", "color") => offset_of!(FillVertex, color),
        ("line", "a_hi") => offset_of!(SegmentInstance, a_hi),
        ("line", "a_lo") => offset_of!(SegmentInstance, a_lo),
        ("line", "b_hi") => offset_of!(SegmentInstance, b_hi),
        ("line", "b_lo") => offset_of!(SegmentInstance, b_lo),
        ("line", "color") => offset_of!(SegmentInstance, color),
        ("marker", "hi") => offset_of!(MarkerInstance, hi),
        ("marker", "lo") => offset_of!(MarkerInstance, lo),
        ("marker", "color") => offset_of!(MarkerInstance, color),
        ("marker", "size") => offset_of!(MarkerInstance, size),
        ("marker", "shape") => offset_of!(MarkerInstance, shape),
        _ => return None,
    })
}

fn struct_size(pipeline: &str) -> Option<usize> {
    Some(match pipeline {
        "fill" => size_of::<FillVertex>(),
        "line" => size_of::<SegmentInstance>(),
        "marker" => size_of::<MarkerInstance>(),
        _ => return None,
    })
}

fn blend_of(state: Option<wgpu::BlendState>) -> Option<Blend> {
    let factor = |f: wgpu::BlendFactor| match f {
        wgpu::BlendFactor::One => "one",
        wgpu::BlendFactor::SrcAlpha => "src-alpha",
        wgpu::BlendFactor::OneMinusSrcAlpha => "one-minus-src-alpha",
        _ => "?",
    };
    let component = |c: wgpu::BlendComponent| BlendComponent {
        src_factor: factor(c.src_factor).to_owned(),
        dst_factor: factor(c.dst_factor).to_owned(),
        operation: match c.operation {
            wgpu::BlendOperation::Add => "add",
            _ => "?",
        }
        .to_owned(),
    };
    state.map(|s| Blend {
        color: component(s.color),
        alpha: component(s.alpha),
    })
}

#[test]
fn the_pipelines_and_their_vertex_layouts_agree_with_the_contract_and_the_wgsl() {
    let contract = contract();
    let (module, _) = module();
    assert_eq!(
        PIPELINES.iter().map(|p| p.name).collect::<Vec<_>>(),
        contract
            .pipelines
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>(),
    );
    for (spec, json) in PIPELINES.iter().zip(&contract.pipelines) {
        let name = spec.name;
        assert_eq!(spec.vertex, json.vertex, "{name}");
        assert_eq!(spec.fragment, json.fragment, "{name}");
        assert_eq!(
            spec.vertex_count, json.vertex_count,
            "{name}: vertices per draw"
        );
        let topology = match spec.topology {
            wgpu::PrimitiveTopology::TriangleList => "triangle-list",
            wgpu::PrimitiveTopology::TriangleStrip => "triangle-strip",
            _ => "?",
        };
        assert_eq!(topology, json.topology, "{name}");
        let blend = contract
            .blends
            .get(&json.blend)
            .expect("the blend is defined");
        assert_eq!(&blend_of(spec.blend), blend, "{name}: blend");

        assert_eq!(spec.buffers.len(), json.buffers.len(), "{name}: buffers");
        let entry = module
            .entry_points
            .iter()
            .find(|e| e.name == spec.vertex)
            .expect("the vertex entry point");
        let mut shader_inputs: Vec<(u32, String)> = entry
            .function
            .arguments
            .iter()
            .filter_map(|arg| match arg.binding {
                Some(naga::Binding::Location { location, .. }) => {
                    Some((location, wgsl_type_name(&module, arg.ty)))
                }
                _ => None,
            })
            .collect();
        shader_inputs.sort();
        let mut layout_inputs = Vec::new();

        for (buffer, jb) in spec.buffers.iter().zip(&json.buffers) {
            let step = match buffer.step_mode {
                wgpu::VertexStepMode::Vertex => "vertex",
                wgpu::VertexStepMode::Instance => "instance",
            };
            assert_eq!(step, jb.step_mode, "{name}: step mode");
            assert_eq!(buffer.array_stride, jb.array_stride, "{name}: stride");
            assert_eq!(
                struct_size(name).map(|s| s as u64),
                Some(jb.array_stride),
                "{name}: Rust struct size"
            );
            assert_eq!(
                buffer.attributes.len(),
                jb.attributes.len(),
                "{name}: attributes"
            );
            for (attribute, ja) in buffer.attributes.iter().zip(&jb.attributes) {
                assert_eq!(attribute.shader_location, ja.location, "{name}.{}", ja.name);
                assert_eq!(attribute.offset, ja.offset, "{name}.{}: offset", ja.name);
                assert_eq!(
                    format_name(attribute.format),
                    ja.format,
                    "{name}.{}",
                    ja.name
                );
                assert_eq!(
                    attribute_offset(name, &ja.name),
                    Some(ja.offset as usize),
                    "{name}.{}: Rust field offset",
                    ja.name
                );
                layout_inputs.push((ja.location, shader_type(&ja.format).to_owned()));
            }
        }
        layout_inputs.sort();
        assert_eq!(
            shader_inputs, layout_inputs,
            "{name}: {}'s inputs",
            spec.vertex
        );
    }
    assert_eq!(layout::LINE.buffers[0].array_stride, 36);
}
