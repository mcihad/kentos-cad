//! The shell's loop: what it listens to (keys, the window, timers, background work) and the handling
//! of every message.

use super::*;
use crate::arch;
use crate::gis;
use crate::map;

impl App {
    pub fn subscription(&self) -> Subscription<Message> {
        // While the drawing has unsaved changes, the app looks every few seconds whether a
        // recovery copy is due; a database project's unsent work goes to its device draft instead.
        let unsaved = self.recovery.on()
            && self
                .document
                .as_ref()
                .is_some_and(|d| d.dirty() && !d.is_database());
        Subscription::batch([
            event::listen_with(if self.drawing_menu.is_some() {
                keys::drawing_menu_key_event
            } else {
                keys::key_event
            }),
            window::close_requests().map(Message::CloseRequested),
            if unsaved {
                Subscription::run(recovery::ticks)
            } else {
                Subscription::none()
            },
            if self.sheet_publish_running(){Subscription::run(crate::exchange::sheet_publish::ticks)}else{Subscription::none()},
            // Veri çıkarımı's preview of a large drawing running on its own thread (docs/adr/1131 §3).
            if self.extract.as_ref().is_some_and(|x| x.running.is_some()) {
                Subscription::run(crate::tables::extract::ticks)
            } else {
                Subscription::none()
            },
            // The labels' answers to the extensions' functions, while asked or come (extensions/tools.rs).
            if self.extensions.tools.labels.waiting() {
                Subscription::run(crate::extensions::tools::label_ticks)
            } else {
                Subscription::none()
            },
            // Toplu çalıştır: a running batch job is looked at every tick (processing/batch.rs).
            if self.gis.processing.batch_running() {
                Subscription::run(crate::processing::batch::ticks)
            } else {
                Subscription::none()
            },
            // A rebuild running as a job in the project tree (road_project_panel.rs, docs/adr/2440).
            if self.road.road_project.running() {
                Subscription::run(crate::road_project_panel::ticks)
            } else {
                Subscription::none()
            },
            // A file dropped on the window goes to the SVG editor while it is open (style/svgedit/), else to
            // Kaynaklar, which adds it by its kind (sources/, docs/adr/0599 §8).
            if self.styles.svg_editor.is_some() {
                event::listen_with(crate::style::svgedit::dropped)
            } else {
                event::listen_with(crate::sources::dropped)
            },
            // A Kaynaklar row being dragged: the pointer and the button let go, wherever they are.
            if self.gis.sources.drag.is_some() {
                event::listen_with(crate::sources::dragging)
            } else {
                Subscription::none()
            },
            // The cloud's timers: autosave, the draft, following, the catalog's search.
            if self.cloud.wants_ticks() {
                Subscription::run(cloud::ticks)
            } else {
                Subscription::none()
            },
            // The sheet templates' library: its long poll's next wait, a retry, a run after events.
            if self.sheet_library.wants_ticks() {
                Subscription::run(crate::sheet_library::ticks)
            } else {
                Subscription::none()
            },
            // The rasters' tiles as they are made (rasters/tiles.rs, docs/adr/0204 §5).
            if crate::rasters::tiles::in_use() {
                Subscription::run(crate::rasters::tiles::ready)
            } else {
                Subscription::none()
            },
            // The point clouds' nodes as they are made (pointclouds/service.rs, docs/adr/0207 §6).
            if crate::pointclouds::service::in_use() {
                Subscription::run(crate::pointclouds::ready)
            } else {
                Subscription::none()
            },
            // The map services' tiles as they come (services/, docs/adr/0208 §6).
            if crate::services::in_use() {
                Subscription::run(crate::services::ready)
            } else {
                Subscription::none()
            },
            // The loaded external references' files, looked at every few seconds (xrefs_watch.rs, docs/adr/0349 §3).
            if self.xrefs_watching() {
                Subscription::run(crate::xrefs_watch::ticks)
            } else {
                Subscription::none()
            },
            // Zaman sürgüsü's playback (temporal/, docs/adr/0210 §5): a tick is handled after the
            // drawing showed the last step; the step itself waits for the speed.
            if self.gis.time.slider.playing {
                Subscription::run(crate::temporal::ticks)
            } else {
                Subscription::none()
            },
            // The status bar's message: when it goes, and its fades (message_log.rs).
            self.log_subscription(Instant::now()),
            self.block_convert_subscription(),
            self.array_find_subscription(),
            // The kept layout, written after its last change; the window's size.
            self.layout_subscription(),
            // Genel bakış's picture, drawn again a moment after a change (navigation_cards.rs).
            self.navigation_subscription(),
            window::resize_events().map(|(_, size)| Message::WindowResized(size)),
            iced::event::listen_with(device_scale),
            // A large import writes a slice of its objects each frame (exchange/drawing_import.rs).
            if self.importing.is_some() {
                window::frames().map(|_| {
                    Message::Exchange(Box::new(crate::exchange::Event::DrawingImport(
                        crate::exchange::drawing_import::Event::Frame,
                    )))
                })
            } else {
                Subscription::none()
            },
            // A bind on its own thread: its job looked at and its bar moved each frame (exchange/binding.rs, docs/adr/0757).
            if self.binding.is_some() {
                window::frames().map(|_| {
                    Message::Exchange(Box::new(crate::exchange::Event::Binding(
                        crate::exchange::binding::Event::Frame,
                    )))
                })
            } else {
                Subscription::none()
            },
            // A topology check on its own thread: its bar moves each frame (topology/running.rs, docs/adr/0756).
            if self.gis.topology.running.is_some() {
                window::frames()
                    .map(|_| Message::Gis(gis::Event::Topology(crate::topology::Event::Frame)))
            } else {
                Subscription::none()
            },
            // İstatistik's summaries, a few fields a frame (statistics/, docs/adr/0227 §11).
            if self.stats_counting() {
                window::frames()
                    .map(|_| Message::Gis(gis::Event::Statistics(crate::statistics::Event::Count)))
            } else {
                Subscription::none()
            },
            // While the key tips show (or Alt is down alone): a click or the
            // window losing the focus sends them away (ribbon_keys.rs).
            if self.key_tips.is_some() || self.alt_armed {
                event::listen_with(crate::ribbon_keys::away_events)
            } else {
                Subscription::none()
            },
        ])
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        // A lock that changes with the pointer still shows at once (locks.rs);
        // the drawing area's own events move the pointer themselves.
        let locks = (!matches!(message, Message::Viewport(_)))
            .then_some((self.locks.length, self.locks.toward));
        let held = self.window_over();
        let view = self.viewpoint();
        let task = self.handle(message);
        // A window resized since the last message keeps its size (dialog_sizes.rs, docs/adr/0746).
        self.dialog_sizes.write_changed();
        // A template's run ends with its tool (templates.rs).
        self.follow_template();
        // The running command's preview follows a view change and a tool run over it (navigation.rs, docs/adr/0283).
        self.follow_view(view);
        // Another drawing goes back to one view (split_view.rs).
        self.follow_split();
        if locks.is_some_and(|l| l != (self.locks.length, self.locks.toward)) {
            self.repoint();
        }
        self.follow_document();
        // Önceki's history follows the drawing's UCS (ucs.rs, docs/adr/0261 §1).
        self.follow_ucs();
        // The sheets follow the project and the drawing (sheets.rs).
        self.follow_sheets();
        self.follow_wall_view();
        self.follow_selection_layers();
        // Follow selection before rebuilding keyboard targets for the visible tab.
        self.follow_selection_tab();
        // The key tips follow what the ribbon shows now (ribbon_keys.rs).
        self.refresh_key_tips();
        self.dialog_back();
        let task = Task::batch([
            task,
            self.text_field_tasks(),
            self.paragraph_tasks(),
            self.blocks_tasks(),
            self.template_editor_tasks(),
            self.attribute_values_tasks(),
            self.find_replace_tasks(),
            self.text_file_tasks(),
            self.image_file_tasks(),
            // A drawn object's form (forms/mod.rs, docs/adr/0403 §4).
            self.open_form_on_create(),
            // The attachments' photos the shell shows, decoded off the thread (attachments.rs).
            self.photo_tasks(),
            self.raster_values_tasks(),
            self.service_info_tasks(),
            self.cloud_query_tasks(),
            self.network_tasks(),
            // Konumdan adres's places, asked on the address thread or of a provider (address/).
            self.address_tasks(),
            self.command_tasks(),
            // An extension's map tool: the shape drawn, run through its code (extensions/runner.rs).
            self.ext_capture_tasks(),
            // The Python environment of an extension the manager chose itself (extensions/environment.rs).
            self.ext_look_tasks(),
            self.follow_hover(),
            self.follow_tracking(),
            // The grids the project's datum choices name, read into the core (grids.rs).
            self.follow_grids(),
            // The rows round a bound layer's part, the obstacles of its labels at the part's edge (band_ghosts.rs).
            self.follow_bands(),
            // The drawing's new calls join Komut geçmişi before the log is followed (history/).
            {
                self.follow_history();
                self.follow_log(Instant::now())
            },
            // The operations and selections as code in the Python console when asked (python/echo.rs).
            self.follow_python_echo(),
            // The sheet templates' library follows the sign-in and the connection (sheet_library.rs).
            self.follow_sheet_library(Instant::now()),
        ]);
        self.follow_layout(Instant::now());
        self.follow_navigation(Instant::now());
        // İstatistik and Grafik keep the layer they read (charts/).
        self.follow_charts();
        // Bilgi's card goes with its tool and its object (info_card.rs).
        self.follow_info_card();
        // Google's geocoding answers only beside a shown Google basemap (address/online.rs).
        self.follow_address();
        self.cloud_after(Instant::now());
        // A window that opens over the drawing has the keyboard: a text box under
        // it (the command line that opened it) lets go first, before the window's
        // own field takes it. The view keeps the window's widgets as they were
        // when something comes over them (view.rs), so nothing else would.
        if !held && self.window_over() {
            self.line_focused = false;
            return release_keyboard().chain(task);
        }
        task
    }

