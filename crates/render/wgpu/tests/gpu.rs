//! The drawing pipeline on a real GPU (TODOS.md REN-02, REN-07): the shared
//! WGSL built into pipelines by the device's own driver, a frame drawn and
//! read back. Test only: the renderer itself never reads the GPU back.
//!
//! Needs a GPU, so it runs only with `KENTOS_GPU_TESTS=1`; otherwise it says
//! it was skipped, which is not a pass. One device, one test function: several
//! devices opened at once have crashed GPU drivers (docs/adr/0016).

use kentos_contracts::{
    DocumentSnapshotV1, Entity, EntityBase, LayerNode, LayerNodeType, LayerStyle, LineEntity,
    LineType,
};
use kentos_render_wgpu::camera::MAX_SCALE;
use kentos_render_wgpu::scene::{self, lod};
use kentos_render_wgpu::{
    Camera, FrameInput, Palette, RenderSettings, Renderer, Rgba8, ScenePart, Vec2,
};

const SAMPLE: &str = include_str!("../../../../fixtures/document/v1/sample.json");
const E: f64 = 487_012.346;
const N: f64 = 4_420_187.521;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: Renderer,
}

impl Gpu {
    fn open() -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::from_env().unwrap_or(wgpu::Backends::PRIMARY),
            flags: wgpu::InstanceFlags::empty(),
            ..wgpu::InstanceDescriptor::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        }))
        .expect("a GPU adapter");
        let info = adapter.get_info();
        eprintln!(
            "GPU: {} ({:?}, {})",
            info.name, info.backend, info.driver_info
        );
        // The limits Iced asks for (iced_wgpu's headless renderer).
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("kentos-render-wgpu test"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits {
                max_bind_groups: 2,
                ..wgpu::Limits::default()
            },
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
        }))
        .expect("a device");
        let renderer = Renderer::new(&device, FORMAT).expect("the pipelines build");
        Self {
            device,
            queue,
            renderer,
        }
    }

    /// Draws `parts` through `camera` into a width × height target and reads it back as RGBA.
    fn draw(
        &mut self,
        parts: &[&ScenePart],
        camera: &Camera,
        settings: &RenderSettings,
        width: u32,
        height: u32,
    ) -> Vec<u8> {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let origin = parts.first().map_or(Vec2::new(0.0, 0.0), |p| p.origin);
        self.renderer
            .prepare(
                &self.device,
                &self.queue,
                1,
                parts,
                &FrameInput {
                    camera,
                    origin,
                    size_px: [width as f32, height as f32],
                    scale_factor: 1.0,
                    settings,
                },
            )
            .expect("the parts upload");
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::RED),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.renderer.draw(&mut pass, 1);
        }
        let row = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(row * height),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let index = self.queue.submit([encoder.finish()]);
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(index),
                timeout: None,
            })
            .expect("the frame finishes");
        let data = slice.get_mapped_range();
        let mut out = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            let start = (y * row) as usize;
            out.extend_from_slice(&data[start..start + (width * 4) as usize]);
        }
        out
    }
}

fn palette() -> Palette {
    Palette {
        background: Rgba8::rgb(0, 0, 0),
        fg: Rgba8::rgb(255, 255, 255),
        fg_dim: Rgba8::rgb(160, 160, 160),
        ink: Rgba8::rgb(255, 255, 255),
    }
}

