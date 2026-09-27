//! Symbol pictures (the web's `ui/style/thumbs.ts` and
//! `render/symbolPreview.ts`): a symbol on a sample object
//! (`kentos_native_style::preview`), drawn by the drawing's own styled
//! pipelines and atlas in a small shader widget, so a picture is exactly
//! what the map draws (the web paints its pictures with Canvas2D). Built
//! pictures are kept by what they show: the symbol, the sample, the size,
//! the palette and the library's version. The GPU keeps a picture's
//! buffers while it is on the screen and lets them go two frames after.

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use iced::widget::{container, shader, space};
use iced::{Element, Rectangle, mouse, wgpu};
use kentos_native_style::StylePalette;
use kentos_native_style::library::StyleLibrary;
use kentos_native_style::preview::{Geometry, preview};
use kentos_native_style::renderer::ref_id;
use kentos_render_wgpu::styled::{StyledLayerPart, StyledScene};
use kentos_render_wgpu::{Camera, FrameInput, RenderSettings, Rgba8, ScenePart, Vec2, ViewId};
use serde_json::Value;

use super::images::Images;
use crate::viewport::Pipeline;

/// Pictures are drawn multisampled: small shapes want smooth edges.
const SAMPLES: u32 = 4;
/// More built pictures than this and the cache starts again.
const KEEP: usize = 4096;

/// Views of pictures, far from the drawing area's own ids.
static NEXT_VIEW: AtomicU64 = AtomicU64::new(1 << 48);
static NEXT_PART: AtomicU64 = AtomicU64::new(1 << 48);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Key {
    symbol: String,
    geometry: Option<Geometry>,
    size: (u32, u32),
    px_per_mm: Option<u64>,
    palette: String,
    library: u64,
}

/// A picture's styled layer and the scale it is drawn at.
pub struct Built {
    part: Arc<StyledLayerPart>,
    px_per_mm: f64,
    /// An image of it waits in the atlas for another frame.
    pending: AtomicBool,
}

/// The pictures built so far.
#[derive(Default)]
pub struct Thumbs {
    built: Mutex<HashMap<Key, Option<Arc<Built>>>>,
}

impl fmt::Debug for Thumbs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Thumbs").finish_non_exhaustive()
    }
}

/// What a picture shows.
pub struct Look<'a> {
    pub palette: &'a StylePalette,
    pub library: &'a StyleLibrary,
    pub images: &'a Arc<Images>,
}

impl Thumbs {
    /// The picture of `symbol` (a library reference is drawn as its symbol),
    /// `size` logical pixels; `px_per_mm` fixes the scale (a legend's), else
    /// the symbol is fitted. Paper-coloured and empty when there is nothing to draw.
    pub fn picture<'a, M: 'a>(
        &self,
        symbol: &Value,
        geometry: Option<Geometry>,
        size: (f32, f32),
        px_per_mm: Option<f64>,
        look: &Look<'_>,
    ) -> Element<'a, M> {
        let background = Rgba8::parse_hex(&look.palette.paper).unwrap_or(Rgba8::rgb(255, 255, 255));
        let resolved = match ref_id(symbol) {
            Some(id) => look.library.symbol(id).cloned(),
            None => Some(symbol.clone()),
        };
        let built = resolved.and_then(|s| self.built(&s, geometry, size, px_per_mm, look));
        match built {
            Some(built) => shader(Program {
                built,
                background,
                images: look.images.clone(),
            })
            .width(size.0)
            .height(size.1)
            .into(),
            None => {
                let [r, g, b, _] = background.0;
                container(space())
                    .width(size.0)
                    .height(size.1)
                    .style(move |_| container::Style {
                        background: Some(iced::Color::from_rgb8(r, g, b).into()),
                        ..container::Style::default()
                    })
                    .into()
            }
        }
    }

    fn built(
        &self,
        symbol: &Value,
        geometry: Option<Geometry>,
        size: (f32, f32),
        px_per_mm: Option<f64>,
        look: &Look<'_>,
    ) -> Option<Arc<Built>> {
        let p = look.palette;
        let key = Key {
            symbol: symbol.to_string(),
            geometry,
            size: (size.0.to_bits(), size.1.to_bits()),
            px_per_mm: px_per_mm.map(f64::to_bits),
            palette: format!("{}{}{}{}", p.fg, p.fg_dim, p.ink, p.paper),
            library: look.library.version(),
        };
        let mut built = self.built.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(hit) = built.get(&key) {
            return hit.clone();
        }
        if built.len() >= KEEP {
            built.clear();
        }
        let made = preview(
            symbol,
            geometry,
            (f64::from(size.0), f64::from(size.1)),
            px_per_mm,
            look.palette,
            look.library,
        )
        .ok()
        .map(|p| {
            Arc::new(Built {
                part: Arc::new(StyledLayerPart {
                    id: NEXT_PART.fetch_add(1, Ordering::Relaxed),
                    layer: p.layer,
                }),
                px_per_mm: p.px_per_mm,
                pending: AtomicBool::new(false),
            })
        });
        built.insert(key, made.clone());
        made
    }
}

