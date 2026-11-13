//! Öznitelikler in the shell: the panel worked out for the selection and kept, its live and topology
//! sections, the view with its thumbnails and cells, and its events.

use super::*;
use crate::gis;

impl App {
    pub(super) fn properties_key(&self, doc: &Document) -> PanelKey {
        (doc.session, doc.model.generation(), self.selection.version(), self.properties_scope.epoch)
    }

    pub(super) fn properties_kind(&self, doc: &Document) -> Option<&str> {
        self.properties_scope.kind.as_ref().and_then(|(session, selection, kind)| {
            (*session == doc.session && *selection == self.selection.version()).then_some(kind.as_str())
        })
    }

    /// The panel's content for the selection now.
    pub(crate) fn properties_panel(&self, doc: &Document) -> Panel {
        // An empty panel includes live source/file state. A single complex polyline/block is
        // expensive too: cache its geometric rows just like a large multi-selection.
        if self.selection.is_empty() {
            return self.work_out_panel(doc);
        }
        let key = self.properties_key(doc);
        let mut cache = self.properties_cache.0.borrow_mut();
        if let Some((known, panel)) = cache.as_ref()
            && *known == key
        {
            let mut panel = panel.clone();
            self.add_live_sections(&mut panel);
            return panel;
        }
        let panel = self.work_out_panel(doc);
        *cache = Some((key, panel.clone()));
        let mut panel = panel;
        self.add_live_sections(&mut panel);
        panel
    }