    /// Whether a window is open over the drawing: a dialog, the application
    /// menu or the sheet templates' gallery.
    pub(super) fn window_over(&self) -> bool {
        self.dialog.is_some() || self.app_menu.is_some() || self.sheets.gallery_open()
    }

    /// After every message: the geometry store takes the drawing's changes
    /// and the selection lets go of objects that are gone (an undo, a
    /// delete, another editor's deletion taken in from the cloud; the web's
    /// `selection.retain` on `changed`), only when the drawing changed
    /// (docs/adr/0029). It keys on the generation, which changes from outside
    /// move too; the revision is the saves' (docs/adr/0040). The snap marker
    /// belongs to a running tool.
    pub(crate) fn follow_document(&mut self) {
        if !self.session.is_running() {
            self.snap = None;
        }
        let Some(doc) = &self.document else {
            return;
        };
        let now = (doc.session, doc.model.generation());
        if self.followed == Some(now) {
            return;
        }
        self.followed = Some(now);
        self.spatial.sync(&doc.model);
        self.styles.follow_project(Some(doc.model.styles()));
        let model = &doc.model;
        self.selection.retain(|slot| model.get(slot).is_some() && model.isolation_shown(slot));
        // The slider's range follows the drawing (docs/adr/0210 §5).
        self.time_refresh();
        // A new object its layer's filter leaves out is said (docs/adr/0211 §3).
        self.say_hidden_new();
        self.follow_map();
        // Kat görünümü with Etkin kat as the drawing has its levels now (docs/adr/0715).
        self.sync_level_view();
        // Undo/Redo and script writes can arrive while the pointer rests
        // over a panel. The active tool's cached handles must follow too.
        self.with_tool(|s, cx| s.document_changed(cx));
        // Boru hattı denetimi open: its answer read again from the network as it is now (docs/adr/4360).
        if self.dialog == Some(crate::app::Dialog::Map(map::MapDialogKind::PipesCheck)) {
            self.pipes_check_run();
        }
    }

