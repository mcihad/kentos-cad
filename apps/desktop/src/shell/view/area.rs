//! The drawing area with the dynamic blocks' grips and Büyüteç's window, the docked panels' bodies and
//! the layer tree's rows.

use super::*;
use iced::widget::column;

impl App {
    /// The selected dynamic blocks' parameter grips (docs/adr/0319 §1), as many objects' as the other grips; and the
    /// selected rooms' edge grips (Odayı büyüt, docs/adr/8023), drawn as a linear parameter's pair of arrowheads square
    /// to the edge.
    pub(super) fn param_grip_marks(&self) -> Vec<kentos_geometry_core::block_dynamic::ParamGrip> {
        use kentos_geometry_core::block_dynamic::{GripKind, ParamGrip};
        if self.selection.len() > kentos_interaction::select::GRIP_LIMIT {
            return Vec::new();
        }
        let mut marks: Vec<ParamGrip> = self
            .spatial
            .param_grips(self.selection.ids())
            .into_iter()
            .map(|(_, g)| g)
            .collect();
        if let Some(doc) = self.document.as_ref() {
            marks.extend(
                kentos_interaction::room_grip::grips(
                    &doc.model,
                    &self.spatial,
                    self.selection.ids(),
                )
                .into_iter()
                .map(|g| ParamGrip {
                    name: kentos_interaction::room_grip::LABEL.to_owned(),
                    kind: GripKind::Linear,
                    at: g.at,
                    from: None,
                    along: Some(g.outward),
                }),
            );
        }
        marks
    }