    pub(super) fn work_out_panel(&self, doc: &Document) -> Panel {
        // The kinds counted with the last one met at hand (docs/adr/1066 §3): a large selection's objects come in runs
        // of a kind, and a name compared for each of a million objects was a tenth of the panel.
        let mut counts: Vec<(&'static str, usize)> = Vec::new();
        let mut last = 0usize;
        let only = self.properties_kind(doc);
        let selected: Vec<Slot> = self.selection.ids().iter().copied().filter(|slot| {
            let Some(e) = doc.model.get(*slot) else { return false; };
            let kind = e.kind();
            if !counts.get(last).is_some_and(|(k, _)| std::ptr::eq(*k, kind) || *k == kind) {
                last = counts.iter().position(|(k, _)| *k == kind).unwrap_or_else(|| {
                    counts.push((kind, 0));
                    counts.len() - 1
                });
            }
            counts[last].1 += 1;
            only.is_none_or(|k| std::ptr::eq(k, kind) || kind == k)
        }).collect();
        let mut types = BTreeMap::<&'static str, usize>::new();
        for (kind, n) in counts {
            *types.entry(kind).or_default() += n;
        }
        // Past the limit (AutoCAD's PROPOBJLIMIT, docs/adr/1027): the selection's size and kinds only, no values and no
        // editors — working out and comparing a million objects' values took most of a select-all.
        if selected.len() > PROPERTIES_LIMIT {
            let total: usize = types.values().sum();
            let kinds = types
                .iter()
                .map(|(k, n)| format!("{n} {}", rows::kind_lower(crate::selecting::kind_title(k))))
                .collect::<Vec<_>>()
                .join(", ");
            // The objects on locked layers, each layer looked up once.
            let layers = doc.model.layers();
            let mut locked_layers: std::collections::HashMap<&str, bool> = std::collections::HashMap::new();
            let locked = selected
                .iter()
                .filter_map(|s| doc.model.get(*s))
                .filter(|e| {
                    let id = e.base().layer_id.as_str();
                    *locked_layers.entry(id).or_insert_with(|| layers.is_locked(id))
                })
                .count();
            return rows::Panel {
                summary: rows::Summary::Many { count: total, kinds },
                meta: Some(format!("{total} nesne")),
                sections: vec![rows::Section {
                    id: "large-selection".into(),
                    title: "Büyük seçim".into(),
                    rows: vec![
                        Row::note(&format!("{} nesne seçili.", selected.len())),
                        Row::note(&format!("Değerler en çok {PROPERTIES_LIMIT} nesnede gösterilir.")),
                        Row::note("Seçimi ya da tür seçimini daraltın."),
                        Row::note("Ya da komut kullanın (Katmana taşı)."),
                    ],
                }],
                types: types.into_iter().collect(),
                total,
                editable: total - locked,
            };
        }
        // The program's own rows (module_surfaces.rs).
        let shown = rows::Shown {
            crs: self.shows_property_rows(crate::module_surfaces::PropertyRows::Crs),
            widths: self.offers_tool_option(kentos_interaction::ToolOption::Widths),
            mask: self.shows_property_rows(crate::module_surfaces::PropertyRows::Mask),
            separators: self.shows_property_rows(crate::module_surfaces::PropertyRows::Separators),
        };
        let mut panel = rows::panel(
            doc,
            &selected,
            |ids| {
                // Summed by the geometry store, as the web sums them: a selection can be large.
                let ids: Vec<f64> = ids.iter().map(|s| f64::from(s.0)).collect();
                self.spatial.store().measure(&ids)
            },
            shown,
        );
        panel.types = types.into_iter().collect();
        panel.total = panel.types.iter().map(|(_, n)| n).sum();
        if self.properties_kind(doc).is_some() {
            panel.meta = Some(format!("{} / {} nesne", selected.len(), panel.total));
        }
        if let Some((slot, sub)) = self.selection.sub().filter(|(_, sub)| !sub.path.is_empty()) {
            let path = sub.path.iter().map(u32::to_string).collect::<Vec<_>>().join("/");
            let mut info = vec![Row::text("Parça yolu", path), Row::text("Hedef", "Blok yerleştirmesi"),
                Row::note("Tanım için: Bloğu düzenle.")];
            if let Some((assigned, displayed)) = kentos_native_application::entities_select::block_piece_colors(&doc.model, slot, &sub.path) {
                info.push(Row::text("Atanan renk", assigned));
                info.push(Row::text("Görünür renk", displayed));
            }
            panel.sections.insert(0, rows::Section {id:"picked-piece".into(),title:"Seçilen blok parçası".into(),rows:info});
        }
        // One object of a map project's work: where it comes from (docs/adr/1200 §4); a map's programs'.
        if let [slot] = selected.as_slice()
            && self.shows_property_rows(crate::module_surfaces::PropertyRows::MapWork)
            && let Some(section) = map_work_section(doc, *slot)
        {
            // Right under the object's own rows, where it is seen without scrolling.
            let at = panel.sections.len().min(1);
            panel.sections.insert(at, section);
        }
        // One accepted GNSS point, or an object made from accepted points: its measurement (docs/adr/1340 §1); read
        // only, a map's and a GIS's.
        if let [slot] = selected.as_slice()
            && self.shows_property_rows(crate::module_surfaces::PropertyRows::Survey)
            && let Some(section) = survey_section(doc, *slot)
        {
            let at = panel.sections.len().min(1);
            panel.sections.insert(at, section);
        }
        // Nothing selected: the layer chosen in the tree (else the active one), when a service draws or fed it; a
        // map's programs' (module_surfaces.rs).
        if selected.is_empty()
            && self.shows_property_rows(crate::module_surfaces::PropertyRows::Services)
        {
            let layers = doc.model.layers();
            let id = self.selected_layer.as_deref().unwrap_or(layers.active());
            if let Some(node) = layers.get(id)
                && let Some(section) = rows::service_section(
                    doc,
                    node,
                    node.service.as_ref().and_then(|s| self.service_failure(s)),
                )
            {
                // The layer chosen is what the user is looking at: its section first.
                panel.sections.insert(0, section);
            }
        }
        panel
    }

    pub(super) fn add_live_sections(&self, panel: &mut Panel) {
        // Attachment tasks may finish without editing geometry. Never freeze their actions in the cache.
        // Several objects: their findings counted by state (docs/adr/4600), under Geometri as one object's are.
        if let ids @ [_, _, ..] = self.selection.ids()
            && self.shows_property_rows(crate::module_surfaces::PropertyRows::Topology)
            && let Some(section) = self.topology_summary_section(ids)
        {
            let at = panel.sections.iter().position(|s| s.id == "geometry").map_or(panel.sections.len(), |i| i + 1);
            panel.sections.insert(at, section);
        }
        if let [slot] = self.selection.ids() {
            // The object's findings of the Topoloji tab's last check (docs/adr/3800): a check changes no drawing, so
            // the section is not cached with the object's rows. Under Geometri.
            if self.shows_property_rows(crate::module_surfaces::PropertyRows::Topology)
                && let Some(section) = self.topology_section(*slot)
            {
                let at = panel.sections.iter().position(|s| s.id == "geometry").map_or(panel.sections.len(), |i| i + 1);
                panel.sections.insert(at, section);
            }
            panel
                .sections
                .extend(rows::attachments::sections(self, *slot));
        }
    }

    /// Topoloji (docs/adr/3800): each finding its problem, its state in a drop-down that marks it as the tab does
    /// (Değerlendirilmedi, Hata, İstisna; docs/adr/4600) with Gerekçe's words (a row of its own, the tab's), its measure,
    /// reason and rule in a quiet line under it, and a button that opens the Topoloji tab on it; a word when the drawing
    /// was not checked, when the check found nothing of the object, or when the drawing changed since.
    pub(super) fn topology_section(&self, slot: Slot) -> Option<rows::Section> {
        use crate::topology::object::ObjectFindings;
        use kentos_commands::{TopologyExceptionOperation as Op, TopologyFindingState as State};
        let (rows, n) = match self.object_findings(slot)? {
            ObjectFindings::NotChecked => (vec![Row::note("Denetlenmedi: Topoloji sekmesinde Denetle.")], 0),
            ObjectFindings::Checked { findings, old } => {
                let n = findings.len();
                let mut rows = Vec::new();
                if n > 0 {
                    rows.push(Row {
                        editor: Some(Editor::Text(Field::TopologyReason)),
                        ..Row::text("Gerekçe", self.gis.topology.reason.clone())
                    });
                    rows.push(Row::note("Hata ve istisna bu gerekçeyle yazılır."));
                }
                for f in findings {
                    let word = match f.status {
                        State::Open => "Değerlendirilmedi",
                        State::Error => "Hata",
                        State::Exception => "İstisna",
                        State::Lapsed => "Yeniden değerlendir",
                    };
                    let pick = |label: &str, state: State, op: Op| rows::Choice::Pick {
                        label: label.to_owned(),
                        swatch: None,
                        icon: None,
                        wide: false,
                        chosen: f.status == state,
                        enabled: true,
                        message: Message::Gis(gis::Event::Topology(
                            crate::topology::Event::Review(f.key.clone(), op),
                        )),
                    };
                    let items = vec![
                        pick("Değerlendirilmedi", State::Open, Op::Unmark),
                        pick("Hata", State::Error, Op::Error),
                        pick("İstisna", State::Exception, Op::Mark),
                    ];
                    // The quiet lines: the mark's reason, then the measure and the rule.
                    let note = if f.measure.is_empty() { format!("Kural: {}", f.rule) } else { format!("{} · kural: {}", f.measure, f.rule) };
                    rows.push(Row {
                        editor: Some(Editor::Select { text: word.to_owned(), swatch: None, icon: None, items }),
                        thumb: Some(rows::Thumb {
                            key: None,
                            icon: "zoomSelection",
                            title: "Topoloji sekmesinde göster".into(),
                            open: Message::Gis(gis::Event::Topology(crate::topology::Event::Show(
                                f.key.clone(),
                            ))),
                        }),
                        ..Row::text(f.problem.clone(), word)
                    });
                    if let Some(why) = &f.reason {
                        rows.push(Row::note(&format!("Gerekçe: {why}")));
                    }
                    rows.push(Row::note(&note));
                }
                if n == 0 {
                    rows.push(Row::note("Son denetimde bulgusu yok."));
                }
                if old {
                    let mut stale = Row::note("Çizim denetimden sonra değişti; Topoloji sekmesinde Değişenleri denetle.");
                    stale.warn = true;
                    rows.push(stale);
                }
                (rows, n)
            }
        };
        Some(rows::Section {
            id: "topology".into(),
            title: if n > 0 { format!("Topoloji ({n})").into() } else { "Topoloji".into() },
            rows,
        })
    }

    /// Topoloji for several objects (docs/adr/4600): the findings naming any of them, counted by state, from the tab's
    /// last check; a word when the drawing was not checked or changed since.
    pub(super) fn topology_summary_section(&self, slots: &[Slot]) -> Option<rows::Section> {
        let rows = match self.selection_findings(slots)? {
            None => vec![Row::note("Denetlenmedi: Topoloji sekmesinde Denetle.")],
            Some(s) => {
                let figure = |label: &'static str, n: usize| Row { numeric: true, ..Row::text(label, n.to_string()) };
                let mut rows = vec![figure("Bulgular", s.total())];
                // Each state with findings, a row of its own (a long line would be cut in the panel).
                for (n, word) in [(s.open, "Değerlendirilmedi"), (s.errors, "Hata"), (s.exceptions, "İstisna"), (s.lapsed, "Yeniden değerlendir")] {
                    if n > 0 {
                        rows.push(figure(word, n));
                    }
                }
                rows.push(Row::note(&format!("Denetlenen {} nesnenin; ortak bulgu bir kez sayılır.", s.objects)));
                if s.old {
                    let mut stale = Row::note("Çizim denetimden sonra değişti; Topoloji sekmesinde Değişenleri denetle.");
                    stale.warn = true;
                    rows.push(stale);
                }
                rows
            }
        };
        Some(rows::Section { id: "topology".into(), title: "Topoloji".into(), rows })
    }

    /// The header's meta: “#12”, “3 nesne”; none with nothing selected.
    pub(crate) fn properties_meta(&self, doc: &Document) -> Option<String> {
        self.properties_panel(doc).meta
    }

    /// The panel's body: the summary or the empty state, then the grid.
    pub(crate) fn properties_view<'a>(&'a self, doc: &'a Document) -> Element<'a, Message> {
        let panel = self.properties_panel(doc);
        let total = panel.total;
        let targets = self.properties_kind(doc).and_then(|kind| panel.types.iter().find(|(k, _)| *k == kind).map(|(_, n)| *n)).unwrap_or(total);
        // Like a CAD property palette: type/count and compact tools share one row.
        use kentos_ui::widget::select::{Choice as SelectChoice, Select};
        let kinds: Vec<_> = panel.types.iter().map(|(kind, _)| kind.to_string()).collect();
        let selected = if panel.types.len() > 1 { self.properties_kind(doc).and_then(|kind| kinds.iter().position(|k| k == kind)).map_or(0, |i| i + 1) } else { 0 };
        let all = match panel.types.as_slice() {
            [] => "Seçili nesne yok".to_owned(),
            [(kind, n)] => format!("{} ({n})", crate::selecting::kind_title(kind)),
            _ => format!("Tümü ({total})"),
        };
        let mut choices = vec![SelectChoice::new(all)];
        if panel.types.len() > 1 {
            choices.extend(panel.types.iter().map(|(kind, n)| SelectChoice::new(format!("{} ({n})", crate::selecting::kind_title(kind)))));
        }
        let scope = Select::new(choices, Some(selected), move |i| Message::Properties(Event::Kind(i.checked_sub(1).and_then(|i| kinds.get(i).cloned()))))
            .searchable(false);
        let summary = match &panel.summary {
            Summary::Empty => "Çizimde bir nesneye tıklayın veya özellik koşullarıyla seçin.".to_owned(),
            Summary::One { kind, label, layer } => format!("{kind}{}{}",
                label.as_ref().map_or(String::new(), |label| format!(" · {label}")),
                layer.as_ref().map_or(String::new(), |(_, path)| format!(" · {path}"))),
            Summary::Many { count, kinds } => format!("{count} nesne · {kinds}. Tür seçimi yalnız özellik panelini daraltır; sahne seçimi korunur."),
        };
        let scope = kentos_ui::widget::tip(scope, kentos_ui::widget::Tip::new("Özellik kapsamı").body(summary), iced::widget::tooltip::Position::Bottom);
        let compact_tool = |glyph, title, message: Option<Message>, active| {
            kentos_ui::widget::tip(
                iced::widget::button(icon(glyph).size(14.0))
                    .padding(kentos_ui::theme::metrics::icon_small(14.0))
                    .on_press_maybe(message)
                    .style(style::button::tool(active)),
                kentos_ui::widget::Tip::new(title),
                iced::widget::tooltip::Position::Bottom,
            )
        };
        let isolated = doc.model.objects_isolated();
        let isolation_command = if isolated { "view.endObjectIsolation" } else { "view.isolateObjects" };
        let query = compact_tool(Icon::Filter, "Özelliklerle seç…", Some(Message::Properties(Event::Query)), false);
        let isolation = compact_tool(
            crate::icons::from_web(Some("layerIsolate")),
            if isolated { "Nesne yalıtımını bitir" } else { "Seçimi geçici yalıt" },
            self.available(isolation_command).then_some(Message::Run(isolation_command)), isolated,
        );
        let selection_bar = boxed(row![container(scope).width(Fill), query, isolation].spacing(4).align_y(Center), [6, 8]);
        let mut body = Column::new().push(selection_bar);
        if isolated {
            body = body.push(boxed(label::caption("Geçici yalıtım açık").style(style::text::accent), [0, 12]));
        }
        if targets > panel.editable {
            body = body.push(boxed(label::caption(format!("{} düzenlenebilir · {} kilitli", panel.editable, targets - panel.editable)), [3, 12]));
        }
        let mut sheet = PropertySheet::new();
        for section in panel.sections {
            let (open, toggle) = match &section.id {
                std::borrow::Cow::Borrowed(id) => (!self.props_closed.contains(id), Event::Toggle(id)),
                std::borrow::Cow::Owned(id) => (!self.props_groups_closed.contains(id), Event::ToggleGroup(id.clone())),
            };
            sheet = sheet.section(
                section.title,
                open,
                Message::Properties(toggle),
            );
            if !open {
                continue;
            }
            for mut row in section.rows {
                // A note under a value of “Bu değer nereden geliyor?” runs the sheet's width (docs/adr/1100 §5).
                if row.note && (row.warn || section.id.starts_with("blockParams") || section.id == "valueOrigin") {
                    sheet = sheet.note(self.cell(row));
                    continue;
                }
                if doc.settings().workspace == Some(kentos_commands::Workspace::Cad)
                    && row.numeric
                    && row.editor.is_none()
                    && row.thumb.is_none()
                    && let Some(unit) = &row.unit
                    && !unit.is_empty()
                {
                    sheet = sheet.measurement(row.label.into_owned(), row.value, unit.to_string());
                    continue;
                }
                let label = row.label.clone();
                // An attachment's picture before its cell (docs/adr/0225 §2).
                let thumb = row.thumb.take();
                let cell = self.cell(row);
                let cell = match thumb {
                    Some(t) => kentos_ui::widget::Cell::flat(
                        iced::widget::row![
                            container(self.thumb(t)).padding(iced::Padding {
                                left: kentos_ui::theme::metrics::cell_end(),
                                right: 2.0,
                                ..iced::Padding::ZERO
                            }),
                            cell,
                        ]
                        .align_y(iced::Center),
                    )
                    .into(),
                    None => cell,
                };
                sheet = sheet.row(label, cell);
            }
        }
        let body: Element<'a, Message> = body.push(sheet).width(Fill).into();
        let key = self.properties_key(doc);
        // A different target must get a fresh edit-cell tree, even if its shown value is identical.
        iced::widget::keyed_column([(key, body.map(move |message| match message {
            Message::Properties(event) => Message::Properties(Event::Guarded(key, Box::new(event))),
            other => other,
        }))]).into()
    }

    /// An attachment's picture: its photo once decoded, else its kind's icon;
    /// pressed, it opens it; under the pointer, where it is.
    pub(super) fn thumb<'a>(&'a self, t: rows::Thumb) -> Element<'a, Message> {
        const W: f32 = 36.0;
        const H: f32 = 27.0;
        let face: Element<'a, Message> = match self.photo_picture(t.key.as_deref()) {
            Some(picture) => {
                crate::photo_view::photo(Some(picture), W, H, crate::photo_view::Fit::Cover)
            }
            None => {
                container(kentos_ui::icon::icon(crate::icons::from_web(Some(t.icon))).size(16.0))
                    .width(W)
                    .height(H)
                    .center_x(W)
                    .center_y(H)
                    .style(kentos_ui::style::container::field)
                    .into()
            }
        };
        kentos_ui::widget::tip(
            iced::widget::button(face)
                .on_press(t.open)
                .padding(0)
                .style(kentos_ui::style::button::ghost),
            kentos_ui::widget::Tip::new(t.title),
            iced::widget::tooltip::Position::Top,
        )
    }

    /// A row's value: text, an editable cell or a drop-down.
    pub(super) fn cell<'a>(&'a self, row: Row) -> Element<'a, Message> {
        let unit = row.unit.clone();
        if row.note {
            let tone = if row.warn {
                style::text::warning
            } else {
                style::text::muted
            };
            return label::caption(row.value).style(tone).into();
        }
        match row.editor {
            None => property_grid::value(row.value, row.numeric, unit.map(|u| u.into_owned())),
            Some(Editor::Text(field) | Editor::Number(field)) => {
                EditCell::new(row.value, move |text| {
                    Message::Properties(Event::Commit(field.clone(), text))
                })
                .numeric(row.numeric)
                .unit(unit.map(|u| u.into_owned()).unwrap_or_default())
                .into()
            }
            Some(Editor::Select {
                text,
                swatch,
                icon,
                items,
            }) => {
                // Colours resolved now: the menu is built again at every opening.
                let items: Vec<(Choice, Option<Color>)> = items
                    .into_iter()
                    .map(|choice| {
                        let color = match &choice {
                            Choice::Pick {
                                swatch: Some(value),
                                ..
                            } => Some(self.drawing_color(value)),
                            _ => None,
                        };
                        (choice, color)
                    })
                    .collect();
                match icon {
                    // A picture before the value (a text's alignment, docs/adr/0145).
                    Some(name) => property_grid::icon_choice(
                        text,
                        crate::icons::from_web(Some(name)),
                        move || menu(&items),
                    ),
                    None => property_grid::choice(
                        text,
                        swatch.map(|value| self.drawing_color(&value)),
                        move || menu(&items),
                    ),
                }
            }
        }
    }

    /// A drawing colour as the panel shows it (a layer's, one of the
    /// colours): the theme's ink colours by the drawing's palette, else the hex.
    pub(crate) fn drawing_color(&self, value: &str) -> Color {
        let palette = crate::viewport::palette(self.canvas());
        palette
            .resolve(value)
            .map_or_else(|| crate::view::hex_color(value), crate::view::rgba_color)
    }

    /// What the panel asked for: an edit of the drawing, through the
    /// product commands as the web's panel writes (docs/adr/0066), or a
    /// section toggled. The command's refusal and warnings are said.
    pub(crate) fn properties_event(&mut self, event: Event) {
        let event = match event {
            Event::Guarded(key, event) => {
                if self.document.as_ref().is_none_or(|doc| self.properties_key(doc) != key) {
                    self.warn("Özellik düzenlemesi uygulanmadı: çizim veya seçim değişti. Güncel hedefi yeniden seçin.");
                    return;
                }
                *event
            }
            other => other,
        };
        // Docs/adr/4600: Gerekçe typed in Öznitelikler › Topoloji is the tab's own words; the drawing does not change.
        if let Event::Commit(Field::TopologyReason, text) = &event {
            self.gis.topology.reason = text.trim().to_owned();
            return;
        }
        if matches!(event, Event::Query) {
            let scope = if self.selection.is_empty() { "all" } else { "selection" };
            let mut input = serde_json::json!({"scope":scope});
            if let Some(kind) = self.document.as_ref().and_then(|doc| self.properties_kind(doc)) {
                input["kinds"] = serde_json::json!([kind]);
            }
            let path = self.selection.sub().map(|(_, sub)| sub.path.iter().map(u32::to_string).collect::<Vec<_>>().join("/")).unwrap_or_default();
            self.open_processing("selection.byExpression", Some(serde_json::from_value(serde_json::json!({
                "input":input,"condition":"true","mode":"new","cadFilter":true,"blockPath":path
            })).expect("fixed query values")));
            return;
        }
        if let Event::Kind(kind) = event {
            self.properties_scope.kind = self.document.as_ref().and_then(|doc| kind.map(|kind| (doc.session, self.selection.version(), kind)));
            self.properties_scope.epoch = self.properties_scope.epoch.wrapping_add(1);
            return;
        }
        if let Event::Toggle(id) = event {
            if !self.props_closed.remove(id) {
                self.props_closed.insert(id);
            }
            return;
        }
        if let Event::ToggleGroup(id) = event {
            if !self.props_groups_closed.remove(&id) {
                self.props_groups_closed.insert(id);
            }
            return;
        }
        if let Event::SelectObject(slot) = event {
            self.selection.set([slot]);
            return;
        }
        if let Event::TextLayoutPreview(slot, engine) = event {
            if self.session.is_running() || self.paragraph.is_some() || self.text_field.is_some() {
                self.warn("Önce açık işlemi tamamlayın veya Esc ile vazgeçin.");
                return;
            }
            let Some(doc) = &self.document else {
                return;
            };
            let Some(Entity::Text(text)) = doc.model.get(slot) else {
                return;
            };
            if doc.model.layers().is_locked(&text.base.layer_id) {
                self.warn("Kilitli katmandaki yazı düzenlenemez.");
                return;
            }
            let text = text.clone();
            self.edit_paragraph(slot, &text);
            self.paragraph_event(crate::paragraph_editor::Event::Layout(engine));
            return;
        }

        if let Event::Casework(slot, "select") = event {
            let Some(model) = self.document.as_ref().map(|doc| &doc.model) else {
                return;
            };
            let Some((run, _)) = model.uid(slot).and_then(|uid| model.casework_member(uid)) else {
                return;
            };
            let slots: Vec<_> = run
                .members
                .iter()
                .filter_map(|m| model.slot_of(kentos_domain::Uuid::from_bytes(m.uid.0)))
                .collect();
            self.selection.set(slots);
            return;
        }
        if let Event::DimensionSet(slot, action @ ("select" | "row" | "source")) = event {
            let Some(model) = self.document.as_ref().map(|doc| &doc.model) else {
                return;
            };
            let Some((set, index)) = model
                .uid(slot)
                .and_then(|uid| model.dimension_set_of_item(uid))
            else {
                return;
            };
            let row = set.members[index].row;
            let slots: Vec<_> = if action == "source" {
                set.sources()
                    .iter()
                    .filter_map(|id| id.to_text().parse().ok())
                    .filter_map(|uid| model.slot_of(uid))
                    .collect()
            } else {
                set.members
                    .iter()
                    .filter(|member| action == "select" || member.row == row)
                    .filter_map(|member| {
                        member
                            .uid
                            .to_text()
                            .parse()
                            .ok()
                            .and_then(|uid| model.slot_of(uid))
                    })
                    .collect()
            };
            self.selection.set(slots);
            if action == "source" {
                self.zoom_selection();
            }
            return;
        }
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        // A linked picture's file is relative to the drawing's folder (docs/adr/0192 §2).
        let folder = doc
            .path
            .as_deref()
            .and_then(std::path::Path::parent)
            .map(std::path::Path::to_path_buf);
        let drawing = doc.path.clone();
        let model = &mut doc.model;
        let said = match event {
            Event::Toggle(_) | Event::ToggleGroup(_) | Event::SelectObject(_) | Event::Kind(_) | Event::Query | Event::Guarded(_, _) | Event::TextLayoutPreview(_, _) => Vec::new(),
            Event::Unlink(ids) => {
                let mut input = set_input(model, &ids, PropertiesOperation::Unlink);
                input.unlink = true;
                properties::set_properties(model, input)
            }
            Event::RecreateBoundary(slot, form) => properties::recreate_boundary(model, slot, form),
            // On a hidden layer they vanish from the drawing, still selected: the command's warning says so.
            Event::Layer(ids, layer) => {
                let mut input = set_input(model, &ids, PropertiesOperation::Layer);
                input.layer_id = Some(layer);
                properties::set_properties(model, input)
            }
            Event::Color(ids, color) => {
                let mut input = set_input(model, &ids, PropertiesOperation::Color);
                input.color = Some(color);
                properties::set_properties(model, input)
            }
            Event::Weight(ids, weight) => {
                let mut input = set_input(model, &ids, PropertiesOperation::LineWeight);
                input.line_weight = Some(weight);
                properties::set_properties(model, input)
            }
            Event::LineType(ids, t) => {
                let mut input = set_input(model, &ids, PropertiesOperation::LineType);
                input.line_type = Some(t);
                properties::set_properties(model, input)
            }
            Event::LinePattern(ids, p) => {
                let mut input = set_input(model, &ids, PropertiesOperation::LineType);
                input.line_pattern = Some(p);
                properties::set_properties(model, input)
            }
            Event::LineTypeScale(ids, s) => {
                let mut input = set_input(model, &ids, PropertiesOperation::LineTypeScale);
                input.line_type_scale = Some(s);
                properties::set_properties(model, input)
            }
            Event::Transparency(ids, t) => {
                let mut input = set_input(model, &ids, PropertiesOperation::Transparency);
                input.transparency = Some(t);
                properties::set_properties(model, input)
            }
            Event::Wipeout(ids, w) => {
                let mut input = set_input(model, &ids, PropertiesOperation::Wipeout);
                input.wipeout = Some(w);
                properties::set_properties(model, input)
            }
            Event::Pattern(slot, choice) => match model.get(slot) {
                Some(Entity::Hatch(h)) => {
                    let plot = model.settings().plot_scale;
                    match kentos_interaction::hatch_options::rechosen(&h.pattern, choice, plot) {
                        Some(pattern) => {
                            let mut h = h.clone();
                            h.pattern = pattern;
                            properties::set_geometry(model, slot, &Entity::Hatch(h))
                        }
                        None => Vec::new(),
                    }
                }
                _ => Vec::new(),
            },
            Event::GradientShape(slot, shape) => match model.get(slot) {
                Some(Entity::Hatch(h)) if h.pattern.gradient.is_some() => {
                    let mut h = h.clone();
                    if let Some(g) = &mut h.pattern.gradient {
                        g.shape = shape;
                    }
                    properties::set_geometry(model, slot, &Entity::Hatch(h))
                }
                _ => Vec::new(),
            },
            Event::GradientColour(slot, colour) => match model.get(slot) {
                Some(Entity::Hatch(h)) if h.pattern.gradient.is_some() => {
                    let mut h = h.clone();
                    if let Some(g) = &mut h.pattern.gradient {
                        g.color2 = colour;
                    }
                    properties::set_geometry(model, slot, &Entity::Hatch(h))
                }
                _ => Vec::new(),
            },
            Event::GradientInverted(slot, inverted) => match model.get(slot) {
                Some(Entity::Hatch(h)) if h.pattern.gradient.is_some() => {
                    let mut h = h.clone();
                    if let Some(g) = &mut h.pattern.gradient {
                        g.inverted = inverted;
                    }
                    properties::set_geometry(model, slot, &Entity::Hatch(h))
                }
                _ => Vec::new(),
            },
            Event::InsertBlock(slot, block) => properties::replace_block(model, slot, block),
            Event::EmbedImage(slot) => crate::pictures::embed(model, slot, drawing.as_deref()),
            Event::EmbedRaster(slot) => crate::rasters::embed(model, slot, folder.as_deref()),
            Event::Wall(slot, change) => properties::edit_wall_kept(model, slot, change, wall_parts::keep()),
            Event::WallKeep(r) => {
                wall_parts::set_keep(r);
                Vec::new()
            }
            Event::WallOp(slot, op) => properties::edit_wall_op(model, slot, op),
            Event::Opening(slot, op) => properties::edit_opening(model, slot, op),
            Event::Module(op) => properties::edit_module(model, op),
            Event::Elevation(op) => properties::edit_elevation(model, op),
            Event::Section(op) => properties::edit_section(model, op),
            Event::Material(op) => properties::edit_material(model, op),
            Event::Space(slot, op) => properties::edit_space(model, slot, *op),
            Event::Slab(slot, op) => properties::edit_slab(model, slot, op),
            Event::SpaceSeparator(slot, on) => {
                let arch = on.then(|| {
                    model
                        .get(slot)
                        .and_then(kentos_domain::space_follow::separator_path)
                        .and_then(|p| p.space_separator.clone())
                        .unwrap_or_default()
                });
                properties::set_space_separator(model, slot, arch)
            }
            Event::Arch(slot, op) => properties::edit_arch(model, slot, op),
            Event::Levels(slots, level) => properties::assign_level(model, &slots, &level),
            Event::Tag(slot, change) => properties::edit_tag(model, slot, change),
            Event::ViewMark(slot, op) => properties::edit_viewmark(model, slot, op),
            Event::Parameter(slot, name, value) => {
                properties::set_parameters(model, slot, Some(BTreeMap::from([(name, value)])), None)
            }
            Event::ParametersReset(slot) => properties::set_parameters(model, slot, None, None),
            Event::ParameterReset(slot, name) => {
                properties::set_parameters(model, slot, None, Some(vec![name]))
            }
            Event::InsertMirror(slot, mirror) => match model.get(slot) {
                Some(Entity::Insert(i)) => {
                    let mut i = i.clone();
                    i.mirror = mirror;
                    properties::set_geometry(model, slot, &Entity::Insert(i))
                }
                _ => Vec::new(),
            },
            Event::TextAlign(slots, to) => properties::realign_texts(model, &slots, to),
            Event::TextMask(slots, on) => properties::set_text_mask(model, &slots, on),
            Event::PolylineClosed(slots, on) => properties::set_polyline_closed(model, &slots, on),
            Event::TextStyle(slots, id) => {
                properties::apply_text_style(model, &slots, id.as_deref())
            }
            Event::RestoreStyle(slots) => properties::restore_styles(model, &slots),
            Event::Annotative(slots, on) => kentos_interaction::annotative::run(
                model,
                &slots,
                if on {
                    kentos_commands::AnnotativeOperation::Make
                } else {
                    kentos_commands::AnnotativeOperation::Unmake
                },
                Vec::new(),
            ),
            Event::AnnotativeReset(slots) => kentos_interaction::annotative::run(
                model,
                &slots,
                kentos_commands::AnnotativeOperation::Reset,
                Vec::new(),
            ),
            Event::DimensionStyle(slots, id) => {
                properties::apply_dimension_style(model, &slots, id.as_deref())
            }
            Event::TableStyle(slots, id) => properties::apply_table_style(model, &slots, id.as_deref()),
            Event::LeaderArrow(slots, arrow) => properties::change_leaders(model, &slots, |l| {
                (l.arrow != arrow).then(|| LeaderEntity { arrow, ..l.clone() })
            }),
            Event::LeaderMask(slots, mask) => properties::change_leaders(model, &slots, |l| {
                (l.mask != mask).then(|| LeaderEntity { mask, ..l.clone() })
            }),
            Event::DimensionMask(slots, mask) => {
                properties::change_dimensions(model, &slots, |d| {
                    (d.mask != mask).then(|| DimensionEntity { mask, ..d.clone() })
                })
            }
            Event::DimensionFree(slots) => properties::free_dimensions(model, &slots),
            Event::DimensionSet(slot, action) => dimension_set_action(model, slot, action),
            Event::Casework(slot, action) => casework_action(model, slot, action),
            Event::CaseworkValue(slot, field, value) => casework_value(model, slot, field, &value),
            Event::DimensionSetValue(slot, field, value) => {
                if matches!(field, "dimensionStyle" | "layerId" | "tight") {
                    let mut operation = serde_json::json!({"kind":"set"});
                    operation[field] = serde_json::json!(value);
                    dimension_set_edit(model, slot, operation)
                } else {
                    Vec::new()
                }
            }
            Event::DimensionEdit(slots, op) => {
                use kentos_commands::DimensionEdit as E;
                properties::edit_dimensions(model, &slots, |uids| match op {
                    DimensionOp::Options(options) => E::Options { uids, options },
                    DimensionOp::Unbreak => E::Unbreak { uids },
                    DimensionOp::Level(level) => E::Level { uids, level },
                    DimensionOp::Home => E::MoveText {
                        uid: uids[0].clone(),
                        along: 0.0,
                        across: 0.0,
                    },
                })
            }
            Event::DimensionAxis(slots, angle) => {
                let y = angle == 0.0;
                properties::change_dimensions(model, &slots, |d| {
                    (rows::is_y_axis(d.angle) != y).then(|| DimensionEntity {
                        angle: Some(angle),
                        ..d.clone()
                    })
                })
            }
            // The same colour whatever its case is left out.
            Event::DimensionLineColor(slots, part, color) => {
                properties::change_dimensions(model, &slots, |d| {
                    let mut look = d.look.clone();
                    let field = match part {
                        Part::Line => &mut look.dim_line_color,
                        Part::Ext => &mut look.ext_color,
                        Part::Value => &mut look.text_color,
                    };
                    let same = match (field.as_deref(), color.as_deref()) {
                        (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
                        (a, b) => a == b,
                    };
                    if same {
                        return None;
                    }
                    *field = color.clone();
                    Some(DimensionEntity { look, ..d.clone() })
                })
            }
            Event::DimensionLineWeight(slots, part, weight) => {
                properties::change_dimensions(model, &slots, |d| {
                    let mut look = d.look.clone();
                    let field = match part {
                        Part::Line => &mut look.dim_line_weight,
                        Part::Ext => &mut look.ext_weight,
                        Part::Value => return None,
                    };
                    if *field == weight {
                        return None;
                    }
                    *field = weight;
                    Some(DimensionEntity { look, ..d.clone() })
                })
            }
            Event::DimensionLineType(slots, part, kind) => {
                properties::change_dimensions(model, &slots, |d| {
                    let mut look = d.look.clone();
                    let field = match part {
                        Part::Line => &mut look.dim_line_type,
                        Part::Ext => &mut look.ext_line_type,
                        Part::Value => return None,
                    };
                    if *field == kind {
                        return None;
                    }
                    *field = kind;
                    Some(DimensionEntity { look, ..d.clone() })
                })
            }
            Event::Commit(field, text) => commit(model, &field, &text),
        };
        // A value the bonds refused leaves their card with the way through, as a drag does (docs/adr/0797).
        if let Some(refused) = kentos_interaction::properties::take_bonds_refused() {
            self.bonds_refused = refused;
        }
        for text in in_saying_order(said) {
            self.warn(text);
        }
    }
}

/// A command's warnings in the order they are said: its first (its weightiest: a stair that does not fit before
/// its step rule) said last, so the status bar keeps it; the others stay in the history above it.
pub(crate) fn in_saying_order(mut said: Vec<String>) -> Vec<String> {
    said.reverse();
    said
}

#[cfg(test)]
mod saying_order_tests {
    /// UR-102: a stair edit's two warnings: “rıhta yetmiyor” is the one the status bar keeps.
    #[test]
    fn the_weightiest_warning_is_said_last() {
        let said = super::in_saying_order(vec![
            "Yürüme çizgisi (5.84 m) 20 rıhta yetmiyor: 2 rıht sığmıyor.".to_owned(),
            "2 × rıht + basamak = 0,580 m".to_owned(),
        ]);
        assert!(said.last().is_some_and(|w| w.contains("rıhta yetmiyor")));
    }
}
