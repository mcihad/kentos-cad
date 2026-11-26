//! The drawing area's state: the scene and the styled layers built and kept, the grid, the views a
//! tool or the history asks for, fits, and the settling of a move or a zoom.

use super::*;

impl Viewport {
    pub fn new() -> Self {
        Self {
            camera: Camera::default(),
            live: super::live::LiveView::new(Camera::default()),
            live_sent: std::cell::Cell::new(Camera::default()),
            pointer_px: None,
            sheet_saved_camera: None,
            cursor: None,
            bounds: Rectangle::default(),
            sized: false,
            fit_pending: false,
            opened_extent: None,
            generation: 0,
            id: NEXT_VIEW.fetch_add(1, Ordering::Relaxed),
            scene: RefCell::new(None),
            highlight: RefCell::new(highlight::Highlighted::default()),
            grid_shown: true,
            grid: RefCell::new(None),
            underlay: None,
            underlay_part: RefCell::new(None),
            status: Arc::new(Mutex::new(Status::default())),
            styled: RefCell::new(crate::style::scene::StyledCache::default()),
            deep_styled: RefCell::new(crate::style::scene::StyledCache::default()),
            deep: std::cell::Cell::new(None),
            line_types: RefCell::new(None),
            phasings: RefCell::new(None),
            zoomed_at: None,
            moved_at: None,
            fresh: false,
            other: None,
            lost_seen: std::cell::Cell::new(0),
            passive: false,
            theme: None,
            themed: RefCell::new(None),
            time: crate::split_view::ViewTime::Follow,
            wall_view: None,
        }
    }

    /// What the built scene and styled layers hold, bytes: the styled layers' vertex data, residuals and batches
    /// (the view's and the deep view's), and the plain scene's fills, segments and markers (`perf::memory`).
    #[cfg(test)]
    pub(crate) fn built_bytes(&self) -> [usize; 6] {
        let [a, b, c] = self.styled.borrow().held_bytes();
        let [d, e, f] = self.deep_styled.borrow().held_bytes();
        fn bytes<T>(v: &Vec<T>) -> usize {
            v.capacity() * std::mem::size_of::<T>()
        }
        let mut plain = [0; 3];
        if let Some(cached) = self.scene.borrow().as_ref() {
            for part in [&cached.fixed, &cached.curves, &cached.construction] {
                plain[0] += bytes(&part.fills);
                plain[1] += bytes(&part.segments);
                plain[2] += bytes(&part.markers);
            }
        }
        [a + d, b + e, c + f, plain[0], plain[1], plain[2]]
    }

    /// Forgets the scene and the styled layers built: a measurement's way to see what they hold (`perf::memory`).
    #[cfg(test)]
    pub(crate) fn forget_built(&self) {
        *self.scene.borrow_mut() = None;
        *self.styled.borrow_mut() = crate::style::scene::StyledCache::default();
        *self.deep_styled.borrow_mut() = crate::style::scene::StyledCache::default();
        *self.highlight.borrow_mut() = highlight::Highlighted::default();
        *self.grid.borrow_mut() = None;
    }

    /// The grid for the view now: the one built while it still covers the
    /// view with this zoom's spacing, origin and ground, so panning and
    /// zooming inside it upload nothing; else new lines three views wide
    /// around the view (the web's `gridExtent`).
    pub(super) fn grid(&self, canvas: Canvas, origin: Vec2, frame: (Vec2, f64)) -> Arc<ScenePart> {
        // On a map of another system the grid is the map's (docs/adr/0583 §3).
        let view = Camera {
            map: None,
            ..self.camera
        }
        .visible_bounds();
        let spacing = scene::grid_spacing(self.camera.map_px(), GRID_MIN_PX);
        let mut cache = self.grid.borrow_mut();
        let covers = cache.as_ref().is_some_and(|g| {
            g.spacing == spacing
                && g.origin == origin
                && g.frame == frame
                && g.canvas == canvas
                && view.min_x >= g.area.min_x
                && view.min_y >= g.area.min_y
                && view.max_x <= g.area.max_x
                && view.max_y <= g.area.max_y
        });
        if !covers {
            let (w, h) = (view.max_x - view.min_x, view.max_y - view.min_y);
            let area = Bounds {
                min_x: view.min_x - w,
                min_y: view.min_y - h,
                max_x: view.max_x + w,
                max_y: view.max_y + h,
            };
            *cache = Some(Grid {
                area,
                spacing,
                origin,
                frame,
                canvas,
                part: Arc::new(scene::build_grid_framed(
                    &area,
                    spacing,
                    origin,
                    grid_colors(canvas),
                    frame.0,
                    frame.1,
                )),
            });
        }
        cache.as_ref().map_or_else(Arc::default, |g| g.part.clone())
    }

    /// The underlay models' plan at the active level (docs/adr/8101): its version and lines; drawn under the drawing.
    pub fn set_underlay(&mut self, plan: (u64, Arc<Vec<kentos_ifc::underlay::PlanLine>>)) {
        self.underlay = (!plan.1.is_empty()).then_some(plan);
    }