    pub(super) fn drawing_area(&self) -> Element<'_, Message> {
        match &self.document {
            // The open drawing on KentOS's own wgpu pipeline (viewport.rs, docs/adr/0019),
            // the running tool's draft and the value field over it (preview.rs).
            Some(doc) => {
                let format = Format::of(doc.settings());
                // The selection box and the snap marker (docs/adr/0029) under the draft.
                // The selection's grips, as many objects' as the web shows (docs/adr/0068).
                let grips = if self.selection.len() <= kentos_interaction::select::GRIP_LIMIT {
                    self.spatial.grips(self.selection.ids())
                } else {
                    Vec::new()
                };
                let marks = Marks {
                    camera: self.viewport.camera,
                    snap: self.snap,
                    snap_note: self.snap_marker_note(),
                    select: self.session.select_box(),
                    grips,
                    params: self.param_grip_marks(),
                    hot: self.session.hot_grips(),
                    marked: self.marked_points(),
                    ways: self.vertex_ways(),
                    range: self.grade_marks().0.into_iter().map(|p| kentos_interaction::Vec2::new(p.x, p.y)).collect(),
                    found: self.address_mark_label().or_else(|| self.data_mark_label()),
                    extent: self.extent_mark(),
                    problem: self.topology_problem().or_else(|| self.gis.networks.mark.clone()),
                    graph: self.parcel_graph_marks(),
                    reach: self.impact_reach_marks(),
                    zoning: self.zoning_marks(),
                    pipes: self.pipes_marks(),
                    baseline: self.baseline_marks(),
                    broken: self.broken_marks(),
                    missing: self.missing_pictures(),
                    clips: self.clip_marks(),
                    bonds: self.bond_marks(),
                    constraints: self.constraint_marks(),
                    constraint_pointer: self.constraint_view.pointer,
                    tracking: self.tracking_marks(&format),
                    crosshair: self.crosshair_mark(),
                    locks: self.lock_marks(&format),
                    colors: mark_colors(self.canvas()),
                };
                let over = preview::layer(
                    &self.viewport.camera,
                    marks,
                    self.drawing_preview(&format),
                    self.field.as_ref().map(|f| preview::FieldView {
                        text: f.text.as_str(),
                        token: f.token,
                        at: f.at,
                        hint: match (&f.setting, f.mode) {
                            // A setting's field (docs/adr/8018): what it holds now, a choice's words.
                            (Some(s), _) if s.choices.is_empty() && s.value.trim().is_empty() => {
                                "Enter uygular · Tab sonraki · Esc kapatır".to_owned()
                            }
                            (Some(s), _) if s.choices.is_empty() => {
                                format!("Şimdi {} · Enter uygular · Tab sonraki · Esc kapatır", s.value)
                            }
                            (Some(s), _) => format!(
                                "{} · şimdi {} · baş harfi ya da ↑↓ · Enter uygular · Tab sonraki",
                                s.choices.join(" · "),
                                s.value
                            ),
                            (None, Some(_)) => "Enter kilitler · Tab öbür değere geçer · Esc kapatır".to_owned(),
                            (None, None) if self.session.tool_id() == "chamfer" => {
                                "1. kenar; 2. kenar · Enter uygular".to_owned()
                            },
                            (None, None) if self.session.tool_id() == "fillet" => {
                                "Yarıçap · ör. 25cm · Enter uygular".to_owned()
                            },
                            // The tool's settings beside the cursor are a Tab away (docs/adr/8018).
                            (None, None) => {
                                let settings: Vec<String> = self.field_settings().into_iter().map(|s| s.label).collect();
                                if settings.is_empty() {
                                    format.input_hint()
                                } else {
                                    format!("{} · Tab: {}", format.input_hint(), settings.join(", "))
                                }
                            },
                        },
                        label: f
                            .setting
                            .as_ref()
                            .map(|s| s.label.as_str())
                            .or_else(|| f.mode.map(|m| self.lock_name(m))),
                        locks: self.lock_chips(&format),
                        length_first: self.locks.length.is_some(),
                    }),
                    self.lock_tag(&format),
                    doc.model.settings().drawing_font.unwrap_or(DrawingFont::Barlow),
                );
                // Vurgu rengi (docs/adr/0195 §1): the theme's accent or a named one.
                let accent = crate::viewport::highlight_color(
                    &self.settings.text("graphics.highlightColor"),
                    rgba8(Tokens::of(&self.theme()).accent),
                );
                // The map services' labels under the drawing's text, their credits in the
                // corner (services/overlay.rs, docs/adr/0208 §3, §9).
                let services = crate::services::in_use().then(|| {
                    let colors = crate::labels::colors(self.canvas(), &crate::viewport::palette(self.canvas()))
                        .in_mode(self.color_mode());
                    let key = crate::services::overlay::key_of_view(&doc.model, &self.viewport.camera, &colors);
                    let marks = self
                        .service_marks
                        .marks(&doc.model, &self.viewport.camera, &colors, key);
                    (marks, key)
                });
                let service_labels = services
                    .as_ref()
                    .and_then(|(marks, key)| crate::services::overlay::layer(marks.clone(), *key));
                let credits = services
                    .as_ref()
                    .map(|(marks, _)| self.credits_view(marks))
                    .unwrap_or_default();
                // The scale bar over the credits' strip.
                let lift = if credits.is_empty() { 0.0 } else { crate::services::app::scale_bar_lift() };
                // The drawing's text over the scene, under the marks (labels.rs, docs/adr/0055).
                // The text being edited in place is hidden meanwhile (text_field.rs).
                // The turned texts' pictures in the window's own pixels (label_picture.rs).
                crate::label_picture::set_device_scale(self.viewport.scale_factor());
                let labels = crate::labels::layer(
                    &doc.model,
                    doc.session,
                    &self.spatial,
                    &self.label_spots,
                    &self.viewport.camera,
                    self.canvas(),
                    &crate::viewport::palette(self.canvas()),
                    &format,
                    self.text_field
                        .as_ref()
                        .and_then(|f| f.editing)
                        .or(self.paragraph.as_ref().and_then(|p| p.editing)),
                    self.paragraph_preview(),
                    self.color_mode(),
                    self.label_size(),
                    lift,
                    self.label_engine(),
                    None,
                );
                // This frame's placed labels: the masks' shapes (docs/adr/0226 §3).
                let spots = self.label_spots.last();
                let area = self.viewport.view(
                    doc,
                    self.canvas(),
                    self.graphics(),
                    &self.selection,
                    accent,
                    self.session.cursor(),
                    Some(crate::viewport::Styling {
                        labels: spots.as_deref(),
                        ..self.styles.styling(&self.spatial, &self.settings)
                    }),
                );
                // The running command's strip on top (command_bar.rs); the right
                // button's menus over it all (drawing_menus.rs).
                // The text field over the drawing (text_field.rs).
                let typing = self.text_field_view();
                // The paragraph editor over the drawing (paragraph_editor.rs, docs/adr/0182 §4).
                let writing = self.paragraph_view();
                let active: Element<'_, Message> = ContextMenu::controlled(
                    stack![area]
                        // A road project's terrain surfaces over the drawing (terrain_layer.rs, plan/road.md Y2.4).
                        .extend(crate::terrain_layer::layer(
                            &doc.model,
                            &self.viewport.camera,
                            mark_colors(self.canvas()).fg,
                            Tokens::of(&self.theme()).danger,
                        ))
                        // A road project's alignments, km and curve points (alignment_layer.rs, plan/road.md Y3).
                        .extend(crate::alignment_layer::layer(
                            &doc.model,
                            &self.viewport.camera,
                            crate::alignment_layer::Colors {
                                ink: mark_colors(self.canvas()).fg,
                                accent: Tokens::of(&self.theme()).accent,
                                warning: Tokens::of(&self.theme()).warning,
                            },
                        ))
                        // Harita işi's approved shapes of the objects whose geometry changed (approved_layer.rs,
                        // docs/adr/4480), while Değişenler's selection stands.
                        .extend(crate::approved_layer::layer(
                            self.map.map_work.approved.as_ref(),
                            self.selection.version(),
                            &self.viewport.camera,
                            Tokens::of(&self.theme()).warning,
                        ))
                        .extend(service_labels)
                        .push(labels)
                        // Kaydırmalı karşılaştır: the other view right of the divider (split_view.rs, docs/adr/0523 §3).
                        .extend(self.swipe_layer())
                        .push(over)
                        .extend(credits)
                        // Genel bakış and Büyüteç over the drawing (navigation_cards.rs, docs/adr/0181).
                        .extend(self.navigation_view())
                        // The rollover card beside the pointer (hover_card.rs).
                        .extend(self.hover_card_view())
                        // Bilgi's card beside the object clicked (info_card.rs, docs/adr/0225 §5).
                        .extend(self.info_card_view())
                        // A changed external reference's card (xrefs_watch.rs, docs/adr/0349 §3).
                        .extend(self.xref_notice_view())
                        // A write the bonds refused, with its way through (bond_notice.rs, docs/adr/0790 §2).
                        .extend(self.bond_notice_view())
                        // Sıradakini seç's chip beside the click (selection_chip.rs, docs/adr/0187 §1).
                        .extend(self.selection_chip())
                        .extend(typing)
                        .extend(writing)
                        .extend(self.command_bar()),
                    self.drawing_menu.map(|open| open.at),
                    move |_| self.drawing_menu_items(),
                    Message::DrawingMenu(crate::drawing_menus::Event::Closed),
                )
                .into();
                // Görünümü böl: the other view beside it (split_view.rs, docs/adr/0285 §3).
                self.with_split(active)
            }
            None => container(
                EmptyState::new(Icon::Document, "Açık çizim yok")
                    .description(
                        "Web'de ya da burada kaydedilmiş bir KentOS çizimini (.kcad) açın. Orta tuşla \
                         sürükleyerek kaydırın, tekerlekle imlecin olduğu yere yakınlaştırın; orta tuşa \
                         çift tıklamak tümünü gösterir.",
                    )
                    .primary("Çizim aç…", Message::Run("file.open")),
            )
            .center(Fill)
            .into(),
        }
    }

    /// Büyüteç's window (docs/adr/0181 §5): the drawing area's scene through
    /// the lens's camera, the drawing's text and marks through it, and a small
    /// cross where the pointer is.
    pub(crate) fn lens_view<'a>(
        &'a self,
        doc: &'a Document,
        camera: kentos_render_wgpu::Camera,
    ) -> Element<'a, Message> {
        let format = Format::of(doc.settings());
        let grips = if self.selection.len() <= kentos_interaction::select::GRIP_LIMIT {
            self.spatial.grips(self.selection.ids())
        } else {
            Vec::new()
        };
        let marks = Marks {
            camera,
            snap: self.snap,
            snap_note: None,
            select: None,
            grips,
            params: self.param_grip_marks(),
            hot: self.session.hot_grips(),
            marked: self.marked_points(),
            ways: self.vertex_ways(),
            range: self.grade_marks().0.into_iter().map(|p| kentos_interaction::Vec2::new(p.x, p.y)).collect(),
            found: self.address_mark_label().or_else(|| self.data_mark_label()),
            extent: self.extent_mark(),
            problem: self
                .topology_problem()
                .or_else(|| self.gis.networks.mark.clone()),
            graph: self.parcel_graph_marks(),
            reach: self.impact_reach_marks(),
            zoning: self.zoning_marks(),
            pipes: self.pipes_marks(),
            baseline: self.baseline_marks(),
            broken: self.broken_marks(),
            missing: self.missing_pictures(),
            clips: self.clip_marks(),
            bonds: self.bond_marks(),
            constraints: self.constraint_marks(),
            constraint_pointer: self.constraint_view.pointer,
            tracking: None,
            crosshair: None,
            locks: None,
            colors: mark_colors(self.canvas()),
        };
        let over = preview::layer(
            &camera,
            marks,
            (if !self.session.is_running() && !self.session.grip_active() {
                self.cad_inspection_preview()
            } else {
                None
            })
                .or_else(|| self.session.preview(&format))
                .or_else(|| self.arch_audit_preview()),
            None,
            None,
            doc.model
                .settings()
                .drawing_font
                .unwrap_or(DrawingFont::Barlow),
        );
        let labels = crate::labels::lens_layer(
            &doc.model,
            doc.session,
            &self.spatial,
            &self.lens_spots,
            &camera,
            self.canvas(),
            &crate::viewport::palette(self.canvas()),
            &format,
            self.text_field
                .as_ref()
                .and_then(|f| f.editing)
                .or(self.paragraph.as_ref().and_then(|p| p.editing)),
            self.paragraph_preview(),
            self.color_mode(),
            self.label_size(),
            self.label_engine(),
        );
        stack![
            self.viewport.lens(camera, self.canvas()),
            labels,
            over,
            crate::navigation_cards::lens_cross(mark_colors(self.canvas()).fg.scale_alpha(0.85))
        ]
        .into()
    }

    pub(crate) fn panel_body(&self, panel: Panel) -> Element<'_, Message> {
        let Some(doc) = &self.document else {
            return container(label::muted("Açık çizim yok."))
                .padding(12)
                .into();
        };
        match panel {
            Panel::Layers => {
                // The open tree as flat rows, built only as they scroll into view
                // (the web's VirtualRows): a DXF can bring hundreds of layers.
                // Katman ara keeps what it finds (layer_tree.rs).
                let search = self.layer_search_state();
                let rows = crate::layer_tree::open_rows(doc.layers(), &search);
                // The selection's one layer is brought into view (layering.rs);
                // else the row the tree's keys chose (layer_tree.rs).
                let reveal = if self.layers_follow {
                    match self.selection_layers.as_slice() {
                        [layer] => rows.iter().position(|row| row.node.id == *layer),
                        _ => None,
                    }
                } else {
                    self.layer_reveal
                        .as_ref()
                        .and_then(|id| rows.iter().position(|row| row.node.id == *id))
                };
                let empty = if self.layer_query.trim().is_empty() && !self.layer_props_on() {
                    "Katman yok."
                } else {
                    "Aramayla eşleşen katman yok."
                };
                // A filtered layer's count is “geçen / bütün” (docs/adr/0211 §4): the column takes the widest.
                let count_width = self.layer_count_width(doc);
                let tree = TreeView::new([
                    TreeColumn::new("Ad").width(Fill),
                    TreeColumn::new("Öğe").width(count_width).align_right(),
                ])
                .virtualized(rows.len(), move |index| {
                    let row = &rows[index];
                    (
                        row.depth,
                        // The row's place is its id for dragging (layer_order.rs).
                        self.layer_node(doc, row.node, row.parent_visible).id(index),
                    )
                })
                // A row dragged before, after or into another (layer_order.rs).
                .on_move(|from, to, place| {
                    Message::Layer(crate::layering::Event::Move(from, to, place))
                })
                .reveal(reveal)
                .empty(empty)
                .height(Fill);
                column![self.layer_search_view(), tree].into()
            }
            // İşlemler: the toolbox and this session's runs (processing/panel.rs, docs/adr/0084).
            Panel::Processing => self.processing_panel(),
            // Bloklar: the drawing's blocks (blocks_panel.rs, docs/adr/0144).
            Panel::Blocks => self.blocks_panel_view(),
            // Şablonlar: the library's object templates (templates_panel.rs, docs/adr/0176 §4).
            Panel::Templates => self.templates_panel_view(),
            // Kaynaklar: folders and KentOS projects to add layers from (sources/, docs/adr/0199 §7).
            Panel::Sources => self.sources_view(),
            // Gezgin: the project's tree (navigator.rs, docs/adr/0778).
            Panel::Navigator => self.navigator_view(),
            Panel::RoadProject => self.road_project_view(),
            // Harita işi: a map project's work structure (map_work_panel.rs, docs/adr/1200 §4).
            Panel::MapWork => self.map_work_view(),
            // GNSS arşivi: a map project's arazi ölçümleri (map_survey_panel.rs, docs/adr/1340 §6).
            Panel::MapSurvey => self.map_survey_view(),
            // Öznitelikler, editable as the web's (properties/, docs/adr/0063).
            Panel::Properties => self.architecture_properties_view(doc),
        }
    }

    /// One row of the layer tree, as the web's LayersPanel draws it: a
    /// group's folder or a layer's colour (a click opens its colours), the
    /// name (a text box while renamed), the eye and the lock, the object
    /// count; the active layer's accent bar; the row's menu. Its children are
    /// rows of their own (layer_tree::open_rows).
    pub(super) fn layer_node<'a>(
        &'a self,
        doc: &'a Document,
        node: &'a LayerNode,
        parent_visible: bool,
    ) -> Node<'a, Message> {
        let active = doc.model.layers().active() == node.id;
        let (shown, total, filtered) = self.layer_counted(doc, node);
        let count: Element<'_, Message> = if filtered {
            kentos_ui::widget::tip(
                label::caption(format!("{shown} / {total}")),
                kentos_ui::widget::Tip::new(format!(
                    "Süzgeçten {shown} nesne geçiyor; katmanda {total} nesne var."
                )),
                iced::widget::tooltip::Position::Top,
            )
        } else {
            label::caption(total.to_string()).into()
        };
        // A filtered layer's funnel and a temporal layer's clock before its count, in the count's column
        // (docs/adr/0211 §4, 0210 §10).
        let mut badges: Vec<Element<'_, Message>> = Vec::new();
        if node.kind == LayerNodeType::Layer
            && node.service.is_none()
            && let Some(f) = &node.filter
        {
            let error = self.spatial.filter_error(&node.id);
            let what = crate::layer_filters::filter_text(f);
            badges.push(kentos_ui::widget::tip(
                kentos_ui::icon::icon(crate::icons::from_web(Some("funnel")))
                    .size(12.0)
                    .tone(if error.is_some() {
                        kentos_ui::icon::Tone::Warning
                    } else {
                        kentos_ui::icon::Tone::Accent
                    }),
                kentos_ui::widget::Tip::new(match error {
                    Some(why) => {
                        format!("Süzgeç çalışmıyor, hiçbir nesne geçmiyor: {why}\nSüzgeç: {what}")
                    }
                    None => format!("Süzgeç: {what}"),
                }),
                iced::widget::tooltip::Position::Top,
            ));
        }
        // Dondurulmuş (docs/adr/0311 §1): its own freeze marked; under a frozen group it is faint.
        if node.frozen == Some(true) {
            badges.push(kentos_ui::widget::tip(
                kentos_ui::icon::icon(crate::icons::from_web(Some("freeze")))
                    .size(12.0)
                    .tone(kentos_ui::icon::Tone::Accent),
                kentos_ui::widget::Tip::new(
                    "Dondurulmuş: çizilmez, seçilmez, kenetlenmez, Tümünü göster saymaz.\nÇözmek için sağ tık › Çöz.",
                ),
                iced::widget::tooltip::Position::Top,
            ));
        }
        if let Some(t) = node
            .time
            .as_ref()
            .filter(|_| node.kind == LayerNodeType::Layer)
        {
            badges.push(kentos_ui::widget::tip(
                kentos_ui::icon::icon(crate::icons::from_web(Some("clock")))
                    .size(12.0)
                    .tone(kentos_ui::icon::Tone::Muted),
                kentos_ui::widget::Tip::new(format!(
                    "Zamansal katman: {}{}{}",
                    t.start,
                    match (&t.end, t.cumulative) {
                        (Some(e), _) => format!(" – {e}"),
                        (None, true) => String::new(),
                        (None, false) => " (anlık)".to_owned(),
                    },
                    if t.cumulative { " (birikimli)" } else { "" }
                )),
                iced::widget::tooltip::Position::Top,
            ));
        }
        // Bağlı katman (bound.rs, docs/adr/0503): its file and table.
        if let Some(link) = doc
            .model
            .settings()
            .sources
            .iter()
            .find(|s| s.layer == node.id)
        {
            badges.push(kentos_ui::widget::tip(
                kentos_ui::icon::icon(crate::icons::from_web(Some("sourceLinked")))
                    .size(12.0)
                    .tone(kentos_ui::icon::Tone::Accent),
                kentos_ui::widget::Tip::new(format!(
                    "Bağlı katman: {} › {}\n{}\nSağ tık › Kaynak: yaz, oku, durumu, bağı çöz.",
                    std::path::Path::new(&link.path)
                        .file_name()
                        .map_or_else(|| link.path.clone(), |n| n.to_string_lossy().into_owned()),
                    link.table,
                    link.path
                )),
                iced::widget::tooltip::Position::Top,
            ));
        }
        let cells: Vec<Element<'_, Message>> = if badges.is_empty() {
            vec![count]
        } else {
            badges.push(count);
            vec![
                iced::widget::Row::with_children(badges)
                    .spacing(4)
                    .align_y(iced::Center)
                    .into(),
            ]
        };
        let mut row = Node::new(node.name.as_str())
            // Its place in the tree (the web's name title).
            .tip(kentos_ui::widget::Tip::new(
                doc.model.layers().path(&node.id),
            ))
            .cells(cells)
            .on_press(Message::LayerSelected(node.id.clone()))
            .selected(self.layer_row_selected(&node.id))
            .active(active)
            // Hidden of its own or under a hidden group, faint (the web's `data-hidden`).
            .muted(!parent_visible || !node.visible || doc.model.layers().is_frozen(&node.id))
            .toggle(Toggle::visible(
                node.visible,
                Message::LayerVisible(node.id.clone()),
            ))
            .toggle(Toggle::locked(
                node.locked,
                Message::LayerLocked(node.id.clone()),
            ))
            // The layer's own snapping (layer_snap.rs, docs/adr/0163 §4).
            .toggle(self.layer_snap_toggle(doc.model.layers(), node))
            .menu(move |_| self.layer_menu(node, active));
        if let Some((id, name)) = &self.renaming
            && *id == node.id
        {
            row = row.editor(tree_view::rename(
                name,
                |text| Message::Layer(crate::layering::Event::RenameInput(text)),
                Message::Layer(crate::layering::Event::RenameDone),
                Message::Layer(crate::layering::Event::RenameCancel),
            ));
        }
        match node.kind {
            // A scenario's own icon, its note in the tip (docs/adr/0210 §10).
            LayerNodeType::Group if node.scenario.is_some() => {
                let note = node.scenario.as_ref().and_then(|s| s.note.clone());
                row.folder()
                    .icon(
                        kentos_ui::icon::icon(crate::icons::from_web(Some("scenario"))).size(14.0),
                    )
                    .tip(kentos_ui::widget::Tip::new(format!(
                        "{}\nSenaryo{}",
                        doc.model.layers().path(&node.id),
                        note.map_or_else(String::new, |n| format!(": {n}"))
                    )))
                    .expanded(node.expanded, Message::LayerExpanded(node.id.clone()))
            }
            LayerNodeType::Group => row
                .folder()
                .icon(kentos_ui::icon::icon(Icon::Folder).size(14.0))
                .expanded(node.expanded, Message::LayerExpanded(node.id.clone())),
            // A layer drawn from a map service: its kind's icon, and why it shows nothing when it failed
            // (docs/adr/0208 §14).
            LayerNodeType::Layer if node.service.is_some() => {
                let service = node.service.as_ref().expect("a service");
                let mut row = row.icon(
                    kentos_ui::icon::icon(crate::icons::from_web(Some(
                        crate::services::app::service_icon(service),
                    )))
                    .size(14.0),
                );
                if let Some(why) = self.service_failure(service) {
                    row = row
                        .cells([kentos_ui::icon::icon(Icon::Warning)
                            .size(12.0)
                            .tone(kentos_ui::icon::Tone::Danger)
                            .into()])
                        .tip(kentos_ui::widget::Tip::new(format!(
                            "{}\n{why}",
                            doc.model.layers().path(&node.id)
                        )));
                }
                row
            }
            LayerNodeType::Layer => {
                let palette = crate::viewport::palette(self.canvas());
                let color = palette
                    .resolve(&node.style.color)
                    .map_or_else(|| hex_color(&node.style.color), rgba_color);
                // A typed layer's glyph in its colour, as a map legend draws its symbol (docs/adr/0622 §5); a layer of
                // any kind its colour's square.
                let face: Element<'_, Message> = match node.geometry {
                    Some(g) => kentos_ui::icon::icon(crate::layer_types::glyph(Some(g)))
                        .size(14.0)
                        .color(color)
                        .into(),
                    None => swatch(color),
                };
                row.icon(MenuButton::new(face, move || self.layer_colors(node)))
            }
        }
    }
}
