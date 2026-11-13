//! Running commands: a web command id's desktop handler, undo and redo, a command's on or off state and
//! whether it can run, a name typed in the command line.

use super::*;

impl App {
    /// Runs a web command id: the desktop's handler, or a note that it is not here yet.
    pub fn run(&mut self, id: &'static str) -> Task<Message> {
        // Whatever runs a command closes the application menu (the web's `executed`).
        self.app_menu = None;
        let Some(command) = catalog().get(id) else {
            self.error(format!("Komut bulunamadı: {id}"));
            return Task::none();
        };
        // A module's program runs only its own commands, whatever asks (docs/adr/0802): a menu, a panel or a cell
        // left from another program's layout finds the same refusal as a typed name.
        if let Some(why) = self.not_offered(id, command.title) {
            self.error(why);
            return Task::none();
        }
        match command.standing {
            Standing::Ported => {}
            Standing::OnTheWeb => {
                self.output(format!(
                    "“{}” web'de var; masaüstüne henüz taşınmadı.",
                    command.title
                ));
                return Task::none();
            }
            Standing::Pending => {
                let why = command.pending_note.unwrap_or("Geliştirme aşamasında");
                self.output(format!("“{}”: {why}.", command.title));
                return Task::none();
            }
        }

        // While Blok düzenleyici is open the drawing set aside waits: its file, cloud and sheet commands are refused
        // (block_editor.rs, docs/adr/0316 §1).
        if self.block_editor.is_some()
            && ["file.", "cloud.", "sheet.", "project."]
                .iter()
                .any(|p| id.starts_with(p))
        {
            self.warn(crate::block_editor::OPEN);
            return Task::none();
        }
        // Yeni sekmede aç, Çizimi kapat, Sonraki çizim (doc_tabs.rs, docs/adr/0351 §3).
        if crate::doc_tabs::COMMANDS.contains(&id) {
            return self.doc_tabs_command(id);
        }
        if let Some(tool) = id.strip_prefix("tool.")
            && Session::tools().contains(&tool)
        {
            return self.start_tool(tool);
        }
        // Modele dön (MODEL), as the web's: the sheet mode's own message (docs/adr/0164).
        if id == "sheet.drawingSet" {
            if self.sheets.book().sheets.is_empty() { self.warn("Önce bir pafta oluşturun."); return Task::none(); }
            let show = if self.sheets.is_active() { Task::none() } else { self.update(Message::Sheet(kentos_sheet_ui::Message::Tab(1))) };
            let open = self.update(Message::Sheet(kentos_sheet_ui::Message::DrawingSet(kentos_sheet_ui::drawing_set::SetMessage::Open)));
            return Task::batch([show,open]);
        }
        if id == "sheet.model" {
            if !self.sheets.is_active() {
                self.output("Model zaten önde.");
                return Task::none();
            }
            return self.update(Message::Sheet(kentos_sheet_ui::Message::Model));
        }
        if matches!(id, "sheet.fitDrawing" | "sheet.fitSelection") {
            if !self.sheets.is_active() || self.sheets.selection().is_empty() {
                self.warn("Paftada bir sabit görünüm penceresi seçin.");
                return Task::none();
            }
            return self.update(Message::Sheet(if id == "sheet.fitDrawing" {
                kentos_sheet_ui::Message::MapFitDrawing
            } else { kentos_sheet_ui::Message::MapFitSelection }));
        }
        if id.starts_with("cloud.") {
            return self.cloud_command(id);
        }
        // A ready basemap (services/basemaps.rs, docs/adr/0208 §14).
        if id.starts_with("basemap.") && id != "basemap.remove" {
            return self.basemap_command(id);
        }
        if crate::exchange::COMMANDS.contains(&id) {
            return self.exchange_command(id);
        }
        if crate::project::COMMANDS.contains(&id) {
            return self.project_command(id);
        }
        if crate::calc::COMMANDS.contains(&id) {
            return self.calc_command(id);
        }
        if crate::processing::answers(id) {
            return self.processing_command(id);
        }
        if crate::clipboard::COMMANDS.contains(&id) {
            return self.clipboard_command(id);
        }
        if crate::view_commands::COMMANDS.contains(&id) {
            return self.view_command(id);
        }
        // The snap kinds one by one and Çizilmekte olan nesneye (snap_menu.rs, docs/adr/0163 §6).
        if self.toggle_snap_setting(id) {
            return Task::none();
        }
        // Önceki and Sonraki görünüm, Kapsam denetimi (navigation.rs, docs/adr/0141).
        if crate::navigation::COMMANDS.contains(&id) {
            return self.navigation_command(id);
        }
        // Görünümler and the view's twist (views.rs, docs/adr/0285).
        if crate::views::COMMANDS.contains(&id) {
            return self.views_command(id);
        }
        // Formlar and Formla düzenle (forms/, docs/adr/0401).
        if crate::forms::COMMANDS.contains(&id) {
            return self.forms_command(id);
        }
        // Çizimler arası alışveriş (drawing_exchange.rs, docs/adr/0193).
        if crate::drawing_exchange::COMMANDS.contains(&id) {
            return self.drawing_exchange_command(id);
        }
        match id {
            // The drawing on screen is left first: its unsent cloud work to its draft, or the question.
            "file.open" => return self.leave(Then::Open),
            "file.recover" => return self.recover_file(),
            "file.start" => self.open_start(),
            // A cloud project saves to the server (cloud/file.rs); Farklı kaydet writes a local file.
            "file.save"
                if self
                    .document
                    .as_ref()
                    .is_some_and(|d| d.cloud_source().is_some()) =>
            {
                return self.save_cloud();
            }
            "file.save" => return self.save(false),
            "file.saveAs" => return self.save(true),
            "view.theme.dark" => self.choose_theme(Mode::Dark),
            "view.theme.light" => self.choose_theme(Mode::Light),
            "view.theme.toggle" => self.choose_theme(self.mode.toggled()),
            id if id.starts_with("workspace.") => self.choose_mode(id),
            "tools.options" => self.open_settings(),
            // Eklentiler (docs/adr/8057 §3): the manager, and a package straight to its review.
            "extensions.manage" => self.open_extensions(),
            "extensions.install" => return self.install_extension(),
            // Yeni pafta and Şablondan pafta (docs/adr/0622 §3): the sheet mode's own, from the ribbon.
            "sheet.new" => return self.sheet_message(kentos_sheet_ui::Message::NewSheet),
            // Kat planı paftaları (docs/adr/0718): the sheet mode's own, from the drawing too.
            "sheet.levels" => return self.sheet_message(kentos_sheet_ui::Message::LevelSheets),
            "sheet.fromTemplate" => {
                return self.sheet_message(kentos_sheet_ui::Message::Gallery(
                    kentos_sheet_ui::gallery::GalleryMessage::Open,
                ));
            }
            // Koordinat oku: a tool that reads the clicked point into the log (docs/adr/0140).
            "crs.query" => return self.start_tool(kentos_interaction::coordinate::ID),
            "draft.ortho" => self.toggle_session("drafting.ortho", "Orto"),
            "draft.rightAngle" => self.toggle_session("drafting.rightAngle", "Dik açı"),
            "draft.polar" => self.toggle_session("drafting.polar", "Kutupsal izleme"),
            "draft.tracking" => self.toggle_session("drafting.tracking", "Nesne izleme"),
            "draft.topology" => self.toggle_session("drafting.topology", "Topolojik düzenleme"),
            "draft.topologyPoints" => self.toggle_session(
                "drafting.topologyPoints",
                "Topolojik düzenlemede noktalar da",
            ),
            "draft.topologyStopOnLocked" => self.toggle_session(
                "drafting.topologyStopOnLocked",
                "Topolojik düzenlemede kilitli komşuda durdur",
            ),
            "draft.overlap" => self.toggle_overlap(),
            // The digitizing locks (docs/adr/0166 §6), on the running command.
            "draft.lock.length" => self.ask_lock(kentos_interaction::LockAsk::Length),
            "draft.lock.angle" => self.ask_lock(kentos_interaction::LockAsk::Angle),
            "draft.lock.deflection" => self.ask_lock(kentos_interaction::LockAsk::Deflection),
            "draft.lock.east" => self.ask_lock(kentos_interaction::LockAsk::East),
            "draft.lock.north" => self.ask_lock(kentos_interaction::LockAsk::North),
            "draft.lock.parallel" => {
                self.with_tool(|s, cx| {
                    s.pick_lock_edge(kentos_interaction::LockPick::Parallel, cx)
                });
            }
            "draft.lock.perpendicular" => {
                self.with_tool(|s, cx| {
                    s.pick_lock_edge(kentos_interaction::LockPick::Perpendicular, cx)
                });
            }
            "draft.lock.reference" => {
                self.with_tool(|s, cx| s.ask_reference(cx));
            }
            // Geçici izleme noktası (docs/adr/0270 §3).
            "draft.trackPoint" => {
                self.with_tool(|s, cx| s.ask_track_point(cx));
            }
            "draft.lock.construction" => {
                let on = !self.session.construction();
                self.with_tool(|s, cx| s.set_construction(on, cx));
            }
            "draft.lock.keep" => {
                let keep = !self.locks.keep;
                self.with_tool(|s, cx| s.keep_locks(keep, cx));
            }
            "draft.lock.clear" => {
                self.with_tool(|s, cx| s.clear_locks(cx));
            }
            "draft.overlap.allow" => self.choose_overlap(kentos_interaction::Overlap::Allow),
            "draft.overlap.layer" => self.choose_overlap(kentos_interaction::Overlap::Layer),
            "draft.overlap.layers" => self.choose_overlap(kentos_interaction::Overlap::Layers),
            "draft.snap" => self.toggle_session("drafting.snap", "Kenetleme"),
            "draft.grid" => self.toggle_session("drafting.grid", "Izgara"),
            // The styled drawing's view choices (style/, docs/adr/0090).
            "view.lineWeights" => self.toggle_session("graphics.lineWeights", "Çizgi kalınlığı"),
            // Görünüm kipleri (docs/adr/0195).
            "view.colorMode.color" | "view.colorMode.mono" | "view.colorMode.gray" => {
                self.choose_color_mode(id);
            }
            // Yazıların boyu (docs/adr/0205 §5).
            "view.annotationSize.legible"
            | "view.annotationSize.true"
            | "view.annotationSize.screen" => self.choose_annotation_size(id),
            "view.fills" => self.toggle_session("graphics.fills", "Dolgular ve taramalar"),
            "view.areaEdges" => self.toggle_session("graphics.areaEdges", "Alan sınırları"),
            "view.pointNames" => self.toggle_session("graphics.pointNames", "Nokta adları"),
            "view.pointCodes" => self.toggle_session("graphics.pointCodes", "Nokta kodları"),
            "view.transparency" => self.toggle_session("graphics.transparency", "Saydamlık"),
            // Sabit ve yerleşmeyen etiketler (docs/adr/0212 §4): the user's view, the drawing unchanged.
            "view.pinnedLabels" => {
                self.toggle_session("graphics.pinnedLabels", "Sabit etiketleri vurgula");
            }
            "view.unplacedLabels" => {
                self.toggle_session("graphics.unplacedLabels", "Yerleşmeyen etiketleri göster");
            }
            "style.layerStyle" => self.open_layer_style(None),
            // The style library (style/manager/, docs/adr/0092).
            "style.manager" => return self.open_style_manager(None, None),
            // Tablo (tables/, docs/adr/0184).
            "table.insert" => self.open_table_insert(false),
            "table.edit" => {
                if let [slot] = self.selection.ids()
                    && matches!(
                        self.document.as_ref().and_then(|d| d.model.get(*slot)),
                        Some(kentos_commands::Entity::Table(_))
                    )
                {
                    let slot = *slot;
                    return self.open_table_editor(slot);
                } else {
                    self.output("Bir tablo seçin (ya da tabloya çift tıklayın).");
                }
            }
            "table.update" => return self.update_tables(),
            "table.export" => return self.export_selected_table(),
            // Veri çıkarımı (docs/adr/1030 §8).
            "table.extract" => return self.open_extract(),
            "sheet.details" => self.open_sheet_details(),
            "sheet.plotStyles" => self.open_plot_styles(),
            // Tablonun dosyası ve blok çizelgesi (docs/adr/0327 §4–§5).
            "table.link" => return self.link_table(),
            "table.blockSchedule" => self.place_block_schedule(),
            // Yazı ve ölçü stilleri (docs/adr/0183 §5).
            "style.textStyles" => self.open_annotation_styles(crate::annotation_styles::Kind::Text),
            // Tablo stilleri (docs/adr/1127 §4).
            "style.tableStyles" => {
                return self.table_style_window_event(crate::table_style_window::Event::Open);
            }
            // Duvar stilleri (docs/adr/0339 §6).
            "arch.wallStyles" => self.open_wall_styles(),
            "arch.wallJoinCheck" => self.open_wall_joins(),
            // Kapı ve pencere tipleri (docs/adr/0772).
            "arch.openingTypes" => self.open_opening_types(),
            // Malzemeler (docs/adr/0785).
            "arch.materials" => self.open_materials(),
            // Oda modülleri (docs/adr/0775).
            "arch.modules" => self.open_room_modules(),
            // Tasarım seçenekleri (docs/adr/0788).
            "arch.options" => self.open_design_options(),
            "arch.moduleExcept" => self.room_module_selection_action(false),
            "arch.moduleRestore" => self.room_module_selection_action(true),
            "arch.moduleAddMembers" => self.room_module_add_selection(None),
            "arch.moduleRemoveMembers" => self.room_module_remove_selection(None),
            "arch.openingSameType" => self.select_same_opening_type(),
            "arch.openingDetach" => self.opening_type_action(kentos_commands::OpeningsOperation::Detach),
            "arch.openingPush" => {
                self.opening_type_action(kentos_commands::OpeningsOperation::Push { fields: Vec::new() });
            }
            "arch.openingReset" => {
                self.opening_type_action(kentos_commands::OpeningsOperation::Reset { fields: Vec::new() });
            }
            "arch.tagsCheck" => self.open_tags_check(),
            "arch.viewRefs" => self.open_view_refs(),
            "arch.sheetSet" => self.open_sheet_set(),
            "arch.navigator" => return self.show_navigator(),
            "xref.manager" => self.open_xrefs(),
            "xref.attach" => return self.start_xref_attach(),
            "arch.template" => self.apply_arch_template(),
            id if id.starts_with("arch.phase.") => self.set_phase_view(&id["arch.phase.".len()..]),
            // Üst kat, Alt kat (docs/adr/0720 §5; Archicad's Go Up/Down a Story).
            "view.levels.up" => self.step_level(true),
            "view.levels.down" => self.step_level(false),
            // Kat görünümü (docs/adr/0715).
            id if id.starts_with("view.levels.") => {
                if let Some(view) = crate::active_level::level_view_named(&id["view.levels.".len()..]) {
                    self.choose_level_view(view);
                }
            }
            // Metraj and Mahalleri birleştir (docs/adr/0342 §5).
            "arch.quantities" => self.open_quantities(),
            // Revizyon listesi (docs/adr/0721 §4).
            "arch.revisionList" => self.place_revision_list(),
            // Emsal ve TAKS tablosu (docs/adr/8030).
            "arch.zoningTable" => self.place_zoning_table(),
            // Model kalite kontrol formu (EK-9, docs/adr/8047 §10).
            "arch.qualityForm" => self.place_quality_form(),
            // Otopark and Sığınak hesabı (docs/adr/8036).
            "arch.parkingTable" => {
                self.place_provision_table(kentos_commands::TableSource::Parking {})
            }
            // Ruhsat takımı (docs/adr/8037): the particulars, the set and its publication.
            "arch.permitSet" => return self.permit_set_open(),
            "arch.shelterTable" => {
                self.place_provision_table(kentos_commands::TableSource::Shelter {})
            }
            "arch.spaceMerge" => self.space_merge(),
            "style.dimensionStyles" => {
                self.open_annotation_styles(crate::annotation_styles::Kind::Dimension)
            }
            // Şablonla çiz without a template: the Şablonlar panel, to choose one (templates_panel.rs).
            "template.draw" | "template.panel" | "template.apply" => {
                return self.show_templates_panel();
            }
            // Şablon düzenleyici, new or from the selection (template_editor.rs, docs/adr/0176 §4).
            "template.new" => return self.open_template_editor(None, None),
            "template.fromSelection" => return self.template_from_selection(),
            "style.legend" => self.open_legend(),
            "style.svgEditor" => self.open_svg_editor(crate::style::svgedit::Opening {
                id: None,
                path: None,
                after: crate::style::svgedit::After::Nothing,
            }),
            "style.assign" => return self.pick_for_selection(),
            "style.clearSymbol" => {
                let said = self.assign_symbol(None);
                self.output(said);
            }
            "view.symbols.plot" | "view.symbols.screen" => self.choose_symbol_size(id),
            // Selecting (docs/adr/0029): the pointer selects while no command runs.
            "tool.select" => self.leave_tool(),
            "edit.deselect" => self.selection.clear(),
            "edit.selectAll" => self.select_all(),
            "edit.invertSelection" => self.invert_selection(),
            // Seçim ekleri (docs/adr/0187, selection_commands.rs).
            "edit.previousSelection" => self.previous_selection(),
            "edit.cycleSelection" => self.cycle_selection(),
            "edit.selectFilter" => self.toggle_select_filter(),
            // Otomatik seçim (layer_types.rs, docs/adr/0622 §5).
            "edit.autoSelect" => self.toggle_auto_select(),
            "edit.selectFilterAll" => self.select_all_kinds(true),
            "edit.selectFilterNone" => self.select_all_kinds(false),
            id if id.starts_with("edit.selectFilter.") => {
                self.toggle_select_kind(&id["edit.selectFilter.".len()..]);
            }
            "view.ribbonCollapse" => {
                self.ribbon_collapsed = !self.ribbon_collapsed;
                self.ribbon_peek = false;
            }
            // Şerit harf ipuçları (F6; Alt tapped alone too, ribbon_keys.rs).
            "view.keyTips" => return self.toggle_key_tips(),
            // The bottom panel (bottom.rs): F2, and the coordinate list.
            "view.bottomPanel" => self.toggle_bottom(),
            "view.coords" => self.show_bottom(crate::bottom::BottomTab::Coords),
            // Nokta editörü: the bottom panel's Noktalar tab (points/, docs/adr/0153).
            "point.editor" => self.show_bottom(crate::bottom::BottomTab::Points),
            // Öznitelik tablosu: the bottom panel's Tablo tab (features/, docs/adr/0199 §4).
            "data.featureTable" => self.open_feature_table(),
            // Alan istatistikleri and Grafik: the bottom panel's İstatistik and Grafik tabs (docs/adr/0227 §8).
            "data.statistics" => self.open_statistics(None),
            "data.chart" => self.open_chart(None, None, None),
            id if id.starts_with("chart.") => {
                if let Some(k) = crate::charts::KINDS.iter().find(|k| k.command == id) {
                    self.open_chart(None, Some(k.kind), None);
                }
            }
            // Veri kaynakları: the dock's Kaynaklar tab (sources/, docs/adr/0199 §7).
            "data.sources" => self.show_sources(),
            // Alanlar: the active layer's fields (layer_fields.rs; the layer tree's menu gives its layer).
            "layer.fields" => {
                let layer = self
                    .document
                    .as_ref()
                    .map(|d| d.model.layers().active().to_owned());
                if let Some(layer) = layer {
                    return self.open_layer_fields(&layer);
                }
            }
            // Veride ara: the bottom panel's Arama tab, its box taking the keyboard (search/, docs/adr/0178).
            "data.search" => return self.open_data_search(),
            "data.unmark" => {
                self.address_unmark();
                self.gis.sources.extent_mark = None;
                self.map.parcel_graph = None;
                self.gis.impact_reach = None;
                self.map.zoning_shown_marks = None;
                self.map.pipes_shown_marks = None;
                self.map.baseline_shown = None;
                return self.data_event(crate::search::Event::Unmark);
            }
            // Topoloji kuralları (topology/, docs/adr/0202 §5, §6).
            "topology.check" => return self.open_topology_check(),
            // Mimari denetim (arch_audit.rs, docs/adr/0703).
            "arch.audit" => return self.open_arch_audit(),
            "cad.inspect" => return self.open_cad_inspection(),
            "cad.flatten" => return self.open_flatten(),
            "map.featuresCode" => return self.open_features_code(),
            "survey.baselines" => return self.open_baselines(),
            "tool.grade" => return self.open_grade(),
            "topology.cleanReport" => return self.open_clean_report(),
            "parcels.check" => return self.open_parcels_check(),
            "pipes.check" => return self.open_pipes_check(),
            "file.exchangeCheck" => return self.open_exchange_check(),
            "asbuilt.check" => return self.open_asbuilt(),
            "map.survey.adjustment" => return self.open_survey_network(),
            "map.featuresReconcile" => return self.open_features_reconcile(),
            "zoning.check" => return self.open_zoning(),
            "scenario.impact" => return self.open_impact(),
            "readjustment.distribute" => return self.open_readjustment(),
            "raster.accuracy" => return self.open_ortho(),
            "raster.compare" => return self.open_ortho_compare(),
            "raster.seamlines" => return self.open_ortho_seamlines(),
            "pointcloud.overlap" => return self.open_overlap(),
            "pointcloud.register" => return self.open_register(),
            "pointcloud.network" => return self.open_scan_network(),
            "pointcloud.place" => return self.open_cloud_place(),
            "map.catalogReport" => return self.open_catalog_report(),
            "terrain.sheetContours" => return self.open_sheet_contours(),
            "terrain.stockpile" => return self.open_stockpile(),
            // Harita işi on a contextual tab (map_work_panel.rs, docs/adr/1201 §3).
            "map.workSource" => {
                self.map_work_show_source();
                return Task::none();
            }
            "map.workRelease" => {
                self.map_work_release();
                return Task::none();
            }
            "topology.rules" => return self.open_topology_rules(),
            // Ağlar (networks/, docs/adr/0209 §10).
            "network.manage" => return self.open_networks(),
            // Adres arama (address/, docs/adr/0229 §13).
            "address.search" => return self.open_address_search(),
            "address.reverse" => return self.start_tool(kentos_interaction::address_reverse::ID),
            "address.data" => return self.open_address_data(),
            // GNSS arşivi (map_survey_panel.rs, docs/adr/1340 §6).
            "map.survey" => {
                self.show_map_survey();
                return Task::none();
            }
            "map.survey.import" => return self.map_survey_event(crate::map_survey_panel::Event::Import),
            "map.survey.select" => {
                self.select_accepted_points();
                return Task::none();
            }
            "address.compare" => return self.open_address_compare(),
            // docs/adr/0583: a layer's source system and the map's.
            "layer.crs" => return self.layer_crs_run(false),
            "layer.reproject" => return self.layer_crs_run(true),
            "view.mapCrs" => self.map_crs_run(),
            "address.providers" => return self.open_address_providers(),
            // Gevşeterek düzenle: the next write relaxes the constraints (docs/adr/0337 §1); again, it does not.
            "constraint.relaxNext" => self.relax_next(),
            // Parametreler and the constraints' marks (docs/adr/0338 §1).
            "constraint.parameters" => self.open_parameters(),
            "constraint.show" => self.constraint_view.toggle(),
            // The user coordinate system (ucs.rs, docs/adr/0261).
            "ucs.world" => self.ucs_world(),
            "ucs.previous" => self.ucs_previous(),
            // Ölçekli açıklamalar (annotative.rs, docs/adr/0323 §4).
            "annotation.make" => {
                self.annotative_selection(kentos_commands::AnnotativeOperation::Make)
            }
            "annotation.unmake" => {
                self.annotative_selection(kentos_commands::AnnotativeOperation::Unmake)
            }
            "annotation.reset" => {
                self.annotative_selection(kentos_commands::AnnotativeOperation::Reset)
            }
            // Sırala (draw_order.rs, docs/adr/0260 §5).
            "edit.bringToFront" => self.order_selection(kentos_commands::OrderTo::Front),
            "edit.sendToBack" => self.order_selection(kentos_commands::OrderTo::Back),
            "edit.textsToFront" => self.order_kinds(
                &crate::draw_order::WORDS,
                kentos_commands::OrderTo::Front,
                "Yazıları öne getir",
            ),
            "edit.hatchesToBack" => self.order_kinds(
                &[kentos_commands::SelectKind::Hatch],
                kentos_commands::OrderTo::Back,
                "Taramaları arkaya gönder",
            ),
            // Zaman and Senaryo (temporal/, docs/adr/0210 §10).
            "time.slider" => self.time_toggle(),
            "time.layer" => self.open_time_layer(None),
            "time.sensors" => self.open_sensors(),
            "time.compare" => self.open_time_compare(),
            "scenario.create" => self.open_scenario_create(),
            "scenario.show" => {
                if let Some(id) = self.scenario_at_hand() {
                    self.show_scenario(&id);
                }
            }
            "scenario.base" => self.show_base(),
            "scenario.compare" => self.open_scenario_compare(None),
            "scenario.apply" => self.ask_scenario_apply(None),
            crate::catalog::PYTHON_CONSOLE => self.toggle_python(),
            // The navigation commands keep the view they leave (navigation.rs, docs/adr/0141).
            "view.zoomIn" => self.navigating(Self::zoom_in),
            "view.zoomOut" => self.navigating(Self::zoom_out),
            "view.zoomExtents" => self.navigating(Self::zoom_extents),
            // Görünümü böl (split_view.rs, docs/adr/0285 §3).
            "view.split" => self.toggle_split(),
            // Görünümleri bağla, Kaydırmalı karşılaştır (split_view.rs, docs/adr/0523).
            "view.splitLink" => self.toggle_split_link(),
            "view.splitAdd" => self.add_view(),
            "view.swipe" => self.toggle_swipe(),
            "view.zoomSelection" => self.navigating(Self::zoom_selection),
            "commandline.focus" => return operation::focus(COMMAND_INPUT),
            "help.about" => self.dialog = Some(Dialog::About),
            "help.shortcuts" => return self.open_shortcuts(),
            // An edit, not an undo step (web: LayerStore.showAll).
            "layer.showAll" => match &self.document {
                Some(_) => self.show_all_layers(),
                None => self.output("Açık çizim yok."),
            },
            // Kopyasını oluştur and Katmanları birleştir (docs/adr/0177 §3): the active layer's;
            // Katmanlar's menu names its layer (layering.rs).
            "layer.duplicate" => self.duplicate_layer(None),
            "layer.merge" => self.open_layer_merge(None),
            // Katman durumları, Kullanılmayanları temizle and Katman listesi (docs/adr/0177 §4–§6).
            "layer.states" => self.open_layer_states(),
            "layer.stateSave" => {
                self.quick_save_layer_state();
            }
            "layer.purge" => self.open_layer_purge(),
            "layer.list" => self.open_layer_list(),
            "text.fonts" => self.open_font_report(),
            // Alanlar and the text commands (text_tools.rs, docs/adr/0325 §4).
            id if self.text_tools_run(id) => {}
            // Çok kollu kılavuz (leader_tools.rs, docs/adr/0326 §5).
            id if self.leader_tools_run(id) => {}
            // Katman süzgeci (layer_filters.rs, docs/adr/0211 §4): the active layer's; Katmanlar's menu names its
            // layer (layering.rs).
            "layer.filter" => return self.open_layer_filter(None),
            "layer.filterFromSelection" => self.filter_from_selection(None),
            "layer.filterClear" => self.clear_layer_filter(None),
            // Nesne ekleri (attachments.rs, layer_info.rs; docs/adr/0225 §2, §5).
            "layer.info" => return self.open_layer_info(None),
            // Veri kümesi bilgisi (dataset_window.rs, docs/adr/0633).
            "layer.dataset" => return self.open_dataset_info(None),
            "attachment.add" | "attachment.addLink" => return self.attachment_command(id),
            // Etiketler (labelling/, docs/adr/0212 §4): the active layer's; Katmanlar's menu names its layer.
            "layer.labels" => self.open_labelling(None),
            // Veri karşılaştır (docs/adr/0179).
            "data.compare" => self.open_data_compare(),
            // Harita servisleri (services/, docs/adr/0208 §14).
            "basemap.remove" => self.remove_basemap(),
            "service.connections" => self.open_connections(None),
            // Bağlı katmanlar (bound.rs, docs/adr/0503): the layer selected in the tree, else the active one.
            "source.write" => return self.bound_event(crate::bound::Event::Write(None)),
            "source.writeTogether" => {
                return self.bound_event(crate::bound::Event::WriteTogether(None));
            }
            "source.read" => return self.bound_event(crate::bound::Event::Read(None)),
            "source.reconcile" => return self.bound_event(crate::bound::Event::Reconcile(None)),
            "source.status" => return self.bound_event(crate::bound::Event::Status),
            "source.unlink" => return self.bound_event(crate::bound::Event::Unlink(None)),
            "source.relink" => return self.bound_event(crate::bound::Event::Relink(None)),
            "service.add" => self.open_service_window(None),
            "service.feed" => self.open_feed_window(),
            "service.info" => self.start_service_info(),
            // Raster ekle and Raster stili (rasters/, docs/adr/0204 §8); Raster oturt is a Hesap window.
            "raster.add" => return self.raster_add_command(),
            // Sanal mozaik (rasters/mosaic.rs, docs/adr/0517 §5).
            "raster.mosaic" => return self.raster_mosaic_command(),
            // STAC'ta ara (rasters/stac.rs, docs/adr/0528 §7).
            "raster.stac" => return self.raster_stac_command(),
            "raster.style" => return self.raster_look_command(),
            // Mesh ekle (rasters/multidim.rs, docs/adr/0243 §11).
            "mesh.add" => return self.mesh_add_command(),
            // Nokta bulutu (pointclouds/, docs/adr/0207 §9).
            "pointcloud.add" => return self.cloud_add_command(),
            "pointcloud.style" => return self.cloud_look_command(),
            "pointcloud.query" => return self.start_tool(kentos_interaction::cloud_query::ID),
            "pointcloud.vpcSave" => return self.cloud_vpc_command(),
            // Kayıtlı ölçüler (docs/adr/0180): the check's window, and the selection's recorded values from the drawing.
            "cogo.check" => self.open_cogo_check(),
            "cogo.update" => {
                self.cogo_update();
            }
            // Genel bakış and Büyüteç over the drawing (docs/adr/0181).
            "view.overview" => self.toggle_overview(),
            "view.magnifier" => self.toggle_magnifier(),
            // Yalıtımı kaldır (docs/adr/0177 §1): what Katmanı yalıt hid, shown again.
            "view.isolateObjects" => self.object_isolation(false),
            "view.endObjectIsolation" => self.object_isolation(true),
            "layer.unisolate" => match &mut self.document {
                Some(doc) => {
                    let (level, text) = kentos_interaction::layer_tools::unisolate(&mut doc.model);
                    self.say(level, text);
                }
                None => self.output("Açık çizim yok."),
            },
            // Bütün katmanları çöz and Katmanları çevir (docs/adr/0311).
            "layer.thawAll" => self.thaw_all_layers(),
            "layer.translate" => self.open_layer_translate(),
            // Standardı denetle and Standardı uygula: one window (docs/adr/0312 §7).
            "standards.check" | "standards.apply" => self.open_drawing_standard(),
            "review.notes" => self.open_review_notes(),
            "profile.window" => self.open_data_profile(),
            "network.subnetworks" => self.open_subnetworks(),
            // Olay bağları (event_links_window.rs, docs/adr/0569 §5).
            "linearRef.eventLinks" => self.open_event_links(),
            // Arazi yüzeyleri and Yüzey oluştur (terrain_window.rs, plan/road.md Y2.4).
            "terrain.surfaces" => self.open_terrains(),
            "terrain.new" => self.new_terrain(),
            // Güzergâh and Yeni aks (alignment_window.rs, plan/road.md Y3).
            "alignment.editor" => self.open_alignments(),
            "alignment.new" => self.new_alignment(),
            // Boykesit and Boykesit oluştur (profile_window.rs, plan/road.md Y4).
            "profile.editor" => self.open_profiles(),
            "profile.new" => self.new_profile(),
            // Enkesit, Kesit takımı oluştur and Tip kesit oluştur (section_window.rs, plan/road.md Y5).
            "section.editor" => self.open_sections(),
            "section.new" => self.new_section_set(),
            "section.template" => self.new_template(),
            // Dever and Dever oluştur (superelevation_window.rs, plan/road.md Y6).
            "superelevation.editor" => self.open_superelevations(),
            "superelevation.new" => self.new_superelevation(),
            // Koridor, Koridor oluştur and Şevli plan (corridor_window.rs, plan/road.md Y7).
            "corridor.editor" => self.open_corridors(),
            "corridor.new" => self.new_corridor(),
            "corridor.slopePlan" => self.corridor_slope_plan(),
            // Kübaj, Kübaj hesabı oluştur and Kübaj cetveli (volume_window.rs, plan/road.md Y8).
            "volume.editor" => self.open_volumes(),
            "volume.new" => self.new_volume(),
            "volume.export" => self.volume_sheet(),
            // Brükner and Brükner oluştur (haul_window.rs, plan/road.md Y9).
            "haul.editor" => self.open_hauls(),
            "haul.new" => self.new_haul(),
            "haul.plan" => self.open_haul_plan(),
            // Aplikasyon cetveli (stakeout_window.rs, plan/road.md Y10).
            "stakeout.table" => self.open_stakeout(),
            "road.sheets" => self.open_road_sheets(),
            "road.views" => self.open_road_views(),
            "alignment.fromLine" => self.open_road_alignment_fit(),
            "profile.fromLine" => self.open_road_profile_fit(),
            "guardrail.editor" => self.open_guardrails(),
            "culvert.editor" => self.open_culverts(),
            "retainingWall.editor" => self.open_retaining_walls(),
            "platform.editor" => self.open_platforms(),
            "terrain.cloud" => self.open_terrain_cloud(),
            "drainage.editor" => self.open_drainage(),
            "swept.editor" => self.open_swept(),
            "terrain.analysis" => self.open_surface_analysis(),
            "profile.sight" => self.open_sight(),
            "profile.propose" => self.open_propose(),
            "alignment.check" => self.open_design_check(),
            "corridor.water" => self.open_platform_water(),
            "payItems.editor" => self.open_pay(),
            "route.finder" => self.open_route(),
            "junction.editor" => self.open_junctions(),
            "scene3d.view" => self.open_scene3d(),
            // LandXML ver (road_exchange.rs, plan/road.md Y10.2).
            "landxml.export" => return self.export_landxml(),
            "landxml.machine" => return self.export_landxml_machine(),
            "landxml.importAlignments" => return self.import_landxml_alignments(),
            "ifc.export" => return self.export_ifc(),
            "gltf.export" => return self.export_gltf(),
            // IFC dışa aktar (bina) (arch_ifc.rs, docs/adr/8047).
            "arch.ifcExport" => return self.export_ifc_building(),
            // Bina programı (program_window.rs, docs/adr/0773).
            "arch.program" => self.open_program(),
            // Çizgi tipleri (docs/adr/0313 §7).
            "lineTypes.manage" => self.open_line_types(),
            "hatchPatterns.manage" => self.open_hatch_patterns(),
            // Edits, not undo steps (layering.rs).
            "layer.new" => self.new_layer(),
            // A typed layer (layer_types.rs, docs/adr/0622 §5).
            id if crate::layer_types::NEW_TYPED.iter().any(|(c, _)| *c == id) => {
                if let Some((_, g)) = crate::layer_types::NEW_TYPED.iter().find(|(c, _)| *c == id) {
                    self.new_typed_layer(*g);
                }
            }
            "layer.newGroup" => self.new_group(),
            // The blocks (blocks_panel.rs, docs/adr/0144).
            "block.panel" => self.show_blocks_panel(),
            "block.find" => self.open_block_find(),
            "array.find" => self.open_array_find(),
            "block.purge" => self.purge_blocks(),
            // Blok düzenleyici (block_editor.rs, docs/adr/0316 §1).
            "block.edit" => self.block_editor_from_selection(),
            // Bloğu dosyaya yaz and Başka çizimden yeniden tanımla (blocks_files.rs, docs/adr/0316 §3).
            "block.writeFile" => {
                if let Some(id) = self.block_at_hand() {
                    return self.blocks_write_file(id);
                }
            }
            "block.redefineFromFile" => {
                if let Some(id) = self.block_at_hand() {
                    return self.blocks_redefine_from(id);
                }
            }
            // Blok çizelgesi, Öznitelikleri eşitle and Blok kartı (docs/adr/0317).
            "block.schedule" => self.open_block_schedule(None),
            // KentOS's own blocks (blocks_files.rs, docs/adr/0318 §4).
            "block.library" => self.blocks_builtin(),
            // Sabit bloğa çevir (docs/adr/0319 §4).
            "block.freeze" => self.blocks_freeze(),
            "dimension.space" => self.dimensions_edit(|uids| {
                let mut uids = uids.into_iter();
                let base = uids.next().unwrap_or_default();
                kentos_commands::DimensionEdit::Space {
                    base,
                    uids: uids.collect(),
                    spacing: None,
                }
            }),
            "dimension.break" => {
                self.dimensions_edit(|uids| kentos_commands::DimensionEdit::Break {
                    uids,
                    objects: None,
                })
            }
            "dimension.unbreak" => {
                self.dimensions_edit(|uids| kentos_commands::DimensionEdit::Unbreak { uids })
            }
            "block.sync" => {
                if let Some(id) = self.block_at_hand() {
                    self.blocks_sync(id, false);
                }
            }
            "block.card" => {
                if let Some(id) = self.block_at_hand() {
                    self.open_block_card(id);
                }
            }
            // Blokları dışa aktar and Blokları al (blocks_files.rs, docs/adr/0314 §3).
            "block.export" => return self.blocks_export(None),
            // Değişikliğin etkisi for the active layer (style_impact.rs, docs/adr/0314 §4).
            "layer.impact" => {
                if let Some(active) = self
                    .document
                    .as_ref()
                    .map(|d| d.model.layers().active().to_owned())
                {
                    return self.style_impact_event(crate::style_impact::Event::Open(
                        kentos_commands::ImpactTarget::Layer,
                        active,
                    ));
                }
            }
            "block.import" => return self.blocks_import(),
            "block.attributes" => self.open_selected_block_attributes(),
            // Bul ve değiştir (find_replace.rs, docs/adr/0145 §6).
            "text.findReplace" => self.open_find_replace(),
            "edit.undo" => self.undo(),
            "edit.redo" => self.step_history(false),
            "tool.confirm" => return self.confirm(),
            // Esc with nothing to cancel leaves full screen (view_commands.rs).
            "tool.cancel" if self.fullscreen && !self.cancellable() => {
                return self.toggle_fullscreen();
            }
            "tool.cancel" => self.cancel(),
            "tool.repeat" => return self.repeat_last(),
            _ => self.error(format!(
                "{id}: masaüstü işleyicisi eksik (catalog::PORTED ile karşılaştırın)"
            )),
        }
        Task::none()
    }