    /// The underlay models' plan as a part in `palette`'s faint ink: the cut lines whole, the edges under the cut at
    /// 45 %; built again when the plan, the origin, the colours or the map changed.
    fn underlay_part(&self, origin: Vec2, palette: &kentos_render_wgpu::Palette) -> Arc<ScenePart> {
        let Some((version, lines)) = &self.underlay else {
            return Arc::default();
        };
        let ink = palette.fg_dim;
        let colors = [ink, Rgba8([ink.0[0], ink.0[1], ink.0[2], 115])];
        let map = self.camera.map;
        let mut kept = self.underlay_part.borrow_mut();
        if let Some((v, o, c, m, part)) = kept.as_ref()
            && *v == *version
            && *o == origin
            && *c == colors
            && m.map(std::ptr::from_ref) == map.map(std::ptr::from_ref)
        {
            return part.clone();
        }
        let at = |p: [f64; 2]| match map {
            Some(_) => self.camera.to_map(Vec2::new(p[0], p[1])),
            None => Vec2::new(p[0], p[1]),
        };
        let (mut cut, mut under) = (Vec::new(), Vec::new());
        for l in lines.iter() {
            let pair = [at(l.a), at(l.b)];
            if l.cut { cut.push(pair) } else { under.push(pair) }
        }
        let part = Arc::new(scene::build_lines(origin, &[(&under, colors[1]), (&cut, colors[0])]));
        *kept = Some((*version, origin, colors, map, part.clone()));
        part
    }

    /// The camera's zoom limits follow the project's units ([`unit_metres`]).
    pub(crate) fn follow_units(&mut self, settings: &kentos_commands::ProjectSettings) {
        self.camera.metres_per_unit = unit_metres(settings);
    }

    /// What the view shows is another drawing now (docs/adr/0576 §3): its scene is built afresh, its camera stays.
    pub(crate) fn rebuild(&mut self) {
        self.generation += 1;
    }