/// A drawing of one white north–south line through x = `x`, 100 m long.
fn one_line(x: f64) -> DocumentSnapshotV1 {
    let mut doc = DocumentSnapshotV1::from_json(SAMPLE).expect("the sample reads");
    doc.origin = kentos_contracts::Vec2 {
        x: 486_512.34,
        y: 4_420_187.52,
    };
    doc.layers = vec![LayerNode {
        id: "l".into(),
        name: "Çizgi".into(),
        kind: LayerNodeType::Layer,
        visible: true,
        locked: false,
        expanded: false,
        style: LayerStyle {
            color: "#ffffff".into(),
            line_type: LineType::Continuous,
            line_weight: 0.25,
            fill: None,
            point: None,
            label: None,
            pick_interior: None,
            renderer: None,
        },
        children: Vec::new(),
    }];
    doc.entities = vec![Entity::Line(LineEntity {
        base: EntityBase {
            id: 1,
            layer_id: "l".into(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
        },
        a: kentos_contracts::Vec2 { x, y: N - 50.0 },
        b: kentos_contracts::Vec2 { x, y: N + 50.0 },
    })];
    doc
}

/// Coverage-weighted centre (in pixels from the left edge) of the white line in row `y`.
fn line_centre(rgba: &[u8], width: u32, y: u32) -> f64 {
    let (mut sum, mut weight) = (0.0, 0.0);
    for x in 0..width {
        let v = f64::from(rgba[((y * width + x) * 4) as usize]) / 255.0;
        sum += (f64::from(x) + 0.5) * v;
        weight += v;
    }
    assert!(weight > 0.5, "the line is drawn in row {y}");
    sum / weight
}

#[test]
fn on_a_real_gpu() {
    if std::env::var("KENTOS_GPU_TESTS").as_deref() != Ok("1") {
        eprintln!(
            "SKIPPED: set KENTOS_GPU_TESTS=1 to run the GPU test (a skipped test is not a pass)"
        );
        return;
    }
    let mut gpu = Gpu::open();
    sub_pixel_pans_at_tm_coordinates_move_the_line_by_exactly_the_pan(&mut gpu);
    the_sample_draws_with_every_pipeline_and_the_cache_uploads_once(&mut gpu);
}

/// REN-07 on the device: at 0.2 mm per pixel, around E 487 012 / N 4 420 187,
/// the view pans by 0.01 mm (0.05 px) at a time; the line must move by
/// exactly that, with no jitter.
fn sub_pixel_pans_at_tm_coordinates_move_the_line_by_exactly_the_pan(gpu: &mut Gpu) {
    let (width, height) = (64, 4);
    let line_x = E + 0.000_03;
    let doc = one_line(line_x);
    let origin = scene::scene_origin(&doc);
    let part = scene::build_fixed(&doc, &palette(), origin);
    let settings = RenderSettings::new(Rgba8::rgb(0, 0, 0));
    let mut worst: f64 = 0.0;
    let mut last: Option<f64> = None;
    for i in 0..40 {
        let camera = Camera {
            center: Vec2::new(E + 0.000_01 * f64::from(i), N),
            scale: MAX_SCALE,
            width: f64::from(width),
            height: f64::from(height),
        };
        let rgba = gpu.draw(&[&part], &camera, &settings, width, height);
        let centre = line_centre(&rgba, width, 2);
        let expected = f64::from(width) / 2.0 + (line_x - camera.center.x) * MAX_SCALE;
        worst = worst.max((centre - expected).abs());
        if let Some(last) = last {
            let step = centre - last;
            assert!(
                (step + 0.05).abs() < 0.02,
                "pan {i}: the line moved {step} px, not -0.05"
            );
        }
        last = Some(centre);
    }
    eprintln!("REN-07 on the GPU: worst line position error {worst:.4} px over 40 pans of 0.01 mm");
    // 8-bit coverage limits the reading to about 1/255 px.
    assert!(
        worst < 0.02,
        "the line strays {worst} px from where float64 puts it"
    );
}

fn the_sample_draws_with_every_pipeline_and_the_cache_uploads_once(gpu: &mut Gpu) {
    let mut doc = DocumentSnapshotV1::from_json(SAMPLE).expect("the sample reads");
    // Every layer visible, the parcel filled: fills, lines and marks all draw.
    fn show(nodes: &mut [LayerNode]) {
        for node in nodes {
            node.visible = true;
            if node.id == "parsel" {
                node.style.fill = Some("#E06C7560".into());
            }
            show(&mut node.children);
        }
    }
    show(&mut doc.layers);
    let origin = scene::scene_origin(&doc);
    let (width, height) = (320, 240);
    let mut camera = Camera {
        width: f64::from(width),
        height: f64::from(height),
        ..Camera::default()
    };
    camera.fit(&scene::extents(&doc).expect("extents"), 16.0);
    let settings = RenderSettings::new(Rgba8::rgb(0x14, 0x1a, 0x21));
    let fixed = scene::build_fixed(&doc, &palette(), origin);
    let tolerance = lod::tolerance(lod::build_band(camera.scale, settings.curve_tolerance_px));
    let curves = scene::build_curves(
        &doc,
        &palette(),
        origin,
        tolerance,
        settings.curve_segment_budget,
    );

    let rgba = gpu.draw(&[&fixed, &curves], &camera, &settings, width, height);
    let stats = gpu.renderer.stats(1).expect("stats");
    assert!(
        stats.segments > 0 && stats.triangles > 0 && stats.markers == 1,
        "{stats:?}"
    );
    assert!(
        stats.uploaded_bytes > 64,
        "the first frame uploads the parts"
    );
    // The background covers the target (it was cleared red).
    assert_eq!(&rgba[0..4], &[0x14, 0x1a, 0x21, 0xff]);
    // The parcel's red outline shows somewhere.
    let red = rgba
        .chunks(4)
        .any(|p| p[0] > 0xc0 && p[1] < 0x90 && p[2] < 0x90);
    assert!(red, "the parcel outline is drawn");

    // Another frame of the same parts, the view moved: only the uniform goes up.
    camera.pan_by(3.0, -2.0);
    let _ = gpu.draw(&[&fixed, &curves], &camera, &settings, width, height);
    let again = gpu.renderer.stats(1).expect("stats");
    assert_eq!(again.uploaded_bytes, 64, "{again:?}");
    assert_eq!(again.resident_bytes, stats.resident_bytes);

    // A view left undrawn is released with its buffers.
    for _ in 0..=kentos_render_wgpu::renderer::KEEP_IDLE_FRAMES {
        gpu.renderer.trim();
    }
    assert_eq!(gpu.renderer.view_count(), 0);
}