    /// Undoes or redoes the drawing's last step, saying which (the web's
    /// “Geri alındı: Ekle”); with nothing to undo or redo, says that.
    pub(crate) fn step_history(&mut self, undo: bool) {
        if let Some(problem) = self
            .document
            .as_ref()
            .and_then(|d| d.model.sheet_history_problem(!undo))
        {
            self.warn(problem);
            return;
        }
        let Some(doc) = &mut self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        // A step already written to a bound layer's source: the undo is the drawing's until the next write
        // (docs/adr/0598 §4).
        let written = if undo && !doc.model.settings().sources.is_empty() {
            doc.model
                .undo_report()
                .map(|r| kentos_native_application::sources::undoing_written(&doc.model, &r))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let step = if undo {
            doc.model.undo()
        } else {
            doc.model.redo()
        };
        if step.is_some() {
            self.echo_history(undo);
        }
        self.output(match (step, undo) {
            (Some(label), true) if !written.is_empty() => format!(
                "Geri alındı: {label}. Bu adım {} kaynağına yazılmıştı; kaynakta hâlâ duruyor. Kaynağa yaz geri almayı \
                 kaynağa da yazar; o arada kaynakta değişen satır varsa yazma reddedilir.",
                written.iter().map(|n| format!("“{n}”")).collect::<Vec<_>>().join(", ")
            ),
            (Some(label), true) => format!("Geri alındı: {label}"),
            (Some(label), false) => format!("Yinelendi: {label}"),
            (None, true) => "Geri alınacak değişiklik yok.".to_owned(),
            (None, false) => "Yinelenecek değişiklik yok.".to_owned(),
        });
    }

    /// Whether a ported command can run now: undo with a step to take, in
    /// the drawing or in the running command's draft, redo with one to take
    /// (web: `isEnabled`, `watch: [doc.canUndo, tools.prompt]`). Buttons of
    /// commands that cannot run are drawn dimmed.
    /// A command's on or off state where it has one (the ribbon and its menus show it).
    pub fn checked(&self, id: &str) -> Option<bool> {
        Some(match id {
            "draft.ortho" => self.draft.ortho,
            "view.split" => self.split.is_some(),
            "view.splitLink" => self
                .split
                .as_ref()
                .is_some_and(|s| s.link != kentos_geometry_core::tools::view_link::Link::Off),
            "view.swipe" => self.split.as_ref().is_some_and(|s| s.swipe.is_some()),
            "draft.rightAngle" => self.draft.right_angle,
            "draft.polar" => self.draft.polar.is_some(),
            "draft.snap" => self.draft.snap,
            id if crate::snap_menu::setting(id).is_some() => {
                crate::snap_menu::setting(id).is_some_and(|key| self.settings.bool(key))
            }
            "draft.tracking" => self.draft.tracking,
            "draft.topology" => self.draft.topology,
            "draft.topologyPoints" => self.draft.topology_points,
            "draft.topologyStopOnLocked" => self.draft.topology_stop_on_locked,
            "draft.overlap" => self.draft.overlap != kentos_interaction::Overlap::Allow,
            "edit.selectFilter" => self.draft.select_kinds.is_some(),
            "edit.autoSelect" => self.auto_select_on(),
            id if id.starts_with("edit.selectFilter.") => {
                self.select_kind_checked(&id["edit.selectFilter.".len()..])
            }
            id if id.starts_with("arch.phase.") => {
                self.phase_view().name() == &id["arch.phase.".len()..]
            }
            // Üst kat and Alt kat move, they are not checked (docs/adr/0720 §5).
            "view.levels.up" | "view.levels.down" => return None,
            id if id.starts_with("view.levels.") => {
                crate::active_level::level_view_named(&id["view.levels.".len()..]) == Some(self.level_view)
            }
            "draft.lock.keep" => self.locks.keep,
            "draft.lock.construction" => self.session.construction(),
            "draft.overlap.allow" => self.draft.overlap == kentos_interaction::Overlap::Allow,
            "draft.overlap.layer" => self.draft.overlap == kentos_interaction::Overlap::Layer,
            "draft.overlap.layers" => self.draft.overlap == kentos_interaction::Overlap::Layers,
            "draft.grid" => self.settings.bool("drafting.grid"),
            "view.lineWeights" => self.settings.bool("graphics.lineWeights"),
            "view.colorMode.color" => self.settings.text("graphics.colorMode") == "color",
            "view.colorMode.mono" => self.settings.text("graphics.colorMode") == "mono",
            "view.colorMode.gray" => self.settings.text("graphics.colorMode") == "gray",
            // The rule the drawing is drawn by: Türe göre's too (docs/adr/0626 §6).
            "view.annotationSize.legible" => {
                matches!(self.label_size(), kentos_interaction::spatial::LabelSize::Legible)
            }
            "view.annotationSize.true" => {
                matches!(self.label_size(), kentos_interaction::spatial::LabelSize::Print)
            }
            "view.annotationSize.screen" => matches!(
                self.label_size(),
                kentos_interaction::spatial::LabelSize::Screen { .. }
            ),
            "view.fills" => self.settings.bool("graphics.fills"),
            "view.areaEdges" => self.settings.bool("graphics.areaEdges"),
            "view.pointNames" => self.settings.bool("graphics.pointNames"),
            "view.pointCodes" => self.settings.bool("graphics.pointCodes"),
            "view.pinnedLabels" => self.settings.bool("graphics.pinnedLabels"),
            "view.unplacedLabels" => self.settings.bool("graphics.unplacedLabels"),
            "view.transparency" => self.settings.bool("graphics.transparency"),
            "view.symbols.plot" => self.settings.text("graphics.symbolSize") != "screen",
            "view.symbols.screen" => self.settings.text("graphics.symbolSize") == "screen",
            "view.theme.dark" => self.mode == Mode::Dark,
            "view.theme.light" => self.mode == Mode::Light,
            "view.bottomPanel" => self.command_expanded,
            crate::catalog::PYTHON_CONSOLE => {
                self.command_expanded && self.bottom_tab == crate::bottom::BottomTab::Python
            }
            "view.rightPanel" => self.right_panel_shown(),
            "view.overview" => self.overview_shown(),
            "view.magnifier" => self.magnifier_shown(),
            "view.fullscreen" => self.fullscreen,
            // Zaman sürgüsü and Mevcut durum (docs/adr/0210 §10).
            "time.slider" => self.gis.time.slider.open,
            "scenario.base" => {
                self.has_scenarios()
                    && self.document.as_ref().is_some_and(|d| {
                        crate::temporal::shown_scenario(d.model.layers())
                            == crate::temporal::Shown::Base
                    })
            }
            id if id.starts_with("workspace.") => {
                crate::catalog::mode_command(self.work_mode()) == id
            }
            _ => return None,
        })
    }

    pub fn available(&self, id: &str) -> bool {
        // A command a module's program does not offer is off wherever it shows (docs/adr/0802).
        if !self.offers(id) {
            return false;
        }
        let doc = self.document.as_ref().map(|doc| &doc.model);
        match id {
            "edit.undo" => {
                doc.is_some_and(kentos_domain::Document::can_undo)
                    || (self.session.is_running() && self.session.point_count() > 0)
            }
            "edit.redo" => doc.is_some_and(kentos_domain::Document::can_redo),
            // The locks work while a point is expected after another (docs/adr/0166 §6).
            "draft.lock.length"
            | "draft.lock.angle"
            | "draft.lock.east"
            | "draft.lock.north"
            | "draft.lock.parallel"
            | "draft.lock.perpendicular"
            | "draft.lock.keep" => self.session.lock_reference().is_some(),
            "draft.lock.reference" | "draft.lock.construction" | "draft.trackPoint" => {
                self.session.takes_points()
            }
            "draft.lock.deflection" => {
                self.session.lock_reference().is_some() && self.session.travel().is_some()
            }
            "draft.lock.clear" => self.session.lock_reference().is_some() && self.locks.any(),
            "view.isolateObjects" => doc.is_some() && !self.selection.is_empty() && !self.session.is_running(),
            "view.endObjectIsolation" => doc.is_some_and(kentos_domain::Document::objects_isolated) && !self.session.is_running(),
            "edit.deselect" | "view.zoomSelection" => !self.selection.is_empty(),
            "map.workSource" | "map.workRelease" => self.map_work_available(id),
            // An attachment goes on one object (attachments.rs, docs/adr/0225 §2), as the web's.
            "attachment.add" | "attachment.addLink" => doc.is_some() && self.selection.len() == 1,
            "edit.previousSelection" => !self.previous_selection_ids().is_empty(),
            "edit.cycleSelection" => self.selection.cycle().is_some(),
            // Only while a place is marked (search/, docs/adr/0178 §6).
            "data.unmark" => {
                self.data_mark().is_some()
                    || self.extent_mark().is_some()
                    || self.parcel_graph_marks().is_some()
                    || self.impact_reach_marks().is_some()
                    || self.zoning_marks().is_some()
                    || self.pipes_marks().is_some()
                    || self.baseline_marks().is_some()
            }
            // Only where there is a view to go to (docs/adr/0141).
            "view.previous" => self.view_history.can_back(),
            "view.next" => self.view_history.can_forward(),
            "view.extentCheck" => doc.is_some(),
            // Only while a ready basemap is at the bottom (docs/adr/0208 §14).
            "basemap.remove" => doc.is_some_and(|d| {
                crate::services::basemaps::bottom_basemap(d.layers().nodes()).is_some()
            }),
            "service.connections" | "service.add" | "service.feed" | "service.info" => {
                doc.is_some()
            }
            // The scenarios' commands while the project has one (docs/adr/0210 §10).
            "scenario.show" | "scenario.base" | "scenario.compare" | "scenario.apply" => {
                self.has_scenarios()
            }
            id if crate::clipboard::COMMANDS.contains(&id) => self.clipboard_available(id),
            "server.check" => !self.server_checking,
            id if id.starts_with("cloud.") => self.cloud_available(id),
            "layer.new" | "layer.newGroup" => self.tree_locked().is_none(),
            id if crate::layer_types::NEW_TYPED.iter().any(|(c, _)| *c == id) => {
                self.tree_locked().is_none()
            }
            "edit.autoSelect" => doc.is_some(),
            // A drawing tool that writes another kind than the typed active layer holds (docs/adr/0622 §5).
            id if self.layer_misfit(id).is_some() => false,
            // A selection tool that takes none of the selected kinds (UR-007).
            id if self.selection_misfit(id).is_some() => false,
            "block.purge" | "block.export" => doc.is_some_and(|d| !d.blocks().is_empty()),
            "block.find" | "array.find" => doc.is_some() && !self.session.is_running(),
            "block.import" | "layer.impact" => doc.is_some(),
            "block.attributes" => doc.is_some_and(|d| {
                kentos_interaction::blocks::selected_block(d, &self.selection).is_some()
            }),
            // Katman stili opens for the active layer: not for a group.
            "style.layerStyle" => doc.is_some_and(|d| {
                d.layers()
                    .get(d.layers().active())
                    .is_some_and(|n| n.kind == kentos_commands::LayerNodeType::Layer)
            }),
            // Symbols for the selected objects; taking them away when one has its own.
            "style.assign" => doc.is_some() && !self.selection.is_empty(),
            "template.fromSelection" => doc.is_some() && !self.selection.is_empty(),
            "style.legend" => doc.is_some(),
            "style.clearSymbol" => doc.is_some_and(|d| {
                self.selection
                    .ids()
                    .iter()
                    .any(|&s| d.get(s).is_some_and(|e| e.base().symbol.is_some()))
            }),
            // A project keeps its type (docs/adr/0622 §1): only one not asked its type yet takes one.
            id if id.starts_with("workspace.") => {
                doc.is_some_and(|d| d.settings().project_type().is_none())
            }
            _ => true,
        }
    }

    /// Why a command is off, when it says (the web's `whyDisabled`): its tip
    /// shows it.
    pub fn why_disabled(&self, id: &str) -> Option<String> {
        // Another module's command says which program has it (docs/adr/0802).
        let title = catalog().get(id).map_or(id, |c| c.title);
        if let Some(why) = self.not_offered(id, title) {
            return Some(why);
        }
        match id {
            "layer.new" | "layer.newGroup" => self.tree_locked().map(str::to_owned),
            id if crate::layer_types::NEW_TYPED.iter().any(|(c, _)| *c == id) => {
                self.tree_locked().map(str::to_owned)
            }
            id if id.starts_with("workspace.") && !self.available(id) => Some(
                "Proje türü projeyle başlar ve değişmez; CAD ile CBS arasında geçiş ileride dönüştürmeyle olacak."
                    .to_owned(),
            ),
            "scenario.show" | "scenario.base" | "scenario.compare" | "scenario.apply"
                if !self.has_scenarios() =>
            {
                Some("Projede senaryo yok; önce Senaryo oluştur ile bir senaryo yapın.".to_owned())
            }
            // The typed active layer does not hold what the tool draws (docs/adr/0622 §5); a selection tool
            // that takes none of the selected kinds (UR-007).
            id => self.layer_misfit(id).or_else(|| self.selection_misfit(id)),
        }
    }

    /// A name typed in the command line: an alias, the command's name or its
    /// title. A tool started from there gets the keyboard for the drawing.
    pub(crate) fn run_typed(&mut self, text: &str) -> Task<Message> {
        if text.is_empty() {
            return Task::none();
        }
        // The program's own names first (module_names.rs, docs/adr/8002): in KentOS Mimari, PENCERE is the window.
        if let Some(task) = self.run_module_name(text) {
            return task;
        }
        // Klasik CAD komut adları, when the preference is on (alias_profile.rs, docs/adr/0282):
        // the profile holds no name of KentOS's, which keep their meaning.
        if let Some(task) = self.run_profile_name(text) {
            return task;
        }
        // A tool's method by its own name (DOR: Ölçülendirme as Koordinat; docs/adr/0147 §7),
        // as the ribbon's menu starts it: the tool, then its option.
        if let Some(method) = catalog().method_by_alias(text)
            && let Some(option) = method.option
        {
            // A module's program runs only its own commands and methods (docs/adr/0802).
            if let Some(why) = self.method_not_offered(method.id, option, method.label) {
                self.error(why);
                return Task::none();
            }
            self.remember(text);
            let task = self.run_method(method.id, option, method.label);
            if self.session.is_running() {
                self.line_focused = false;
                return Task::batch([task, release_keyboard()]);
            }
            return task;
        }
        let folded = fold(text);
        // A full name first: an alias, a title (docs/adr/0281).
        let named = crate::typed_names::named(text, self.sheet_tab);
        let found = match &named {
            crate::typed_names::Named::One(command, false) => Some(*command),
            _ => None,
        };
        // A model of the user's, by its name (“Parsel ölçüleri”, “Parsel ölçüleri…”).
        let model = found.is_none().then(|| {
            self.user_models()
                .find(|m| {
                    fold(&m.label) == folded
                        || fold(&format!("{}…", m.label)) == folded
                        || text == format!("processing.model.{}", m.id)
                })
                .map(|m| m.id.clone())
        });
        if let Some(Some(id)) = model {
            self.remember(text);
            return self.processing_command(&format!("processing.model.{id}"));
        }
        // The start of one command's name runs it, said; a name or start several share is said with them.
        let found = match named {
            crate::typed_names::Named::One(command, true) => {
                self.output(format!("“{text}”: {}.", command.title));
                Some(command)
            }
            crate::typed_names::Named::Several(commands) => {
                self.error(format!(
                    "“{text}” birden çok komutu anlatıyor: {}. Komutun adını tam yazın.",
                    crate::typed_names::candidates(&commands)
                ));
                return Task::none();
            }
            _ => found,
        };
        match found {
            Some(command) => {
                // A module's program runs only its own commands (docs/adr/0802).
                if let Some(why) = self.not_offered(command.id, command.title) {
                    self.error(why);
                    return Task::none();
                }
                // As on the web (`CommandLine.run`): the line's ↑ brings it back; the
                // typed name is not said, a tool says its own as it starts.
                self.remember(text);
                let task = self.run(command.id);
                if command.id.starts_with("tool.") && self.session.is_running() {
                    self.line_focused = false;
                    return Task::batch([task, release_keyboard()]);
                }
                task
            }
            None => {
                self.error(format!(
                    "“{text}” adında bir komut yok. Tüm komutlar ve kısayollar için F1’e basın."
                ));
                Task::none()
            }
        }
    }
}