    /// The map's system the view draws in (docs/adr/0583 §3): the scene is built afresh in it, the world point in
    /// the middle and the pointer stay where they are; whether it changed.
    pub(crate) fn set_map(
        &mut self,
        map: Option<&'static kentos_geometry_core::crs::map::MapProjection>,
    ) -> bool {
        let same = match (self.camera.map, map) {
            (Some(a), Some(b)) => std::ptr::eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if same {
            return false;
        }
        let at = self.cursor.map(|c| self.camera.world_to_screen(c));
        self.camera = self.camera.with_map(map);
        self.cursor = at.map(|[x, y]| self.camera.screen_to_world(x, y));
        self.generation += 1;
        true
    }

    /// The origin the scene's numbers are relative to: the drawing's, in the map's system when it has one.
    pub(crate) fn draw_origin(&self, doc: &Document) -> Vec2 {
        let o = scene::scene_origin(doc);
        match self.camera.map {
            Some(_) => {
                let m = self.camera.to_map(o);
                if m.x.is_finite() && m.y.is_finite() {
                    m
                } else {
                    o
                }
            }
            None => o,
        }
    }

    /// A drawing was opened: its scene is built afresh and the view goes to
    /// its start view, else to `extent`, its box as its store has it (the
    /// web's `replaceDrawing`); without one, to its objects' boxes.
    pub fn opened(&mut self, doc: &Document, extent: Option<Bounds>) {
        self.sheet_saved_camera=None;
        self.generation += 1;
        self.cursor = None;
        self.fit_pending = true;
        self.opened_extent = extent;
        // A drawing opens north up (UR2-046: the last drawing's turn went on in the next one): the start view it
        // keeps has no turn; a tab brought in front gets its own camera back after this (doc_tabs.rs).
        self.camera.twist = 0.0;
        // Its zoom limits in its own units before the fit: a drawing in degrees fits as closely as one in metres.
        self.follow_units(doc.model.settings());
        if self.sized {
            self.fit(doc);
        }
    }

    pub fn update(&mut self, event: Event, doc: Option<&Document>) {
        match event {
            Event::Resized(bounds) => {
                self.bounds = bounds;
                self.camera
                    .set_size(f64::from(bounds.width), f64::from(bounds.height));
                self.sized = true;
                if self.fit_pending
                    && let Some(doc) = doc
                {
                    self.fit(doc);
                }
            }
            Event::Moved(at) => {
                self.pointer_px = Some(at);
                self.cursor = Some(self.world(at));
            }
            Event::Left => {
                self.pointer_px = None;
                self.cursor = None;
            }
            Event::Panned { by, at } => {
                self.camera.pan_by(f64::from(by.x), f64::from(by.y));
                self.cursor = Some(self.world(at));
                self.moved_at = Some(Instant::now());
            }
            Event::Zoomed { factor, at } => {
                self.camera
                    .zoom_at(factor, f64::from(at.x), f64::from(at.y));
                self.cursor = Some(self.world(at));
                self.zoomed_at = Some(Instant::now());
                self.moved_at = self.zoomed_at;
            }
            // A gesture the area made on its live camera, settled (live.rs): its camera is the app's now, and the
            // pointer's world point is the one under the same place of the screen. The view has already been still
            // for the labels' settle: they and screen-sized symbols are placed for it at once. A pan's drag moved the
            // pointer with the drawing and no move told it: it is where the drag left it, off the area past its edge
            // (docs/adr/9251). Read through the camera it stopped at, it is the world point the drag held under it,
            // so a running tool's preview still ends at the pointer.
            Event::ViewChanged { to, at, .. } => {
                self.camera = to;
                self.live_sent.set(to);
                if let Some(at) = at {
                    let inside = at.x >= 0.0
                        && at.y >= 0.0
                        && at.x < self.bounds.width
                        && at.y < self.bounds.height;
                    self.pointer_px = inside.then_some(at);
                }
                self.cursor = self.pointer_px.map(|at| self.world(at));
            }
            // The app fits the store's box (`App::zoom_extents`); this is its objects' boxes.
            Event::Extents => {
                if let Some(extents) = doc.and_then(scene::extents) {
                    let at = self.cursor.map(|c| self.camera.world_to_screen(c));
                    self.camera.fit(&extents, FIT_PADDING);
                    self.cursor = at.map(|[x, y]| self.camera.screen_to_world(x, y));
                }
            }
            Event::CenterOn(p) => self.camera.center_on(p),
            // The app gives these to the tool session (app.rs); the pointer is there too.
            Event::Pressed(at)
            | Event::RightPressed(at)
            | Event::RightHeld(at)
            | Event::RightClick(at) => {
                self.cursor = Some(self.world(at));
            }
            // It may come from off the area; the last move placed the pointer.
            Event::Released(_) => {}
            Event::Settled => {
                self.zoomed_at = None;
                self.moved_at = None;
            }
        }
    }

    /// What the last frame drew, or why it could not.
    pub fn status(&self) -> Status {
        self.status
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Tests: what a frame would report about the device (no GPU in unit tests).
    #[cfg(test)]
    pub fn set_status_for_tests(&self, supported: Vec<u32>, failure: Option<SampleFailure>) {
        let mut status = self.status.lock().unwrap_or_else(PoisonError::into_inner);
        status.supported = supported;
        status.failure = failure;
    }

    /// Objects on shown layers the area does not draw yet, by kind (text,
    /// dimensions, construction lines): said, not hidden.
    pub fn not_drawn(&self) -> BTreeMap<&'static str, usize> {
        self.scene
            .try_borrow()
            .ok()
            .and_then(|cache| cache.as_ref().map(|c| c.fixed.not_drawn.clone()))
            .unwrap_or_default()
    }

    /// The drawing area showing `doc`, drawn as `graphics` says, with the
    /// selection and the hovered object highlighted in `accent` (docs/adr/0029)
    /// and the pointer looking as the running tool asks (docs/adr/0056).
    /// A change of either value reaches the next frame: new targets on the
    /// same device, nothing reopened (TODOS.md AA-02).
    #[allow(clippy::too_many_arguments)]
    pub fn view<'a>(
        &'a self,
        doc: &Document,
        canvas: impl Into<Canvas>,
        graphics: Graphics,
        selection: &Selection,
        accent: Rgba8,
        cursor: Cursor,
        styling: Option<Styling<'_>>,
    ) -> Element<'a, Message> {
        let canvas = canvas.into();
        let palette = palette(canvas);
        let settings = RenderSettings {
            samples: graphics.samples,
            hi_dpi: graphics.hi_dpi,
            highlight_width: graphics.highlight_width as f32,
            ..RenderSettings::new(palette.background)
        };
        // Every shown layer through the style engine, as on the web (docs/adr/0090);
        // without the library, the plain scene of before.
        let styled = styling.map(|s| self.styled_scene(doc, &palette, s));
        let (origin, fixed, curves, construction, clip) =
            self.scene_with(doc, canvas, &palette, &settings, styled.is_none());
        let spatial = styling.map(|s| s.spatial);
        let (selected, hovered) =
            self.highlights(doc, selection, accent, &fixed, &curves, &clip, spatial);
        // The underlay models' plan over the grid, under the drawing (docs/adr/8101).
        let underlay = self.underlay_part(origin, &palette);
        // The grid under everything (Izgara, F7).
        let grid = if self.grid_shown {
            let u = doc.settings().ucs.unwrap_or(kentos_commands::Ucs::WORLD);
            let at = match self.camera.map {
                Some(_) => self.camera.to_map(Vec2::new(u.origin.x, u.origin.y)),
                None => Vec2::new(u.origin.x, u.origin.y),
            };
            self.grid(canvas, origin, (at, u.angle))
        } else {
            Arc::default()
        };
        // What the picture shows apart from the camera (docs/adr/1068): a zoom while the wheel turns scales the last
        // sharp picture only while this stays. The hovered object is drawn over the picture, so it is not in it; the
        // view modes and the style library change in dialogs, never while the wheel turns.
        let drawing = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            (doc.session, doc.model.generation(), selection.version(), self.grid_shown).hash(&mut h);
            (palette.background.0, accent.0, settings.samples, settings.hi_dpi).hash(&mut h);
            settings.highlight_width.to_bits().hash(&mut h);
            styling.map(|s| (s.screen_symbols, s.line_weights)).hash(&mut h);
            (!self.fresh && graphics.zoom_scaled && ZOOM_SCALED.with(std::cell::Cell::get)).then(|| h.finish())
        };
        let area: Element<'a, Message> = shader(Program {
            id: self.id,
            document: doc.session,
            parts: [grid, underlay, fixed, curves, construction, selected, hovered],
            origin,
            settings,
            status: self.status.clone(),
            cursor,
            styled: styled.unwrap_or_default(),
            images: styling.map(|s| s.styles.images.clone()),
            settle_at: styling
                .filter(|s| {
                    s.screen_symbols || self.styled_in_use().try_borrow().is_ok_and(|c| c.scales_with_view())
                })
                .and_then(|_| self.settling())
                .max(self.moved_until()),
            moving: self.moving(),
            drawing,
            keep: !self.fresh,
            live: self.live_for_view(),
            other: self.other,
            passive: self.passive,
            glyphs: styling.map(|s| s.glyphs.to_vec()).unwrap_or_default(),
        })
        .width(Fill)
        .height(Fill)
        .into();
        match self.status().error {
            None => area,
            Some(error) => stack![
                area,
                container(EmptyState::error("Çizim alanı çizilemedi").description(format!(
                    "KentOS'un wgpu çizim hattı: {error}. Çizim, katmanlar ve kayıt etkilenmedi. \
                     Ekran kartı sürücüsünü güncelleyin ya da WGPU_BACKEND=gl ile başka bir \
                     sürücü yolu deneyin."
                )))
                .center(Fill)
            ]
            .into(),
        }
    }

    /// The tree the view shows (docs/adr/0523 §1): the theme's, made again only when the drawing or the theme
    /// changed; none without a theme.
    pub(crate) fn themed_tree(
        &self,
        doc: &Document,
    ) -> Option<std::sync::Arc<kentos_domain::LayerTree>> {
        let theme = self.theme.as_ref()?;
        let key = (doc.session, doc.model.generation());
        let mut kept = self.themed.borrow_mut();
        if let Some((k, t, tree)) = kept.as_ref()
            && *k == key
            && t == theme
        {
            return Some(std::sync::Arc::clone(tree));
        }
        let tree = std::sync::Arc::new(kentos_domain::layer_states::themed(
            doc.model.layers(),
            theme,
        ));
        *kept = Some((key, theme.clone(), std::sync::Arc::clone(&tree)));
        Some(tree)
    }

    /// The drawing's styled layers (docs/adr/0090), from the cache where nothing they depend on changed.
    pub(crate) fn styled_scene(
        &self,
        doc: &Document,
        palette: &Palette,
        s: Styling<'_>,
    ) -> StyledScene {
        let hex = |c: Rgba8| format!("#{:02X}{:02X}{:02X}", c.0[0], c.0[1], c.0[2]);
        let key = (doc.session, doc.model.generation());
        let line_types = {
            let mut kept = self.line_types.borrow_mut();
            match kept.as_ref() {
                Some((k, t)) if *k == key => std::sync::Arc::clone(t),
                _ => {
                    let t = std::sync::Arc::new(
                        kentos_native_style::line_types::LineTypes::with_resources(
                            doc.model.settings(),
                            doc.model.styles(),
                        ),
                    );
                    *kept = Some((key, std::sync::Arc::clone(&t)));
                    t
                }
            }
        };
        // The phase view's looks (docs/adr/0346 §2) and Kat görünümü's (docs/adr/0715 §2): the levels it hides left
        // out, the one under the active level faint; made again only when the drawing or the level view changed.
        let (level_version, level_looks) = s.spatial.level_looks();
        let phasing = {
            let mut kept = self.phasings.borrow_mut();
            let at = (key.0, key.1, level_version);
            match kept.as_ref() {
                Some((k, p)) if *k == at => std::sync::Arc::clone(p),
                _ => {
                    let mut p = kentos_plot::layers::phasing_of(&doc.model);
                    for &id in &level_looks.hidden {
                        p.hide(id);
                    }
                    for &id in &level_looks.faded {
                        p.fade(id);
                    }
                    // Seen from another level (docs/adr/8012, step 3): the store's look; a change of it builds again.
                    for &id in level_looks
                        .upper
                        .iter()
                        .chain(&level_looks.overhead)
                        .chain(&level_looks.roofs)
                    {
                        p.drawn_across(id);
                    }
                    let p = std::sync::Arc::new(p);
                    *kept = Some((at, std::sync::Arc::clone(&p)));
                    p
                }
            }
        };
        // Deep, the view's own scene of the objects round it, their paths split at its box (plan/render.md RM-24).
        let deep = self.deep_box();
        let look = crate::style::scene::Look {
            phasing,
            line_types,
            palette: kentos_native_style::StylePalette {
                fg: hex(palette.fg),
                fg_dim: hex(palette.fg_dim),
                ink: hex(palette.ink),
                paper: hex(palette.background),
            },
            // Widths and sizes in the map's units: a map unit is `map_ratio` world units (docs/adr/0583 §3).
            symbol_scale: self.symbol_scale(doc, s.screen_symbols) / self.camera.map_ratio,
            screen: s.screen_symbols,
            hairlines: !s.line_weights,
            // Paper widths held on the model screen (docs/adr/0793 §1).
            hold_weights: true,
            origin: self.draw_origin(doc),
            view_build: kentos_native_style::View {
                map: self.camera.map,
                deep,
                // Curves round to a quarter pixel at every zoom short of the deep view, which rounds its own
                // (plan/render.md RM-09): the tolerance at its threshold, in world units.
                curve_tolerance: Some(DEEP_TOLERANCE_PX / (DEEP_PX_PER_M * self.metres_a_unit())),
                ..s.view_build
            },
            view_colors: s.view_colors,
            // What a renderer compares is read in the project's drawing unit, as Katman stili listed it.
            reading_unit: kentos_native_style::reading_unit(doc.model.settings()),
        };
        // A view with a theme draws the layers as it shows them (docs/adr/0523 §1).
        let themed = self.themed_tree(doc);
        let nodes = themed
            .as_deref()
            .map_or(doc.model.layers().nodes(), |t| t.nodes());
        // The data layers shown read what the view shows from their sources (docs/adr/9101 §6).
        let data_nodes: Vec<&kentos_commands::LayerNode> = crate::style::scene::shown_layers(nodes)
            .into_iter()
            .filter(|n| n.data.is_some())
            .collect();
        if !data_nodes.is_empty() || crate::data_view::in_use() {
            crate::data_view::service().text_from(s.styles.images.clone());
            crate::data_view::service().want(
                &data_nodes,
                doc.model.settings(),
                doc.model.styles(),
                &self.camera.visible_bounds(),
                self.camera.scale,
                &look,
                &s.styles.library,
                self.settling().is_none(),
                &|layer| doc.model.data_hidden(layer),
                &|layer| doc.model.editing(layer),
            );
        }
        // The renderer met a part whose numbers were on the GPU only, in buffers it no longer had (plan/render.md
        // RM-19): it drew nothing for it, and the styled layers are built again, once for the frame that said so.
        let lost = {
            let status = self.status.lock().unwrap_or_else(PoisonError::into_inner);
            status.stats.parts_lost.then_some(status.stats.frames)
        };
        if let Some(frame) = lost
            && self.lost_seen.get() != frame
        {
            self.lost_seen.set(frame);
            *self.styled.borrow_mut() = Default::default();
            *self.deep_styled.borrow_mut() = Default::default();
        }
        // A filter calling an extension's function worked out again with its answers: its layer is drawn again.
        let cache = self.styled_in_use();
        cache.borrow_mut().filters_at(s.spatial.filters_version());
        cache.borrow_mut().set_only(deep.map(|d| d.bounds));
        cache.borrow_mut().text_from(s.styles.images.clone());
        let scene = cache.borrow_mut().scene_themed(
            self.generation,
            &doc.model,
            nodes,
            self.theme.as_ref(),
            s.spatial.store(),
            self.wall_view,
            &s.styles.library,
            &look,
            &self.camera.visible_bounds(),
            // A renderer built for the view's scale waits for the wheel to stop (docs/adr/0213 §3).
            self.camera.scale,
            self.settling().is_none(),
            // Over the grid and the underlay models' plan, under the highlights.
            2,
        );
        // The masks cutting with a layer's placed labels (docs/adr/0226 §3).
        let asks = cache.borrow().label_masks().to_vec();
        let scene = with_mask_labels(scene, &asks, s.labels, &doc.model, look.origin);
        // The pictures the style engine made (heat maps, docs/adr/0213 §2.6): handed to the atlas by
        // key, the ones no longer drawn let go.
        let mut made = std::collections::HashSet::new();
        for part in &scene.layers {
            s.styles.images.put_made(part.pictures());
            made.extend(part.pictures().iter().map(|p| p.key.as_str()));
        }
        s.styles.images.keep_made(&made);
        // The pictures the scene draws, decoded once each (docs/adr/0192 §3); its rasters'
        // files for the raster service (docs/adr/0204 §5).
        let folder = doc.path.as_deref().and_then(std::path::Path::parent);
        let mut rasters = std::collections::HashSet::new();
        // Its point clouds' files for the point cloud service (docs/adr/0207 §6).
        let mut clouds = std::collections::HashSet::new();
        for part in &scene.layers {
            for b in part.batches() {
                use kentos_native_style::batches::{BatchKind, FillPaintBatch};
                match &b.kind {
                    BatchKind::Fill {
                        paint: FillPaintBatch::Image { image, .. },
                    } if !s.styles.images.has_bitmap(image) => {
                        let bitmap = crate::pictures::fetch(image, &doc.model, folder);
                        s.styles.images.put_bitmap(image.clone(), bitmap);
                    }
                    BatchKind::Fill {
                        paint: FillPaintBatch::Raster { raster, .. },
                    } if rasters.insert(raster.clone()) => {
                        crate::rasters::tiles::service().register(raster, || {
                            crate::rasters::origin_of(raster, &doc.model, folder)
                        });
                    }
                    BatchKind::Fill {
                        paint: FillPaintBatch::PointCloud { cloud, .. },
                    } => {
                        clouds.insert(cloud.clone());
                    }
                    _ => {}
                }
            }
        }
        if crate::rasters::tiles::in_use() {
            crate::rasters::tiles::service().keep_only(&rasters);
        }
        // The map services the scene shows (docs/adr/0208 §3): known to the hub
        // with what they rest on; vector tiles built with the scene's colours.
        let shown = crate::style::scene::shown_service_layers(nodes);
        if !shown.is_empty() || crate::services::in_use() {
            let settings = doc.model.settings();
            let project =
                std::sync::Arc::new(crate::services::systems::ProjectSystem::of_map(settings));
            let hub = crate::services::hub();
            let mut keys = std::collections::HashSet::new();
            for service in shown {
                let connection = service
                    .connection
                    .as_ref()
                    .and_then(|c| settings.connections.iter().find(|k| &k.id == c));
                let key = crate::services::key_of(service, connection);
                hub.register(&key, service, connection, &project);
                keys.insert(key);
            }
            hub.keep_only(&keys);
            hub.set_look(crate::services::decode::VectorLook {
                origin: kentos_geometry_core::Vec2 {
                    x: look.origin.x,
                    y: look.origin.y,
                },
                palette: look.palette.clone(),
                view: look.view_colors,
            });
        }
        if !clouds.is_empty() || crate::pointclouds::service::in_use() {
            crate::pointclouds::register_scene(&clouds, &doc.model, folder);
        }
        scene
    }

    /// The scale paper-mm symbols are built at: the project's plot scale, or
    /// with symbols of a fixed size on the screen the view's own in quarter
    /// octaves. While the wheel turns the layers keep the scale they were
    /// built at; they are built again when it stops (the web's 150 ms).
    pub(super) fn symbol_scale(&self, doc: &Document, screen: bool) -> f64 {
        let now = kentos_native_style::symbol_scale_of(
            screen,
            doc.model.settings().plot_scale,
            self.camera.scale,
        );
        if !screen || self.settling().is_none() {
            return now;
        }
        match self.styled_in_use().try_borrow().ok().and_then(|c| c.built_scale()) {
            Some((built, true)) => built,
            _ => now,
        }
    }

    /// The deep view (plan/render.md RM-24, docs/adr/9220): from [`DEEP_PX_PER_M`] in, the view and three times its
    /// size round it, its curves round to a quarter pixel there; kept while the view stays in the box and its scale
    /// within twice the one the tolerance was made at; none again below half the threshold, so a view at the threshold
    /// does not switch back and forth.
    /// Metres a world unit is (1 when the camera's is not a positive number).
    fn metres_a_unit(&self) -> f64 {
        let unit = self.camera.metres_per_unit;
        if unit.is_finite() && unit > 0.0 { unit } else { 1.0 }
    }

    fn deep_box(&self) -> Option<kentos_native_style::Deep> {
        let px_per_m = self.camera.scale / self.metres_a_unit();
        let was = self.deep.get();
        let deep = px_per_m >= if was.is_some() { DEEP_PX_PER_M / 2.0 } else { DEEP_PX_PER_M };
        let view = self.camera.visible_bounds();
        let tolerance = DEEP_TOLERANCE_PX / self.camera.scale;
        let next = deep.then(|| match was {
            Some(d) if !clip_stale(&view, &d.bounds) && (0.5..=2.0).contains(&(d.tolerance / tolerance)) => d,
            _ => kentos_native_style::Deep {
                bounds: construction_clip(&view),
                tolerance,
            },
        });
        self.deep.set(next);
        next
    }

    /// The styled scenes built anew at the next frame, as a drawing opened afresh builds them: the visual conformance
    /// check's own (plan/render.md RM-16), to tell a kept part that missed an edit from a drawing that draws differently.
    #[cfg(any(test, feature = "testing"))]
    pub fn forget_styled(&self) {
        *self.styled.borrow_mut() = Default::default();
        *self.deep_styled.borrow_mut() = Default::default();
    }

    /// The styled layers the view draws: the deep view's scene while there is one, else the drawing's.
    fn styled_in_use(&self) -> &RefCell<crate::style::scene::StyledCache> {
        if self.deep.get().is_some() {
            &self.deep_styled
        } else {
            &self.styled
        }
    }

    /// The layers that left dots, markers or waves out over a limit since last asked, each once a kind
    /// (docs/adr/0213 §2.4, plan/render.md RM-11): name, what and how many.
    pub fn take_dropped(&self) -> Vec<(String, crate::style::scene::LeftOut, u64)> {
        self.styled_in_use()
            .try_borrow_mut()
            .map(|mut c| c.take_dropped())
            .unwrap_or_default()
    }

    /// A test's zoom without the scaled picture (docs/adr/1068 §4's control): every zoom frame drawn whole.
    #[cfg(test)]
    pub(crate) fn set_zoom_scaled(on: bool) {
        ZOOM_SCALED.with(|z| z.set(on));
    }

    /// A test's pan held on (until `false`): the view counts as moving as a pan makes it (docs/adr/1067).
    #[cfg(test)]
    pub(crate) fn hold_moving(&mut self, on: bool) {
        self.moved_at = on.then(|| Instant::now() + std::time::Duration::from_secs(3600));
    }

    /// The live camera for the area this view builds (live.rs): a change the app made to its own camera since the last
    /// view is pushed to it first, and ends a gesture the area was running (the app's change wins).
    pub(super) fn live_for_view(&self) -> super::live::LiveView {
        if self.camera != self.live_sent.get() {
            self.live.set(self.camera);
            self.live_sent.set(self.camera);
        }
        self.live.clone()
    }

    /// A gesture the area is running or has not yet told, taken by the app before it reads the pointer (live.rs):
    /// the camera it started at, the one the area shows, and where a pan's drag left the pointer.
    pub fn take_live(&mut self) -> Option<(Camera, Camera, Option<Point>)> {
        self.live.take()
    }

    /// Whether the view is moving (a pan or the wheel within the last [`SYMBOLS_SETTLE`]): the labels keep their
    /// placing meanwhile (docs/adr/0597).
    pub fn moving(&self) -> bool {
        !self.fresh && self.moved_until().is_some()
    }

    /// When the view's move settles and the labels are placed again, while it has not.
    pub(super) fn moved_until(&self) -> Option<Instant> {
        self.moved_at
            .map(|t| t + SYMBOLS_SETTLE)
            .filter(|t| *t > Instant::now())
    }

    /// When the wheel's zoom settles and screen-sized symbols are built again, while it has not.
    pub(super) fn settling(&self) -> Option<Instant> {
        self.zoomed_at
            .map(|t| t + SYMBOLS_SETTLE)
            .filter(|t| *t > Instant::now())
    }

    /// What the last styled build cost and how many layers it built (docs/adr/0090).
    #[cfg(test)]
    pub fn styled_build(&self) -> Option<(Duration, usize)> {
        self.styled.try_borrow().ok().and_then(|c| c.last_build)
    }

    /// The last styled build, taken, so the next one is seen alone; and the layer parts built now (docs/adr/1062's
    /// edit scope).
    #[cfg(test)]
    pub fn take_styled_build(&self) -> (Option<(Duration, usize)>, usize) {
        self.styled
            .try_borrow_mut()
            .map(|mut c| (c.last_build.take(), c.part_count()))
            .unwrap_or_default()
    }

    /// The plain scene of `doc`, from the cache when nothing it depends on changed.
    #[cfg(test)]
    pub(super) fn scene(
        &self,
        doc: &Document,
        canvas: impl Into<Canvas>,
        palette: &Palette,
        settings: &RenderSettings,
    ) -> (Vec2, Arc<ScenePart>, Arc<ScenePart>, Arc<ScenePart>, Bounds) {
        self.scene_with(doc, canvas, palette, settings, true)
    }

    /// The scene of `doc`, from the cache when nothing it depends on changed.
    /// With `plain` off the drawing's layers are styled (docs/adr/0090): the
    /// parts then only carry the origin and the curves' tolerance the
    /// highlights are built with.
    pub(super) fn scene_with(
        &self,
        doc: &Document,
        canvas: impl Into<Canvas>,
        palette: &Palette,
        settings: &RenderSettings,
        plain: bool,
    ) -> (Vec2, Arc<ScenePart>, Arc<ScenePart>, Arc<ScenePart>, Bounds) {
        let canvas = canvas.into();
        let view = self.camera.visible_bounds();
        let needed = lod::band(self.camera.scale, settings.curve_tolerance_px);
        let band = lod::build_band(self.camera.scale, settings.curve_tolerance_px);
        let budget = settings.curve_segment_budget;
        let changes = changes(doc);
        let mut cache = self.scene.borrow_mut();
        let current = cache.as_ref().is_some_and(|c| {
            c.generation == self.generation
                && c.changes == changes
                && c.canvas == canvas
                && c.plain == plain
        });
        let empty = |origin: Vec2, tolerance: f64| {
            Arc::new(ScenePart {
                origin,
                tolerance,
                ..ScenePart::default()
            })
        };
        if !current {
            let origin = self.draw_origin(doc);
            let clip = construction_clip(&view);
            *cache = Some(if plain {
                Cached {
                    generation: self.generation,
                    changes,
                    canvas,
                    plain,
                    origin,
                    fixed: Arc::new(scene::build_fixed(doc, palette, origin)),
                    curves: Arc::new(scene::build_curves(
                        doc,
                        palette,
                        origin,
                        lod::tolerance(band),
                        budget,
                    )),
                    band,
                    construction: Arc::new(scene::build_construction(doc, palette, origin, &clip)),
                    clip,
                }
            } else {
                Cached {
                    generation: self.generation,
                    changes,
                    canvas,
                    plain,
                    origin,
                    fixed: empty(origin, 0.0),
                    curves: empty(origin, lod::tolerance(band)),
                    band,
                    construction: empty(origin, 0.0),
                    clip,
                }
            });
        } else if let Some(cached) = cache.as_mut() {
            if lod::stale(cached.band, needed) {
                cached.curves = if plain {
                    Arc::new(scene::build_curves(
                        doc,
                        palette,
                        cached.origin,
                        lod::tolerance(band),
                        budget,
                    ))
                } else {
                    empty(cached.origin, lod::tolerance(band))
                };
                cached.band = band;
            }
            if clip_stale(&view, &cached.clip) {
                cached.clip = construction_clip(&view);
                if plain {
                    cached.construction = Arc::new(scene::build_construction(
                        doc,
                        palette,
                        cached.origin,
                        &cached.clip,
                    ));
                }
            }
        }
        match cache.as_ref() {
            Some(c) => (
                c.origin,
                c.fixed.clone(),
                c.curves.clone(),
                c.construction.clone(),
                c.clip,
            ),
            None => (
                Vec2::default(),
                Arc::default(),
                Arc::default(),
                Arc::default(),
                construction_clip(&view),
            ),
        }
    }

    /// Shows `b` as large as it fits, with the margin a fit keeps; before the
    /// area has its size, once it has it. (A new project made on the start
    /// screen, its sheet fitted to the camera's 1 × 1 pixel, opened at about
    /// 1:500 000; docs/adr/0165 §3.)
    pub fn show(&mut self, b: &Bounds) {
        if self.sized {
            // The pointer stays where it is on the area, as with any view change.
            let at = self.cursor.map(|c| self.camera.world_to_screen(c));
            self.camera.fit(b, FIT_PADDING);
            self.cursor = at.map(|[x, y]| self.camera.screen_to_world(x, y));
        } else {
            self.fit_pending = true;
            self.opened_extent = Some(*b);
        }
    }

    /// A view change a tool asked for (Kaydır, Pencere yakınlaştır;
    /// docs/adr/0056). The pointer stays where it is on the area, so the
    /// world point under it follows the view.
    pub fn change(&mut self, change: ViewChange) {
        let at = self.cursor.map(|c| self.camera.world_to_screen(c));
        match change {
            ViewChange::Pan { dx, dy } => self.camera.pan_by(dx, dy),
            ViewChange::Fit { bounds, padding } => self.camera.fit(&bounds, padding),
            // The app opens the text field (text_field.rs), its window again
            // with the picked point (calc/), Blok oluştur's (blocks.rs), the
            // clipboard's copy (clipboard.rs), Blok ekle's values (attribute_values.rs), a text file's dialog
            // (text_file.rs) or Bilgi's card (info_card.rs); the camera stays.
            ViewChange::Text(_)
            | ViewChange::Paragraph(_)
            | ViewChange::Picked(_)
            | ViewChange::PickedObjects(_)
            | ViewChange::Captured(_)
            | ViewChange::DefineBlock(_)
            | ViewChange::Copied(_)
            | ViewChange::AttributeValues(_)
            | ViewChange::OpenTextFile
            | ViewChange::OpenImageFile
            | ViewChange::RasterValues(_)
            | ViewChange::CloudQuery { .. }
            | ViewChange::DataInfo { .. }
            | ViewChange::DataSelect(_)
            | ViewChange::Network(_)
            | ViewChange::Command(_)
            | ViewChange::InfoCard(_)
            | ViewChange::InfoHits(..)
            | ViewChange::AddressAt(_)
            | ViewChange::LayerLeftOut
            | ViewChange::PlaceTable(..) => {}
            // Görünümü döndür: the view turns about its middle (docs/adr/0285 §2).
            ViewChange::Twist(t) => self.camera.twist = kentos_render_wgpu::normal_twist(t),
        }
        self.cursor = at.map(|[x, y]| self.camera.screen_to_world(x, y));
    }

    /// A camera kept with a drawing set aside (a tab, Blok düzenleyici, a sheet's model view) put back: its middle,
    /// scale and twist, the size the area has now (UR2-055, UR2-057: the size it was kept with, before the tab strip
    /// came or a split view went, put the picture and the pointer apart by the difference).
    pub fn put_camera(&mut self, camera: Camera) {
        self.camera = camera;
        if self.bounds.width > 0.0 && self.bounds.height > 0.0 {
            self.camera.set_size(f64::from(self.bounds.width), f64::from(self.bounds.height));
        }
    }

    /// Where the camera stands, for the view history (docs/adr/0141).
    pub fn viewpoint(&self) -> Viewpoint {
        Viewpoint {
            // The world's point, whatever system the map is in (docs/adr/0583 §3).
            center: self.camera.world_center(),
            scale: self.camera.scale,
            twist: self.camera.twist,
            ceiling: None,
        }
    }

    /// The camera back at a recorded view. The pointer stays where it is on
    /// the area, as with any view change, so the world point under it follows.
    pub fn restore(&mut self, view: Viewpoint) {
        let at = self.cursor.map(|c| self.camera.world_to_screen(c));
        self.camera.center_on(view.center);
        if view.scale.is_finite() && view.scale > 0.0 {
            self.camera.scale = self.camera.clamp_scale(view.scale);
        }
        self.camera.twist = kentos_render_wgpu::normal_twist(view.twist);
        self.cursor = at.map(|[x, y]| self.camera.screen_to_world(x, y));
    }

    /// A named view (docs/adr/0285 §1): its middle in the middle of the area, its width and
    /// height as large as they fit, turned by its twist; the pointer stays where it is on the area.
    pub fn show_view(&mut self, view: &kentos_commands::NamedView) {
        let at = self.cursor.map(|c| self.camera.world_to_screen(c));
        self.camera.twist = kentos_render_wgpu::normal_twist(view.twist);
        self.camera.fit_size(
            Vec2::new(view.center.x, view.center.y),
            view.width,
            view.height,
            0.0,
        );
        self.cursor = at.map(|[x, y]| self.camera.screen_to_world(x, y));
    }

    /// The view as a named view keeps it: its middle, the world width and height it shows and its twist.
    pub fn named(&self, name: &str) -> kentos_commands::NamedView {
        let (width, height) = self.camera.view_size();
        kentos_commands::NamedView {
            id: String::new(),
            name: name.to_owned(),
            center: {
                let c = self.camera.world_center();
                kentos_commands::Vec2 { x: c.x, y: c.y }
            },
            width,
            height,
            twist: self.camera.twist,
            recipe: None,
        }
    }

    /// The document's start view, else its extents; its origin when it has neither.
    pub(super) fn fit(&mut self, doc: &Document) {
        self.fit_pending = false;
        match start_view(doc)
            .or(self.opened_extent)
            .or_else(|| scene::extents(doc))
        {
            Some(b) => self.camera.fit(&b, FIT_PADDING),
            None => self.camera.center_on(scene::scene_origin(doc)),
        }
    }

    /// The window's device pixels per logical pixel as the area was last
    /// drawn (1 before the first frame).
    pub fn scale_factor(&self) -> f64 {
        let s = self
            .status
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .scale_factor;
        if s > 0.0 { f64::from(s) } else { 1.0 }
    }

    /// Büyüteç's window (docs/adr/0181 §5): this area's scene through
    /// `camera`, drawn by the same renderer from the same GPU parts and
    /// styled layers, on the drawing's ground.
    pub fn lens<'a>(&self, camera: Camera, canvas: impl Into<Canvas>) -> Element<'a, Message> {
        let Some(origin) = self.scene.borrow().as_ref().map(|c| c.origin) else {
            return container("").into();
        };
        let settings = RenderSettings {
            samples: 1,
            ..RenderSettings::new(palette(canvas.into()).background)
        };
        shader(Lens {
            main: self.id,
            origin,
            camera,
            settings,
        })
        .width(Fill)
        .height(Fill)
        .into()
    }

    /// The pointer's world point again, through the app's camera now, where the pointer is on the screen; none off the
    /// area. Yakınlaştır and Uzaklaştır zoom about the middle (`App::zoom`): the drawing moves under a still pointer
    /// (docs/adr/9251).
    pub fn repoint_cursor(&mut self) {
        self.cursor = self
            .pointer_px
            .map(|at| self.camera.screen_to_world(f64::from(at.x), f64::from(at.y)));
    }

    /// The world point at a position of the area, through the float64 camera.
    pub fn world(&self, at: Point) -> Vec2 {
        self.shown()
            .screen_to_world(f64::from(at.x), f64::from(at.y))
    }

    /// The camera the area shows: a running gesture's live one (live.rs), else the app's. The pointer's world point
    /// and a snap's reach are read through it, so a move while the wheel turns lands where the screen shows it.
    pub fn shown(&self) -> Camera {
        if self.live.moving() { self.live.camera() } else { self.camera }
    }
}

/// Metres a drawing unit is for the view's zoom limits: in a project drawn in degrees (GIS-040) a degree along its
/// ellipsoid's equator, a·π/180 (WGS 84: 111 319.49 m), so its deepest zoom is about the 0.1 mm a pixel a project of
/// metres has; else 1. A limit, not a measure: the status bar's scale takes the degree's length where the view is
/// (map_marks.rs).
pub(crate) fn unit_metres(settings: &kentos_commands::ProjectSettings) -> f64 {
    match kentos_interaction::ground::geographic_system(settings) {
        Some(system) => system.datum().ellipsoid().0 * std::f64::consts::PI / 180.0,
        None => 1.0,
    }
}