struct Program {
    built: Arc<Built>,
    background: Rgba8,
    images: Arc<Images>,
}

/// A picture's view in the renderer: one per widget on the screen.
pub struct State {
    id: ViewId,
}

impl Default for State {
    fn default() -> Self {
        State {
            id: NEXT_VIEW.fetch_add(1, Ordering::Relaxed),
        }
    }
}

impl<M> shader::Program<M> for Program {
    type State = State;
    type Primitive = Frame;

    fn update(
        &self,
        _state: &mut State,
        event: &iced::Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<shader::Action<M>> {
        // The atlas left an image of this picture for another frame.
        match event {
            iced::Event::Window(iced::window::Event::RedrawRequested(_))
                if self.built.pending.load(Ordering::Relaxed) =>
            {
                Some(shader::Action::request_redraw())
            }
            _ => None,
        }
    }

    fn draw(&self, state: &State, _cursor: mouse::Cursor, _bounds: Rectangle) -> Frame {
        Frame {
            id: state.id,
            built: self.built.clone(),
            background: self.background,
            images: self.images.clone(),
        }
    }
}

/// One frame of a picture, handed to the drawing's renderer.
pub struct Frame {
    id: ViewId,
    built: Arc<Built>,
    background: Rgba8,
    images: Arc<Images>,
}

impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Frame")
            .field("id", &self.id)
            .field("part", &self.built.part.id)
            .finish_non_exhaustive()
    }
}

impl shader::Primitive for Frame {
    type Pipeline = Pipeline;

    fn prepare(
        &self,
        pipeline: &mut Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        viewport: &shader::Viewport,
    ) {
        let Some(renderer) = pipeline.renderer_mut() else {
            return;
        };
        let scale = viewport.scale_factor();
        let camera = Camera {
            center: Vec2::new(0.0, 0.0),
            scale: self.built.px_per_mm,
            width: f64::from(bounds.width),
            height: f64::from(bounds.height),
        };
        let mut settings = RenderSettings::new(self.background);
        settings.samples = SAMPLES;
        let parts: [&ScenePart; 0] = [];
        let frame = FrameInput {
            camera: &camera,
            origin: Vec2::new(0.0, 0.0),
            size_px: [bounds.width * scale, bounds.height * scale],
            origin_px: [bounds.x * scale, bounds.y * scale],
            scale_factor: f64::from(scale),
            settings: &settings,
        };
        if renderer
            .prepare(device, queue, self.id, &parts, &frame)
            .is_err()
        {
            return;
        }
        let scene = StyledScene {
            layers: vec![self.built.part.clone()],
            under: 0,
        };
        let pending = renderer.prepare_styled(device, queue, self.id, &scene, &*self.images);
        self.built.pending.store(pending, Ordering::Relaxed);
    }

    fn draw(&self, pipeline: &Pipeline, render_pass: &mut wgpu::RenderPass<'_>) -> bool {
        match pipeline.renderer() {
            Some(renderer) if renderer.owns_targets(self.id) => false,
            Some(renderer) => {
                renderer.draw(render_pass, self.id);
                true
            }
            None => true,
        }
    }

    fn render(
        &self,
        pipeline: &Pipeline,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        if let Some(renderer) = pipeline.renderer() {
            renderer.render(
                encoder,
                target,
                [
                    clip_bounds.x,
                    clip_bounds.y,
                    clip_bounds.width,
                    clip_bounds.height,
                ],
                self.id,
            );
        }
    }
}