    /// The drawing area draws in the project's map system (docs/adr/0583 §3): vectors moved in the style engine's
    /// sink, pictures, rasters and point clouds meshed onto it as they are drawn (docs/adr/0630).
    pub(super) fn follow_map(&mut self) {
        let Some(doc) = &self.document else {
            return;
        };
        let map = kentos_project::systems::map_projection(doc.model.settings());
        if let Some(split) = self.split.as_mut() {
            for o in &mut split.others {
                o.viewport.set_map(map);
            }
        }
        self.viewport.set_map(map);
    }

    pub(super) fn handle(&mut self, message: Message) -> Task<Message> {
        if matches!(message, Message::InputFocusLost) {
            self.input_focus_lost();
            return Task::none();
        }
        if matches!(message, Message::CapturedEscape) {
            if self.field.is_some() && self.drawing_menu.take().is_some() {
                return Task::none();
            }
            if self.field.take().is_some() {
                self.refresh_value_preview();
                return crate::input::release_keyboard();
            }
            // Esc in a window's text box closes the window, as Esc beside it does (DESIGN.md §7.9): F1 stayed open over
            // its search box (plan G11), and the editors with their own save ask first (docs/adr/8006).
            if matches!(
                self.dialog,
                Some(
                    Dialog::Gis(gis::GisDialogKind::AttributeValues)
                        | Dialog::BlockDefine
                        | Dialog::Shortcuts
                        | Dialog::Arch(arch::ArchDialogKind::WallStyles)
                        | Dialog::Arch(arch::ArchDialogKind::Program)
                        | Dialog::Arch(arch::ArchDialogKind::OpeningTypes)
                        | Dialog::Arch(arch::ArchDialogKind::Materials)
                )
            ) {
                self.close_dialog();
            }
            return Task::none();
        }
        if let Message::ScopedReply {
            document,
            run,
            reply,
        } = message
        {
            if self
                .document
                .as_ref()
                .is_some_and(|d| d.session == document)
                && run.is_none_or(|run| self.session.runs() == run)
            {
                return self.handle(*reply);
            }
            return Task::none();
        }
        // What the drawing area's device can draw with, known after its first frame (AA-01).
        self.sync_device();
        // Dots Nokta yoğunluğu left out are said once a layer (docs/adr/0213 §2.4).
        for (name, n) in self.viewport.take_dropped() {
            self.warn(format!(
                "“{name}” katmanında Nokta yoğunluğunun {} noktası çizilmedi: bir katmanda en çok 1 000 000 nokta çizilir. Nokta değerini büyütün.",
                crate::view::thousands(n as f64)
            ));
        }
        // What the extensions' İşlemler tools noted on their threads: their logs and notices (extensions/tools.rs).
        self.ext_tool_notes();
        // While a drawing is being opened the app takes no command (opening.rs).
        if let Some(task) = self.while_opening(&message) {
            return task;
        }
        // While Python code runs the drawing takes no other edit (python/), nor while an extension's code may write it
        // (extensions/runner.rs).
        if let Some(task) = self.while_scripting(&message) {
            return task;
        }
        if let Some(task) = self.while_extension_writes(&message) {
            return task;
        }
        // Nor while a large import goes in, a frame at a time (exchange/drawing_import.rs).
        if let Some(task) = self.while_importing(&message) {
            return task;
        }
        // A command from the ribbon or a menu keeps the text field's text first (the web's blur),
        // and takes the keyboard from the layer tree (the web's button takes the focus).
        if matches!(
            message,
            Message::Run(_)
                | Message::RunMethod { .. }
                | Message::SplitChosen { .. }
                | Message::DrawTemplate(_)
                | Message::ApplyTemplate(_)
        ) {
            // A command closes the folded ribbon open over the drawing (the web's).
            if !matches!(message, Message::Run("view.keyTips")) {
                self.ribbon_peek = false;
            }
            self.close_text_field(true);
            self.close_paragraph(true);
            self.layers_keyboard = false;
            self.blocks_panel.keyboard = false;
            self.arch.templates_panel.keyboard = false;
            self.vertices.keyboard = false;
        }
        match message {
            Message::Run(id) | Message::RunKept(id) => return self.run(id),
            Message::DrawTemplate(id) => return self.draw_template(&id),
            Message::ApplyTemplate(id) => self.apply_template(&id),
            Message::ActiveLevel(level) => self.choose_level(level),
            Message::LayerKeys(on) => self.choose_layer_keys(on),
            Message::LevelView(view) => self.choose_level_view(view),
            Message::RunMethod { id, option, label } => {
                return self.run_method(id, option, label);
            }
            Message::SplitChosen {
                key,
                id,
                option,
                label,
            } => return self.split_chosen(key, id, option, label),
            Message::QuickAccess(id, on) => self.quick_access_changed(&id, on),
            Message::ShortcutsSearch(text) => self.shortcuts_query = text,
            Message::Swallowed => {}
            Message::KeyTipsAway => {
                self.key_tips = None;
                self.alt_armed = false;
            }
            Message::RibbonPeekAway => self.ribbon_peek = false,
            // The contextual Pafta tab is the sheet mode's; any other closes it (sheets.rs).
            Message::RibbonTab(id) => {
                self.sheet_tab = id == crate::sheets::SHEET_TAB;
                if !self.sheet_tab {
                    self.tab_clicked(id);
                }
            }
            Message::Sheet(m) => return self.sheet_message(m),
            Message::SheetLibrary(m) => return self.sheet_library_message(m),
            Message::SheetExportTo(kind, path) => self.sheet_export_to(kind, path),
            Message::RibbonPanel(event) => self.ribbon_panel_event(event),
            Message::CommandInput(text) => self.command_input = text,
            Message::CommandSubmitted => {
                let text = std::mem::take(&mut self.command_input);
                return self.submit_line(text.trim());
            }
            Message::CommandRun(name) => {
                self.command_input.clear();
                return self.run_typed(&name);
            }
            Message::CommandHistoryToggled => self.toggle_bottom(),
            Message::BottomTab(tab) => self.show_bottom(tab),
            Message::Gis(inner) => return self.gis_event(inner),
            Message::TableWritten(outcome) => match outcome {
                crate::table_write::Outcome::Written { file, rows, notes } => {
                    self.say(Level::Success, format!("“{file}” yazıldı: {rows} satır."));
                    for n in notes {
                        self.say(Level::Info, format!("“{file}”: {n}"));
                    }
                }
                crate::table_write::Outcome::Failed(why) => self.warn(why),
            },
            Message::Search(event) => return self.data_event(event),
            Message::History(event) => return self.calls_event(event),
            Message::Arch(inner) => return self.arch_event(inner),
            Message::CadInspection(event) => return self.cad_inspection_event(event),
            Message::Recover(Some(path)) => return self.recover_path(path),
            Message::Recover(None) => {}
            Message::Vertices(event) => return self.vertices_event(event),
            Message::BottomResized(height) => self.bottom_dragged(Some(height), Instant::now()),
            Message::BottomReset => self.bottom_dragged(None, Instant::now()),
            Message::HistoryCleared => self.clear_history(),
            Message::AskProjectType => self.ask_project_type(),
            Message::AskCadWork => self.ask_cad_work(),
            Message::LogFrame => self.log_frame(Instant::now()),
            Message::LogScrolled(viewport) => self.log_scrolled(viewport),
            Message::LayoutSave => self.layout.write(Instant::now(), false),
            Message::BandRead(read) => self.band_read(read),
            Message::WindowResized(size) => self.window_resized(size),
            Message::DeviceScale(scale) => kentos_ui::widget::brand::set_device_scale(scale),
            // The next frame draws them; the bottom panel shows the pyramids' progress.
            Message::RastersReady => {
                // A raster's address changed: it is read again, said once (docs/adr/0642 §2).
                for words in crate::rasters::tiles::service().notices() {
                    self.say(Level::Info, words);
                }
            }
            // The next frame draws them; the panel shows the indexes' progress.
            Message::CloudsReady => {}
            // The next frame draws them; a service that failed is said once.
            Message::ServicesReady => {
                for text in crate::services::hub().take_notices() {
                    self.warn(text);
                }
            }
            Message::Services(event) => return self.services_event(event),
            Message::Bound(event) => return self.bound_event(event),
            Message::CommandCancelled => {
                self.line_focused = false;
                return self.run("tool.cancel");
            }
            Message::CommandFocus(focused) => {
                self.line_focused = focused;
                // The value field and the layer tree lose the keyboard to the command line (web: their blur).
                if focused {
                    self.field = None;
                    self.layers_keyboard = false;
                    self.blocks_panel.keyboard = false;
                    self.arch.templates_panel.keyboard = false;
                    self.vertices.keyboard = false;
                }
            }
            Message::PromptOption(key) => return self.prompt_option(key),
            Message::PromptChoice(key, typed) => return self.prompt_choice(key, &typed),
            Message::Map(inner) => return self.map_event(inner),
            Message::Key(press) => return self.key(press),
            Message::CursorValueInput { token, text, pasted } => {
                return self.value_field_input(token, text, pasted);
            }
            Message::CursorValueSubmit(token) => return self.value_field_submit(token),
            Message::InputFocusLost => self.input_focus_lost(),
            Message::CapturedEscape => unreachable!("checked before dispatch"),
            Message::ScopedReply { .. } => unreachable!("checked before dispatch"),
            Message::Modifiers(modifiers) => {
                let shift = self.modifiers.shift() != modifiers.shift();
                self.modifiers = modifiers;
                self.sheets.set_modifiers(modifiers);
                // Shift turns ortho over while held (docs/adr/0268 §2): the preview follows at once, the pointer still.
                if shift && self.session.takes_points() {
                    self.repoint();
                }
                // Alt let go after a tap alone: the ribbon's key tips (ribbon_keys.rs).
                return self.alt_released(modifiers);
            }
            Message::Dock(event) => {
                if matches!(event, kentos_ui::widget::docking::Event::Closed(Panel::Blocks)) {
                    self.close_block_find();
                }
                self.docks.update(event.clone());
                self.dock_dragged(&event, Instant::now());
            }
            Message::LayerSelected(id) => self.layer_pressed(id),
            Message::SelectUid(uid) => {
                let slot = self.document.as_ref().and_then(|d| {
                    kentos_domain::Uuid::parse_str(&uid)
                        .ok()
                        .and_then(|u| d.model.slot_of(u))
                });
                if let Some(slot) = slot {
                    self.selection.set(vec![slot]);
                    let ids = [f64::from(slot.0)];
                    self.navigating(|app| app.zoom_to_objects(&ids));
                }
            }
            Message::LayerSearch(text) => self.layer_search(text),
            Message::LayerProps(prop) => self.layer_props(prop),
            Message::LayerSearchDown => return self.layer_search_down(),
            Message::RibbonSearch(text) => self.search_typed(text),
            Message::RibbonSearchRun(index) => return self.search_run(index),
            Message::RibbonSearchReveal(index) => return self.search_reveal(index),
            Message::RibbonFlashEnd(id) => self.search_flash_end(id),
            Message::RibbonModelFlashEnd(id) => self.search_model_flash_end(&id),
            Message::Layer(event) => return self.layer_event(event),
            Message::DrawingMenu(event) => return self.drawing_menu_event(event),
            Message::DropLock(length) => self.drop_lock(length),
            Message::TextField(event) => self.text_field_event(event),
            Message::Paragraph(event) => self.paragraph_event(event),
            Message::Properties(event) => self.properties_event(event),
            Message::Blocks(event) => self.blocks_event(event),
            Message::BlocksPanel(event) => return self.blocks_panel_event(event),
            Message::Navigator(event) => return self.navigator_event(event),
            Message::Road(inner) => return self.road_event(inner),
            Message::BlockAttributes(event) => return self.block_attributes_event(event),
            Message::BlockCard(event) => return self.block_card_event(event),
            Message::BlockParameterGroups(event) => return self.block_parameter_groups_event(event),
            Message::BlockTypeValues(event) => return self.block_type_values_event(event),
            Message::BlockSchedule(event) => return self.block_schedule_event(event),
            Message::FindReplace(event) => return self.find_replace_event(event),
            Message::DrawingStandard(event) => return self.drawing_standard_event(event),
            Message::ReviewNotes(event) => return self.review_notes_event(event),
            Message::StandardInstall(event) => return self.standard_install_event(event),
            Message::LineTypes(event) => return self.line_types_event(event),
            Message::Parameters(event) => return self.parameters_event(event),
            Message::HatchPatterns(event) => return self.hatch_patterns_event(event),
            Message::StyleImpact(event) => return self.style_impact_event(event),
            Message::BlockEditor(event) => return self.block_editor_event(event),
            Message::DocTabs(event) => return self.doc_tabs_event(event),
            Message::Attachment(event) => return self.attachment_event(event),
            Message::InfoCard(event) => return self.info_card_event(event),
            Message::LayerStates(event) => return self.layer_states_event(event),
            Message::Views(event) => return self.views_event(event),
            Message::Propose(event) => return self.propose_event(event),
            Message::SavePicture(saved) => return self.save_picture(saved),
            Message::PictureWritten(written) => self.picture_written(written),
            Message::Scene3d(event) => return self.scene3d_event(event),
            Message::LandxmlExport(path) => return self.landxml_chosen(path),
            Message::LandxmlMachine(path) => return self.landxml_machine_chosen(path),
            Message::LandxmlAlignments(path) => return self.landxml_alignments_chosen(path),
            Message::IfcExport(path) => return self.ifc_chosen(path),
            Message::IfcBuildingExport(path) => return self.ifc_building_chosen(path),
            Message::GltfExport(path) => return self.gltf_chosen(path),
            Message::AnnotationStyles(event) => return self.annotation_styles_event(event),
            Message::TableStyleDefs(event) => return self.table_style_window_event(event),
            Message::SheetSet(event) => return self.sheet_set_event(event),
            Message::Xrefs(event) => return self.xrefs_event(event),
            Message::Flatten(event) => return self.flatten_event(event),
            Message::CleanReport(event) => return self.clean_report_event(event),
            Message::OpenReport(event) => self.open_report_event(event),
            Message::ExchangeCheck(event) => return self.exchange_check_event(event),
            Message::CatalogReport(event) => return self.catalog_report_event(event),
            Message::XrefWatch(event) => return self.xref_watch_event(event),
            Message::BondNotice(event) => return self.bond_notice_event(event),
            Message::LayerPurge(event) => return self.layer_purge_event(event),
            Message::DrawingExchange(event) => return self.drawing_exchange_event(event),
            Message::LayerList(event) => return self.layer_list_event(event),
            Message::FontReport(event) => return self.font_report_event(event),
            Message::TextTools(event) => return self.text_tools_event(event),
            Message::LeaderTools(event) => return self.leader_tools_event(event),
            Message::PlotScale(event) => return self.plot_scale_event(event),
            Message::TableInsert(event) => return self.table_insert_event(event),
            Message::TableEditor(event) => return self.table_editor_event(event),
            Message::TableUpdate(event) => return self.table_update_event(event),
            Message::TableExport(event) => return self.table_export_event(event),
            Message::Extract(event) => return self.extract_event(event),
            Message::SheetDetails(event) => return self.sheet_details_event(event),
            Message::PlotStyles(event) => return self.plot_styles_event(event),
            Message::Navigation(event) => self.navigation_event(event),
            Message::TextFile(file) => self.text_file_given(file),
            Message::ImageFile(file) => self.image_file_given(file),
            Message::HoverCard(version) => self.hover_card_due(version),
            Message::TrackDwell(number) => {
                let spatial = &self.spatial;
                self.tracking
                    .dwell_due(number, |id, at| spatial.extensions_at(id, at));
            }
            Message::LayerStyle(event) => return self.layer_style_event(event),
            Message::StyleManager(event) => return self.style_manager_event(*event),
            Message::Legend(event) => return self.legend_event(event),
            Message::Designer(event) => return self.designer_event(*event),
            Message::SvgEdit(event) => return self.svgedit_event(*event),
            Message::Python(event) => return self.python_event(event),
            Message::Assistant(event) => return self.assistant_event(event),
            Message::Extensions(event) => return self.extensions_event(event),
            // The layer tree's changes go through the document, as on the web: visibility
            // and lock are edits (unsaved) but not undo steps.
            // Through `cad.layers.set`, as Python and MCP do (docs/adr/0248 §6).
            Message::LayerVisible(id) => self.toggle_layer_visible(&id),
            Message::LayerLocked(id) => self.toggle_layer_locked(&id),
            Message::LayerPlot(id) => self.toggle_layer_plot(&id),
            Message::LayerFrozen(id) => self.toggle_layer_frozen(&id),
            Message::LayerExpanded(id) => {
                if let Some(doc) = &mut self.document
                    && let Some(expanded) = doc.find(&id).map(|n| !n.expanded)
                {
                    // Folding is kept in the file but is not an edit (web).
                    doc.model.set_layer_expanded(&id, expanded);
                }
            }
            Message::Opened(None) | Message::Saved(None) => {}
            Message::Opened(Some(Ok(mut doc))) => {
                // Its file as read now: a later save checks nobody wrote it meanwhile.
                if doc.disk.is_none() {
                    doc.disk = doc.path.as_deref().and_then(crate::document::disk_stamp);
                }
                // Bound GeoPackages moved with the project are found again (docs/adr/0506).
                let located = match doc.path.clone() {
                    Some(path) if doc.cloud_source().is_none() => {
                        kentos_native_application::source_places::relocate(&mut doc.model, &path)
                    }
                    _ => Default::default(),
                };
                // The grids it names found beside it too: a delivery package's izgaralar/ (docs/adr/3580).
                let grids = match doc.path.clone() {
                    Some(path) if doc.cloud_source().is_none() => kentos_native_application::project_grids::find(&doc.model, &path),
                    _ => Vec::new(),
                };
                match doc.cloud_source() {
                    Some(source) => self.say(
                        Level::Success,
                        format!(
                            "“{}” bulut projesi açıldı ({}, {}): {} nesne.",
                            doc.name(),
                            source.workspace,
                            cloud::words::storage_title(source.storage()),
                            doc.entity_count()
                        ),
                    ),
                    None => self.output(format!(
                        "{} açıldı: {} nesne, {} katman.",
                        doc.name(),
                        doc.entity_count(),
                        doc.layer_count()
                    )),
                }
                if doc.legacy {
                    self.output(
                        "Dosya eski biçimde (KCAD v1). Kaydet, yeni biçimde (v2) yazmak için yer sorar; eski dosyanın üzerine kendiliğinden yazmaz.",
                    );
                }
                self.show_document(*doc);
                // The level and view the drawing was saved with (docs/adr/0720 §1).
                self.say_level_state();
                self.say_located(&located);
                // Docs/adr/3580: the grids taken from beside the file, and any it names that are nowhere.
                if let Some(words) = kentos_native_application::project_grids::said(&grids) {
                    let lost = grids.iter().any(|g| matches!(g.place.name(), "missing" | "changed" | "unread"));
                    self.say(if lost { Level::Warn } else { Level::Info }, words);
                }
                // A drawing opened from a file goes first in the recent files (a recovered one has none).
                self.remember_file();
                // The health check on opening, when asked for (docs/adr/1009).
                return self.open_health_check();
            }
            Message::Opened(Some(Err(error))) => self.error(error),
            Message::Saved(Some(Err(error))) => {
                self.error(error);
                self.save_then_dropped();
            }
            Message::Saved(Some(Ok(written))) => match &mut self.document {
                Some(doc) if doc.session == written.session => {
                    // “Yerel dosyaya kaydet” (cloud/file_follow.rs): the work is in the
                    // file now, so a save of it kept for the project goes, and the
                    // drawing leaves the project, which stays as it is.
                    let detached = self
                        .cloud
                        .detaching
                        .take()
                        .filter(|(session, _)| {
                            *session == doc.session && doc.cloud_source().is_some()
                        })
                        .map(|(_, name)| name);
                    if detached.is_some()
                        && let Some(held) = self
                            .cloud
                            .held
                            .as_mut()
                            .filter(|h| h.session == doc.session && h.kept_save)
                        && held.replica.clear_save().is_ok()
                    {
                        held.kept_save = false;
                    }
                    doc.saved(written.path.clone(), written.revision);
                    let later = if doc.dirty() {
                        " Kayıt sürerken yapılan değişiklikler kaydedilmedi."
                    } else {
                        ""
                    };
                    self.output(format!("Kaydedildi: {}.{later}", written.path.display()));
                    if let Some(name) = detached {
                        self.cloud.file_conflict = None;
                        self.output(crate::cloud::revisions::texts::detached(&name));
                    }
                    self.recovery_saved();
                    self.remember_file();
                    // UR-001: “Kaydet ve …” goes on once its drawing is saved whole.
                    if let Some((session, then)) = self.save_then
                        && session == written.session
                    {
                        self.save_then = None;
                        if self.document.as_ref().is_some_and(|d| !d.dirty()) {
                            return self.proceed(then);
                        }
                        self.warn("Kayıt sürerken yapılan değişiklikler kaydedilmedi; çizim açık kaldı.");
                    }
                }
                // Its drawing set aside in its tab meanwhile (doc_tabs.rs, docs/adr/0351 §3): the file is
                // its file all the same.
                _ if let Some(doc) = self.doc_tabs.find_mut(written.session) => {
                    doc.saved(written.path.clone(), written.revision);
                    self.output(format!("Kaydedildi: {}.", written.path.display()));
                }
                // Another drawing was opened meanwhile: the file is written, but it is
                // not the open drawing's file, which keeps its path and its state.
                _ => self.output(format!("Kaydedildi: {}.", written.path.display())),
            },
            Message::CloseRequested(_) if self.block_editor.is_some() => {
                self.warn(crate::block_editor::OPEN);
            }
            Message::CloseRequested(window) => {
                // The layout is written now, not 250 ms later.
                self.layout.write(Instant::now(), true);
                // A save stopped by the window closing would leave no file (the previous one
                // stays): it finishes first.
                let unsaved = self.doc_tabs.unsaved();
                if let Some(s) = &self.saving {
                    let name = s.name();
                    self.warn(format!(
                        "{name} kaydediliyor; kayıt bitince pencereyi yeniden kapatın."
                    ));
                } else if !unsaved.is_empty() {
                    // A drawing set aside in its tab is not asked about from here: it is brought in front first.
                    self.warn(format!(
                        "Sekmelerdeki {} çizimde kaydedilmemiş değişiklik var ({}): o sekmeye geçip kaydedin ya da Çizimi kapat ile kapatın, sonra pencereyi yeniden kapatın.",
                        unsaved.len(),
                        unsaved.join(", ")
                    ));
                } else {
                    // Unsent cloud work to its draft first, else the question (cloud/leaving.rs).
                    return self.leave(Then::Close(window));
                }
            }
            Message::DialogConfirmed => match self.dialog.take() {
                Some(Dialog::Unsaved(then)) => {
                    // The unsaved changes are dropped on purpose: their recovery copy goes too.
                    self.recovery.discard();
                    self.cloud.leave_failure = None;
                    return self.proceed(then);
                }
                Some(Dialog::RemoveLayer) => {
                    if let Some(id) = self.removing_layer.take() {
                        self.remove_layer(&id);
                    }
                }
                Some(Dialog::Arch(arch::ArchDialogKind::RemoveTemplate)) => self.remove_template(),
                Some(Dialog::Arch(arch::ArchDialogKind::SpaceMerge)) => {
                    self.space_merge_confirmed()
                }
                Some(Dialog::SourceDiscard) => self.bound_discard(),
                _ => {}
            },
            Message::DialogSave => {
                if let Some(Dialog::Unsaved(then)) = self.dialog.take()
                    && let Some(session) = self.document.as_ref().map(|d| d.session)
                {
                    self.cloud.leave_failure = None;
                    self.save_then = Some((session, then));
                    return self.update(Message::Run("file.save"));
                }
            }
            Message::DialogClosed => self.close_dialog(),
            Message::Cloud(event) => return self.cloud_event(*event),
            Message::Exchange(event) => return self.exchange_event(*event),
            Message::Project(event) => return self.project_event(*event),
            Message::AppMenu(event) => return self.app_menu_event(event),
            Message::Start(event) => return self.start_event(event),
            Message::Viewport(event) => return self.pointer(event),
            Message::OtherViewport(i, event) => return self.other_viewport(i, event),
            Message::Split(event) => return self.split_event(event),
            Message::Settings(edit) => return self.settings_edit(edit),
            Message::Opening(event) => return self.opening_event(event),
            Message::Saving(event) => return self.saving_event(event),
            Message::Recovery(event) => return self.recovery_event(event),
            Message::ServerChecked(answer) => self.server_checked(answer),
            Message::OverlapLayer(id) => self.toggle_overlap_layer(id),
            Message::CycleTo(index) => {
                self.selection.cycle_to(index);
                self.say_sub_object();
            }
            Message::CycleHover(slot) => self.selection.set_hover(slot),
            Message::Snap(event) => self.snap_event(event),
            Message::ScreenScale(event) => return self.screen_scale_event(event),
            Message::ToastClosed(id) => {
                self.toasts.dismiss(id);
            }
        }
        Task::none()
    }
}
