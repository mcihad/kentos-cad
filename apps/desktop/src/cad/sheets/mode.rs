//! The sheet mode in the shell: its window and body, the project as it sees it, the book following the
//! project and the drawing's data, its messages and effects, the PDF export and what it writes differently.

use super::*;

impl App {
    /// The window while a sheet is in front: the ribbon with its Pafta tab,
    /// the sheet mode where the drawing and its docks were, the tabs under it,
    /// its cells in the status bar, its windows over everything.
    pub(crate) fn sheet_view(&self) -> iced::Element<'_, Message> {
        use iced::widget::{column, container};
        let painter: Painter<'_> = Rc::new(self.sheet_painter());
        let base = container(column![
            self.ribbon(false),
            self.sheet_body(painter.clone()),
            if self.sheets.model_scope().is_some() {
                self.command_line()
            } else {
                iced::widget::space().height(0).into()
            },
            self.sheets.tabs().map(Message::Sheet),
            self.status_bar(),
        ])
        .width(iced::Fill)
        .height(iced::Fill)
        .style(kentos_ui::style::container::window);
        let mut layers: Vec<iced::Element<'_, Message>> = vec![base.into()];
        layers.extend(self.app_menu_view());
        layers.extend(self.sheets.window(painter).map(|w| w.map(Message::Sheet)));
        if let Some(dialog) = self.dialog {
            layers.push(self.dialog_view(dialog));
        }
        if layers.len() == 1 {
            return layers.remove(0);
        }
        iced::widget::Stack::with_children(layers).into()
    }

    pub(super) fn sheet_body<'a>(&'a self, painter: Painter<'a>) -> iced::Element<'a, Message> {
        use iced::widget::{button, column, container, pin, row, scrollable, stack};
        use kentos_ui::{label, style};
        if self.sheets.model_scope().is_none() {
            return self.sheets.body(painter).map(Message::Sheet);
        }
        let ruler = kentos_ui::widget::rulers::thickness();
        let left = kentos_ui::widget::rulers::left_thickness();
        let stage = stack![self.sheets.stage(painter).map(Message::Sheet)]
            .extend(
                self.text_field_view()
                    .map(|e| pin(e).x(left).y(ruler).into()),
            )
            .extend(
                self.paragraph_view()
                    .map(|e| pin(e).x(left).y(ruler).into()),
            );
        let stage = kentos_ui::widget::ContextMenu::controlled(
            stage,
            self.drawing_menu
                .map(|o| Point::new(o.at.x + ruler, o.at.y + ruler)),
            move |_| self.drawing_menu_items(),
            Message::DrawingMenu(crate::drawing_menus::Event::Closed),
        );
        let side = column![
            container(
                column![
                    label::body("Model düzenleme"),
                    label::caption(
                        "Çizim araçları modelde; tekerlek ve orta tuş bu görünümde çalışır."
                    ),
                    button(label::body("Kâğıda dön"))
                        .padding(kentos_ui::theme::metrics::action())
                        .on_press(Message::Sheet(kentos_sheet_ui::Message::ReturnPaper))
                        .style(style::button::primary)
                ]
                .spacing(8)
            )
            .padding(12),
            scrollable(self.panel_body(crate::app::Panel::Properties)).height(iced::Fill)
        ]
        .width(kentos_sheet_ui::inspector_view::WIDTH)
        .height(iced::Fill);
        row![
            container(self.sheets.left_panel().map(Message::Sheet))
                .height(iced::Fill)
                .style(style::container::surface),
            container(stage).width(iced::Fill).height(iced::Fill),
            container(side)
                .height(iced::Fill)
                .style(style::container::surface)
        ]
        .height(iced::Fill)
        .into()
    }

    /// The maps' painter of the open drawing (its scene and its text).
    pub(crate) fn sheet_painter(&self) -> SheetPainter<'_> {
        let mut painter = painter_of(
            &self.sheet_maps,
            self.document.as_ref(),
            &self.spatial,
            &self.styles.library,
            &self.styles.images,
        );
        painter.interaction = Some(self);
        painter.book = Some(self.sheets.book());
        painter.sheet = self.sheets.open_sheet().map(|s| s.id.as_str());
        painter
    }

    /// The project as the sheet mode sees it: its mode and type, whether it
    /// has a coordinate system and attributes, its plot scale, its name, the
    /// user and today, and where the drawing area looks.
    pub(crate) fn sheet_context(&self) -> Context {
        let doc = self.document.as_ref();
        let settings = doc.map(Document::settings);
        let srid = settings.map_or(0, |s| s.srid);
        // The registry's system or the project's own definition (docs/adr/0168 §1); the
        // latter without the registry's transverse Mercator values.
        let crs = settings
            .filter(|s| s.has_system())
            .map(crate::crs::project_name);
        let c = self.viewport.camera.world_center();
        Context {
            // The type the interface shows: a project not asked its type is CBS (docs/adr/0165).
            workspace: doc.map(|_| self.work_mode()),
            // The module's type in a module's program (module_surfaces.rs): its templates first, as the profile
            // lists them; kentos-cad's drawing carries none yet.
            project_type: self.sheet_project_type(),
            // A module's program's own catalog types (docs/adr/0802): a new sheet starts with its own template and
            // the gallery shows its types' templates and those made for no type; kentos-cad's, none (every type).
            project_types: self
                .program
                .map(crate::program::project_types_of)
                .unwrap_or_default(),
            exact_plot_scale: settings
                .map(|s| s.plot_scale)
                .filter(|s| s.is_finite() && (1e-6..=1e8).contains(s)),
            capabilities: Capabilities {
                georeferenced: crs.is_some(),
                attribute_layers: self.sheet_attributes.get(),
                plot_scale: settings
                    .map(|s| s.plot_scale.round())
                    .filter(|s| *s >= 1.0)
                    .map(|s| s as u32),
            },
            project: ProjectInfo {
                name: doc.map_or_else(String::new, |d| d.name().to_owned()),
                user: std::env::var("USER")
                    .or_else(|_| std::env::var("USERNAME"))
                    .unwrap_or_default(),
                date: kentos_sheet_ui::text::today(),
                crs_name: crs.clone().unwrap_or_default(),
                // The drawing's own variables, read after the book's (docs/adr/0214 §2.3), and what the drawing says
                // of itself for a title block (docs/adr/8037), read once a change of the drawing.
                variables: doc.map_or_else(Vec::new, |d| {
                    let key = Some((d.session, d.model.generation()));
                    let mut cached = self.sheet_drawing_variables.borrow_mut();
                    if cached.0 != key {
                        *cached = (key, kentos_plot::inputs::drawing_variables(&d.model));
                    }
                    cached.1.clone()
                }),
                // The coordinate facts (docs/adr/1680) and the survey quality (docs/adr/2400), as the printed sheet
                // has them.
                ..settings.map_or_else(ProjectInfo::default, |s| {
                    let f = kentos_project::facts::coordinate_facts(s);
                    let q = kentos_project::survey_facts::survey_facts(s);
                    ProjectInfo {
                        datum: f.datum,
                        ellipsoid: f.ellipsoid,
                        projection: f.projection,
                        heights: f.heights,
                        second_crs: f.second,
                        epsg: f.epsg,
                        epoch: f.epoch,
                        position_accuracy: q.position,
                        height_accuracy: q.height,
                        network_adjustment: q.network,
                        survey_quality: q.note,
                        ..ProjectInfo::default()
                    }
                })
            },
            // With its transverse Mercator parameters: the core works out the meridian convergence.
            crs: crs.map(|name| {
                kentos_plot::inputs::crs_info(srid)
                    .filter(|_| srid != crate::crs::LOCAL_SRID)
                    .unwrap_or(CrsInfo { name, tm: None })
            }),
            center: doc.map(|_| GroundPoint { x: c.x, y: c.y }),
            annotation_scales: self.sheet_maps.annotation_scales.borrow().clone(),
            named_views: settings.map_or_else(Vec::new, |s| s.drawn_views()),
            drawing_bounds: self.sheet_extent.get(),
            level_bounds: self.sheet_level_extent.get(),
            selection_bounds: doc.and_then(|d| {
                let ids: Vec<_> = self
                    .selection
                    .ids()
                    .iter()
                    .filter_map(|s| d.model.uid(*s))
                    .map(|u| u.to_string())
                    .collect();
                if ids.is_empty() {
                    None
                } else {
                    kentos_native_application::sheets_fit::object_extent(&d.model, Some(&ids))
                        .ok()
                        .flatten()
                }
            }),
            // The layers a coordinate list may read (docs/adr/0206 §2).
            // By their paths: an external reference's Duvar (`Dış başvuru: Plan / Mimari / Duvar`) is told from the
            // drawing's own when a map's own override names it (docs/adr/1110 §4).
            layers: doc.map_or_else(Vec::new, |d| {
                d.model
                    .layers()
                    .leaves()
                    .into_iter()
                    .map(|l| (l.id.clone(), crate::properties::layer_path(d.model.layers(), &l.id)))
                    .collect()
            }),
            // How much ground the drawing area shows (Görünüme sığdır, docs/adr/0206 §1).
            view_size: doc
                .map(|_| &self.viewport.camera)
                .filter(|cam| cam.scale > 0.0 && cam.width > 0.0 && cam.height > 0.0)
                .map(|cam| [cam.width / cam.scale, cam.height / cam.scale]),
            // A map's own layers (docs/adr/0314 §1): the project's layer states and patterns by name.
            layer_states: settings.map_or_else(Vec::new, |s| {
                s.layer_states.iter().map(|l| l.name.clone()).collect()
            }),
            line_types: settings.map_or_else(Vec::new, |s| {
                s.line_types.iter().map(|t| t.name.clone()).collect()
            }),
            // A map's level (docs/adr/0716): the project's levels.
            levels: settings.map_or_else(Vec::new, |s| {
                s.arch.as_ref().map_or_else(Vec::new, |a| {
                    a.levels
                        .iter()
                        .map(|l| (l.id.clone(), l.name.clone()))
                        .collect()
                })
            }),
        }
    }

    /// The key the open project's book is kept under (the web's `projectKeyOf`).
    pub(crate) fn sheet_key(&self) -> kentos_sheet_ui::ProjectKey {
        let doc = self.document.as_ref();
        let cloud =
            doc.and_then(|d| d.cloud_source())
                .map(|c| kentos_sheet_ui::store::CloudProject {
                    tenant: c.tenant.to_string(),
                    project: c.project.to_string(),
                    name: c.info.name.clone(),
                });
        let project_id = doc
            .and_then(|d| d.model.project_id())
            .map(|id| id.to_string());
        let file = doc
            .and_then(|d| d.path.as_ref())
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned());
        let session = doc.map_or(0, |d| d.session);
        kentos_sheet_ui::project_key(
            cloud.as_ref(),
            project_id.as_deref(),
            file.as_deref(),
            &format!("{}-{session}", std::process::id()),
        )
    }

    /// After every message: the book follows the project (the drawing's own,
    /// docs/adr/0251; sheet_book.rs), the mode follows the project's values,
    /// the maps follow the drawing.
    pub(crate) fn follow_sheets(&mut self) {
        if let Some(store) = self.sheet_store.clone()
            && !self.sheets.has_store()
        {
            self.sheets.attach(store);
        }
        self.follow_sheet_book();
        self.sync_sheet_model_camera();
        // The gallery over the drawing reads the project too (what a template needs that it lacks).
        if !self.sheets.is_active() && !self.sheets.gallery_open() {
            return;
        }
        // What only a changed drawing changes: its objects' attributes, the maps' scene.
        let generation = self
            .document
            .as_ref()
            .map(|d| (d.session, d.model.generation()));
        if generation != self.sheet_generation {
            self.sheet_generation = generation;
            self.sheet_attributes.set(
                self.document
                    .as_ref()
                    .is_some_and(|d| d.model.entities().any(|e| !e.base().attrs.is_empty())),
            );
            self.refresh_sheet_extents();
            self.sheet_maps.sync(self.document.as_ref());
        }
        let ctx = self.sheet_context();
        // The drawing area's centre changes as it is looked around; it matters only to a new map.
        let same = {
            let mut a = ctx.clone();
            let mut b = self.sheets.context().clone();
            (a.center, a.view_size) = (None, None);
            (b.center, b.view_size) = (None, None);
            a == b
        };
        if !same {
            self.sheets.set_context(ctx);
            self.sync_sheet_model_camera();
        }
        self.follow_sheet_data();
    }

    /// The sheet's tables, coordinate lists and legends follow the drawing,
    /// the chosen objects and the sheet's own items (what they name, where its
    /// maps look): worked out again only when one of these changed.
    pub(super) fn follow_sheet_data(&mut self) {
        let (Some(doc), Some(sheet)) = (self.document.as_ref(), self.sheets.open_sheet()) else {
            return;
        };
        let book = self.sheets.book();
        let mut h = DefaultHasher::new();
        (doc.session, doc.model.generation()).hash(&mut h);
        self.selection
            .ids()
            .iter()
            .map(|s| s.0)
            .collect::<Vec<_>>()
            .hash(&mut h);
        // The items that read the drawing, as the sheet and its master page have them.
        let master = sheet
            .master
            .as_deref()
            .and_then(|m| book.masters.iter().find(|x| x.id == m));
        for it in master
            .into_iter()
            .flat_map(|m| m.items.iter())
            .chain(sheet.items.iter())
        {
            if matches!(
                it.kind,
                kentos_sheet::kinds::ItemKind::Table(_)
                    | kentos_sheet::kinds::ItemKind::CoordinateList(_)
                    | kentos_sheet::kinds::ItemKind::Legend(_)
                    | kentos_sheet::kinds::ItemKind::Map(_)
            ) {
                serde_json::to_string(&it.kind)
                    .unwrap_or_default()
                    .hash(&mut h);
                it.id.hash(&mut h);
            }
        }
        for (id, e) in kentos_plot::inputs::extents_of(self.sheets.display_list()) {
            (id, e.map(f64::to_bits)).hash(&mut h);
        }
        let key = h.finish();
        if self.sheet_data_key == Some(key) {
            return;
        }
        self.sheet_data_key = Some(key);
        let unscoped = (self.spatial.draws_otherwise() || doc.model.objects_isolated())
            .then(|| {
                let mut spatial = Spatial::new();
                spatial.ignore_object_isolation();
                spatial.reload(&doc.model);
                spatial
            });
        let data = kentos_plot::inputs::sheet_data(
            &doc.model,
            unscoped.as_ref().unwrap_or(&self.spatial),
            &self.styles.library,
            self.selection.ids(),
            book,
            sheet,
            self.sheets.display_list(),
        );
        let inputs = kentos_sheet::display::RenderInputs {
            tables: data.tables,
            coordinates: data.coordinates,
            legends: data.legends,
            // Each map's own survey quality (docs/adr/2800).
            map_surveys: data.surveys,
            ..kentos_sheet_ui::empty_inputs()
        };
        self.sheets.set_data(inputs);
    }

    /// A message of the sheet mode, and what it asks of the desktop.
    /// The drawing's and its levels' boxes, as the paper fits them (docs/adr/0718).
    pub(super) fn refresh_sheet_extents(&self) {
        self.sheet_extent.set(self.document.as_ref().and_then(|d| {
            kentos_native_application::sheets_fit::object_extent(&d.model, None)
                .ok()
                .flatten()
        }));
        self.sheet_level_extent
            .set(self.document.as_ref().and_then(|d| {
                let levels: std::collections::BTreeSet<String> = d
                    .model
                    .settings()
                    .arch
                    .as_ref()
                    .map(|a| a.levels.iter().map(|l| l.id.clone()).collect())
                    .unwrap_or_default();
                kentos_native_application::sheets_levels::level_extent(&d.model, &levels)
            }));
    }

    pub(crate) fn sheet_message(&mut self, m: kentos_sheet_ui::Message) -> Task<Message> {
        // A new map looks where the drawing area does now; a map takes its place and scale from
        // it (docs/adr/0206 §1).
        if matches!(
            &m,
            kentos_sheet_ui::Message::Stage(_)
                | kentos_sheet_ui::Message::NewSheet
                | kentos_sheet_ui::Message::Gallery(_)
                | kentos_sheet_ui::Message::MapFitDrawing
                | kentos_sheet_ui::Message::MapFitSelection
                | kentos_sheet_ui::Message::MapFitView
                | kentos_sheet_ui::Message::MapCentreFromView
        ) {
            let ctx = self.sheet_context();
            let now = self.sheets.context();
            if ctx.center != now.center
                || ctx.view_size != now.view_size
                || ctx.drawing_bounds != now.drawing_bounds
                || ctx.selection_bounds != now.selection_bounds
            {
                self.sheets.set_context(ctx);
            }
        }
        // Kat planı paftaları reads the project's levels and their box, from the drawing as well (docs/adr/0718).
        if matches!(&m, kentos_sheet_ui::Message::LevelSheets) {
            self.refresh_sheet_extents();
            let ctx = self.sheet_context();
            if ctx != *self.sheets.context() {
                self.sheets.set_context(ctx);
            }
        }
        // A coordinate list takes the drawing's choice by its objects' lasting ids, and shows its
        // objects in the drawing (docs/adr/0206 §2).
        if matches!(&m, kentos_sheet_ui::Message::CoordTakeSelection) {
            let uids: Vec<String> = self.document.as_ref().map_or_else(Vec::new, |d| {
                self.selection
                    .ids()
                    .iter()
                    .filter_map(|s| d.model.uid(*s))
                    .map(|u| u.to_string())
                    .collect()
            });
            if uids.is_empty() {
                self.warn("Çizimde seçili nesne yok: önce modelde nesneleri seçin, sonra “Seçimi al”a basın.");
                return Task::none();
            }
            return self.sheet_message(kentos_sheet_ui::Message::CoordObjects(uids));
        }
        if matches!(&m, kentos_sheet_ui::Message::CoordShow) {
            let (Some(doc), Some(source)) = (
                self.document.as_ref(),
                self.sheets.chosen_coordinate_source(),
            ) else {
                return Task::none();
            };
            let (slots, _) =
                kentos_plot::inputs::coordinate_objects(&doc.model, self.selection.ids(), &source);
            if slots.is_empty() {
                return Task::none();
            }
            self.selection.set(slots);
            self.sheet_tab = false;
            return self.update(Message::Run("view.zoomSelection"));
        }
        // The gallery's pictures show the drawing as it is now.
        if matches!(&m, kentos_sheet_ui::Message::Gallery(_)) {
            self.sheet_maps.sync(self.document.as_ref());
        }
        let effects = self.sheets.update(m);
        if self.sheets.model_scope().is_none()
            && let Some(camera) = self.viewport.sheet_saved_camera.take()
        {
            self.cancel();
            self.viewport.camera = camera;
        }
        self.sheet_effects(effects)
    }

    pub(crate) fn sync_sheet_model_index(&mut self) {
        if let Some(doc) = &self.document {
            self.spatial
                .set_sheet_view(&doc.model, self.sheets.active_viewport());
            self.spatial.sync(&doc.model);
        }
    }

    pub(crate) fn sync_sheet_model_camera(&mut self) {
        if self.sheets.model_scope().is_some() && self.sheets.active_viewport().is_none() {
            let _ = self.sheets.update(kentos_sheet_ui::Message::ReturnPaper);
            self.sheet_tab = true;
            self.output("Görünüm kaynağı artık çözülemiyor; model düzenlemesi kapatıldı. Kaynak bağını onarın.");
        }
        self.sync_sheet_model_index();
        if let Some(v) = self.sheets.model_camera() {
            self.viewport
                .sheet_saved_camera
                .get_or_insert(self.viewport.camera);
            self.viewport.camera = Camera {
                center: Vec2::new(v.center[0], v.center[1]),
                scale: v.resolution,
                width: v.size[0],
                height: v.size[1],
                twist: f64::from(v.rotation).to_radians() / 1000.,
                ..Default::default()
            };
        } else if let Some(camera) = self.viewport.sheet_saved_camera.take() {
            self.cancel();
            self.viewport.camera = camera;
        }
    }

    /// What the sheet mode asks of the desktop.
    pub(crate) fn sheet_effects(&mut self, effects: Vec<Effect>) -> Task<Message> {
        let mut tasks = Vec::new();
        for effect in effects {
            let mut task = Task::none();
            match effect {
                Effect::ModelScope(active) => {
                    self.cancel();
                    self.sheet_tab = !active;
                    self.sync_sheet_model_camera();
                }
                Effect::ModelPointer(p) => {
                    use kentos_sheet_ui::model_scope::PointerAction;
                    self.sync_sheet_model_camera();
                    let screen = self
                        .viewport
                        .camera
                        .world_to_screen(Vec2::new(p.world[0], p.world[1]));
                    let at = Point::new(screen[0] as f32, screen[1] as f32);
                    let event = match p.action {
                        PointerAction::Move => crate::viewport::Event::Moved(at),
                        PointerAction::Press => crate::viewport::Event::Pressed(at),
                        PointerAction::Release => crate::viewport::Event::Released(at),
                        PointerAction::Leave => crate::viewport::Event::Left,
                        PointerAction::RightClick => crate::viewport::Event::RightClick(at),
                    };
                    task = self.pointer(event);
                    self.sheet_maps.sync(self.document.as_ref());
                }
                Effect::ShowSheet => {
                    self.sheet_tab = true;
                    self.sheet_maps.sync(self.document.as_ref());
                }
                Effect::ShowModel => {
                    self.sheet_tab = false;
                    if let Some(camera) = self.viewport.sheet_saved_camera.take() {
                        self.cancel();
                        self.viewport.camera = camera;
                    }
                }
                Effect::AskExportPath { kind, suggested } => {
                    let (name, ext) = match kind {
                        ExportKind::Pdf => ("PDF belgesi", "pdf"),
                        ExportKind::Svg => ("SVG resmi", "svg"),
                        ExportKind::Png => ("PNG resmi", "png"),
                        ExportKind::Kpafta => ("KentOS pafta dosyası", "kpafta"),
                    };
                    task = Task::perform(
                        async move {
                            rfd::AsyncFileDialog::new()
                                .set_title("Dışa aktar")
                                .set_file_name(suggested)
                                .add_filter(name, &[ext])
                                .save_file()
                                .await
                                .map(|f| f.path().to_owned())
                        },
                        move |path| Message::SheetExportTo(kind, path),
                    );
                }
                Effect::AskImportPath => {
                    task = Task::perform(
                        async {
                            let file = rfd::AsyncFileDialog::new()
                                .set_title(".kpafta dosyasından")
                                .add_filter("KentOS pafta dosyası", &["kpafta"])
                                .pick_file()
                                .await?;
                            let name = file.file_name();
                            let bytes = file.read().await;
                            Some((name, String::from_utf8_lossy(&bytes).into_owned()))
                        },
                        |picked| match picked {
                            Some((name, text)) => {
                                Message::Sheet(kentos_sheet_ui::Message::ImportText(name, text))
                            }
                            None => Message::Sheet(kentos_sheet_ui::Message::CloseDialog),
                        },
                    );
                }
                // Resim seç…: a PNG or JPEG file, its bytes back to the sheet mode.
                Effect::AskPicturePath => {
                    task = Task::perform(
                        async {
                            let file = rfd::AsyncFileDialog::new()
                                .set_title("Resim seç")
                                .add_filter(
                                    "Resim (PNG, JPEG, SVG)",
                                    &["png", "jpg", "jpeg", "svg"],
                                )
                                .pick_file()
                                .await?;
                            Some((file.file_name(), file.read().await))
                        },
                        |picked| match picked {
                            Some((name, bytes)) => {
                                Message::Sheet(kentos_sheet_ui::Message::PictureFile(name, bytes))
                            }
                            None => Message::Sheet(kentos_sheet_ui::Message::CloseDialog),
                        },
                    );
                }
                // Yazdır: the PDF in the system's viewer (sheet_pdf.rs).
                Effect::Print => task = self.sheet_print(),
                Effect::PrintPreview => self.sheet_print_preview(),
                // Koordinat sistemi seç: the project's settings at its coordinate system.
                Effect::HostAction(action) if action == "project.crs" => {
                    task = Task::done(Message::Run("crs.set"));
                }
                // Atlas: the atlas's window (docs/adr/0534), the preview and the PDF with a page for each object.
                Effect::HostAction(action) if action == "sheet.preflight" => {
                    task = Task::done(Message::Run("file.preflight.sheets"));
                }
                Effect::HostAction(action) if action == "sheet.publish" => {
                    task = Task::done(Message::Run("file.publish.sheets"));
                }
                Effect::HostAction(action) if action == "sheet.atlas" => {
                    task = Task::done(Message::Run("file.export.atlas"));
                }
                Effect::HostAction(action) => self.warn(format!(
                    "Bu düzeltme ({action}) masaüstünde bu sürümde yok."
                )),
                Effect::Notice(text) => self.output(text),
                Effect::Say(say, text) => self.say(
                    match say {
                        Say::Info => Level::Info,
                        Say::Success => Level::Success,
                        Say::Warn => Level::Warn,
                        Say::Error => Level::Error,
                    },
                    text,
                ),
                Effect::Library(request) => task = self.library_request(request),
                Effect::FocusRename => {
                    task = iced::widget::operation::focus(kentos_ui::widget::tree_view::RENAME);
                }
            }
            tasks.push(task);
        }
        Task::batch(tasks)
    }

    /// Writes an export where the user chose (nothing when they did not).
    pub(crate) fn sheet_export_to(&mut self, kind: ExportKind, path: Option<PathBuf>) {
        let Some(path) = path else {
            return;
        };
        // The file takes the project as it is now (its name, the date, its system), whether or
        // not the sheet mode followed the drawing since (an agent may have changed the book).
        self.sheets.set_context(self.sheet_context());
        let made = match kind {
            ExportKind::Pdf => self.sheet_native_pdf().map(|p| (p.bytes, p.findings)),
            ExportKind::Png | ExportKind::Svg => (|| {
                let doc = self.sheet_output_snapshot()?;
                let id = self
                    .sheets
                    .open_sheet()
                    .ok_or("Önce bir pafta açın.")?
                    .id
                    .clone();
                let drawing = self
                    .document
                    .as_ref()
                    .and_then(|d| d.path.as_ref())
                    .map(|p| p.to_string_lossy().into_owned());
                let mut captured = kentos_plot::provenance::Capture::new(
                    &doc,
                    &kentos_plot::book::current_user(),
                    &kentos_plot::book::today(),
                    drawing.as_deref(),
                )
                .map_err(|e| e.message)?;
                captured
                    .dpi_override(Some(self.sheets.output_dpi()))
                    .map_err(|e| e.message)?;
                let image =
                    kentos_plot::preview::page_captured(&captured, &id, self.sheets.output_dpi())
                        .map_err(|e| e.message)?;
                Ok((
                    if kind == ExportKind::Png {
                        image.png
                    } else {
                        image.svg.into_bytes()
                    },
                    Vec::new(),
                ))
            })(),
            ExportKind::Kpafta => self
                .sheets
                .kpafta()
                .map(|text| (text.into_bytes(), Vec::new())),
        };
        // Whole or not at all (docs/adr/0253 §10): a failed write leaves the old file as it was.
        let written = made.and_then(|(bytes, findings)| {
            kentos_headless::files::write_bytes(&bytes, &path)
                .map(|_| findings)
                .map_err(|e| e.message)
        });
        match written {
            Ok(findings) => {
                self.output(format!("{} yazıldı.", path.display()));
                self.say_pdf_findings(&findings);
            }
            Err(e) => {
                self.sheets.output_failed(kind, false, &e);
                self.warn(format!("Dışa aktarılamadı: {e}"));
            }
        }
    }

    /// What a PDF writes differently from the screen (an SVG picture as a picture, or the missing
    /// picture's box), in the log: a note as information, the rest as a warning.
    pub(crate) fn say_pdf_findings(&mut self, findings: &[kentos_sheet::preflight::Finding]) {
        use kentos_sheet::preflight::Severity;
        for f in findings {
            match f.severity {
                Severity::Info => self.output(f.message.clone()),
                Severity::Warning | Severity::Error => {
                    self.warn(format!("{} {}", f.message, f.fix));
                }
            }
        }
    }
}
