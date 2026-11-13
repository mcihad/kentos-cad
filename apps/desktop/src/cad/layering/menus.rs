//! The layer tree's menus: a layer's own (docs/adr/0213), its look and its data.

use super::*;
use crate::gis;

impl App {
    /// A row's context menu: what is done most at its top (make active, hide, lock, isolate, select, zoom), the rest
    /// in submenus by what it is about (Görünürlük, Görünüş, Veri, Kenet, Senaryo, Sıra), then the node itself
    /// (rename, add, copy, merge) and Sil.
    pub(crate) fn layer_menu(&self, node: &LayerNode, active: bool) -> Menu<Message> {
        let id = node.id.clone();
        let event = |e: Event| Message::Layer(e);
        let is_layer = node.kind == LayerNodeType::Layer;
        let icon = |name: &str| crate::icons::from_web(Some(name));
        let mut menu = Menu::new();
        if is_layer {
            menu = menu
                .item(
                    "Etkin katman yap",
                    // A map service layer takes no objects (docs/adr/0208 §2).
                    (!active && node.service.is_none()).then(|| event(Event::Activate(id.clone()))),
                )
                .icon(Icon::Check);
        }
        menu = menu
            .item(
                if node.visible { "Gizle" } else { "Göster" },
                Message::LayerVisible(id.clone()),
            )
            .icon(if node.visible {
                Icon::EyeOff
            } else {
                Icon::Eye
            })
            .shortcut("Boşluk")
            .item(
                if node.locked { "Kilidi aç" } else { "Kilitle" },
                Message::LayerLocked(id.clone()),
            )
            .icon(if node.locked {
                Icon::Unlock
            } else {
                Icon::Lock
            })
            .item("Yalnızca bunu göster", event(Event::Isolate(id.clone())))
            .icon(icon("layerIsolate"))
            .separator();
        // Görünürlük ▸: the whole tree's visibility, the freeze and printing (docs/adr/0311 §1, 0260 §4).
        let visibility = Menu::new()
            .item("Tüm katmanları göster", Message::Run("layer.showAll"))
            .icon(icon("layersShowAll"))
            .separator()
            .item(
                if node.frozen == Some(true) {
                    "Çöz"
                } else {
                    "Dondur"
                },
                // The active layer is never frozen.
                (node.frozen == Some(true) || !self.holds_active(node))
                    .then(|| Message::LayerFrozen(id.clone())),
            )
            .icon(icon(if node.frozen == Some(true) {
                "sun"
            } else {
                "freeze"
            }))
            .item("Bütün katmanları çöz", Message::Run("layer.thawAll"))
            .icon(icon("layersThawAll"))
            .separator()
            .item(
                if node.plot == Some(false) {
                    "Yazdır"
                } else {
                    "Yazdırma"
                },
                Message::LayerPlot(id.clone()),
            )
            .icon(Icon::Print);
        // Sıra ▸ (layer_order.rs): the node among its siblings, or out of its group.
        let in_group = self
            .document
            .as_ref()
            .and_then(|d| d.model.layers().place_of(&id))
            .is_some_and(|(parent, _)| parent.is_some());
        let order = |step| event(Event::Order(id.clone(), step));
        use crate::layer_order::Step;
        let order_menu = Menu::new()
            .item("Yukarı taşı", order(Step::Up))
            .icon(icon("moveUp"))
            .item("Aşağı taşı", order(Step::Down))
            .icon(icon("moveDown"))
            .item("En üste taşı", order(Step::Top))
            .icon(icon("bringToFront"))
            .item("En alta taşı", order(Step::Bottom))
            .icon(icon("sendToBack"))
            .separator()
            .item("Gruptan çıkar", in_group.then(|| order(Step::Out)))
            .icon(icon("layerOut"));
        // A layer drawn from a map service: its service's items, none of the objects' (docs/adr/0208 §14).
        if node.service.is_some() {
            // Its service's items where the program has map services (module_surfaces.rs).
            let menu = if self.shows_layer_entry(LayerEntry::Service) {
                self.service_menu(node, menu)
            } else {
                menu
            };
            // What the extensions add to a layer's menu (extensions/ribbon.rs, docs/adr/8057 §3).
            let menu = self.extension_menu(kentos_extensions::manifest::MenuTarget::Layer, menu);
            return menu
                .separator()
                .submenu("Görünürlük", visibility)
                .icon(Icon::Eye)
                .submenu("Sıra", order_menu)
                .icon(icon("layerOrder"))
                .separator()
                .item("Yeniden adlandır", event(Event::Rename(id.clone())))
                .icon(icon("edit"))
                .shortcut("F2")
                .separator()
                .item("Sil", event(Event::Remove(id)))
                .icon(icon("trash"))
                .shortcut("Del");
        }
        menu = menu
            .item("Nesnelerini seç", event(Event::SelectObjects(id.clone())))
            .icon(icon("select"))
            .item(
                if is_layer {
                    "Katmana yakınlaştır"
                } else {
                    "Gruba yakınlaştır"
                },
                (self.objects_below(node) > 0).then(|| event(Event::ZoomTo(id.clone()))),
            )
            .icon(icon("zoomSelection"));
        // A layer whose objects came from a service: taken again (docs/adr/0208 §10), where the program has services.
        if node.feed.is_some() && self.shows_layer_entry(LayerEntry::Service) {
            menu = menu
                .item(
                    "Yenile",
                    Message::Services(crate::services::app::Event::FeedRefresh(id.clone())),
                )
                .icon(icon("feedRefresh"));
        }
        menu = menu
            .separator()
            .submenu("Görünürlük", visibility)
            .icon(Icon::Eye);
        if is_layer {
            menu = menu
                .submenu("Görünüş", self.layer_look_menu(node))
                .icon(icon("layerStyle"))
                .submenu("Veri", self.layer_data_menu(node))
                .icon(icon("layerFields"));
            // What it holds (layer_types.rs, docs/adr/0622 §5): KentOS GIS's typed layers (module_surfaces.rs).
            if self.shows_layer_entry(LayerEntry::Kind) {
                menu = menu
                    .submenu("Katman türü", self.layer_type_menu(node))
                    .icon(crate::layer_types::glyph(node.geometry));
            }
            // Where its objects came from, and moving them (docs/adr/0583 §1–§2): a map's programs'.
            if self.shows_layer_entry(LayerEntry::Crs) {
                menu = menu
                    .submenu("Koordinat sistemi", self.layer_crs_menu(node))
                    .icon(icon("crs"));
            }
        }
        // Kaynak ▸ on a bound layer (bound.rs, docs/adr/0503), where the program has sources (module_surfaces.rs).
        if let Some(link) = self.bound_link(&id)
            && self.shows_layer_entry(LayerEntry::Source)
        {
            // A part of a table (its area): Kaynağa yaz's neighbours outside it (docs/adr/0531).
            let part = link.extent.is_some();
            let bound = |e: crate::bound::Event| Message::Bound(e);
            let mut source = Menu::new()
                .item(
                    "Kaynağa yaz",
                    bound(crate::bound::Event::Write(Some(id.clone()))),
                )
                .icon(icon("sourceWrite"))
                .item(
                    "Kaynağa birlikte yaz",
                    bound(crate::bound::Event::WriteTogether(Some(id.clone()))),
                )
                .icon(icon("sourceWrite"));
            if part {
                source = source
                    .item(
                        "Kaynağa yaz, komşular gevşek",
                        bound(crate::bound::Event::WriteLoose(Some(id.clone()))),
                    )
                    .icon(icon("sourceWrite"));
            }
            let source = source
                .item(
                    "Kaynaktan oku",
                    bound(crate::bound::Event::Read(Some(id.clone()))),
                )
                .icon(icon("sourceRead"))
                .item(
                    "Kaynakla uzlaştır…",
                    bound(crate::bound::Event::Reconcile(Some(id.clone()))),
                )
                .icon(icon("sourceReconcile"))
                .item("Kaynak durumu", bound(crate::bound::Event::Status))
                .icon(icon("sourceLinked"))
                // Anahtar alanı ▸ (docs/adr/0636 §7): the field that names a row, ticked; Yok takes it away.
                .submenu("Anahtar alanı", {
                    let mut keys = Menu::new().check(
                        "Yok",
                        link.key.is_none(),
                        bound(crate::bound::Event::Key(id.clone(), None)),
                    );
                    for f in &node.fields {
                        keys = keys.check(
                            f.name.clone(),
                            link.key.as_deref() == Some(f.name.as_str()),
                            bound(crate::bound::Event::Key(id.clone(), Some(f.name.clone()))),
                        );
                    }
                    keys
                })
                .icon(icon("sourceKey"))
                .item(
                    "Yeniden bağla…",
                    bound(crate::bound::Event::Relink(Some(id.clone()))),
                )
                .separator()
                .item(
                    "Bağı çöz",
                    bound(crate::bound::Event::Unlink(Some(id.clone()))),
                )
                .icon(icon("sourceUnlink"));
            menu = menu.submenu("Kaynak", source).icon(icon("sourceLinked"));
        }
        // Kenet ▸ (layer_snap.rs, docs/adr/0163 §4).
        if let Some(doc) = &self.document {
            menu = menu
                .submenu("Kenet", self.layer_snap_menu(doc.model.layers(), node))
                .icon(icon("magnet"));
        }
        // Senaryo ▸ on a scenario group (docs/adr/0210 §10): shown, compared, applied; KentOS GIS's (module_surfaces.rs).
        if node.scenario.is_some() && self.shows_layer_entry(LayerEntry::Scenario) {
            let time = |e: crate::temporal::Event| Message::Gis(gis::Event::Time(e));
            menu = menu
                .submenu(
                    "Senaryo",
                    Menu::new()
                        .item(
                            "Senaryoyu göster",
                            time(crate::temporal::Event::Show(id.clone())),
                        )
                        .icon(icon("scenarioShow"))
                        .item("Mevcut durum", time(crate::temporal::Event::Base))
                        .icon(icon("scenarioBase"))
                        .item(
                            "Senaryoyu karşılaştır…",
                            time(crate::temporal::Event::Compare(id.clone())),
                        )
                        .icon(icon("scenarioCompare"))
                        .item(
                            "Senaryoyu uygula…",
                            time(crate::temporal::Event::AskApply(id.clone())),
                        )
                        .icon(icon("scenarioApply")),
                )
                .icon(icon("scenario"));
        }
        menu = menu
            .submenu("Sıra", order_menu)
            .icon(icon("layerOrder"))
            .separator()
            .item("Yeniden adlandır", event(Event::Rename(id.clone())))
            .icon(icon("edit"))
            .shortcut("F2")
            .item(
                if is_layer {
                    "Yanına yeni katman"
                } else {
                    "İçine yeni katman"
                },
                event(Event::AddBeside(id.clone())),
            )
            .icon(icon("layerAdd"));
        // Kopyasını oluştur and Başka katmanlarla birleştir… (docs/adr/0177 §3).
        if is_layer {
            menu = menu
                .item("Kopyasını oluştur", event(Event::Duplicate(id.clone())))
                .icon(icon("layerDuplicate"))
                .item(
                    "Başka katmanlarla birleştir…",
                    event(Event::MergeInto(id.clone())),
                )
                .icon(icon("layerMerge"));
        }
        // What the extensions add to a layer's menu (extensions/ribbon.rs, docs/adr/8057 §3).
        let menu = self.extension_menu(kentos_extensions::manifest::MenuTarget::Layer, menu);
        menu.separator()
            // Always offered: what cannot go says why (the web's).
            .item("Sil", event(Event::Remove(id)))
            .icon(icon("trash"))
            // The web's `formatChord`: Delete is “Del”, Space “Boşluk”.
            .shortcut("Del")
    }

    /// Görünüş ▸: the layer's colour, line type and weight, its style, labels, SLD and what a change would reach.
    pub(super) fn layer_look_menu(&self, node: &LayerNode) -> Menu<Message> {
        let id = node.id.clone();
        let event = |e: Event| Message::Layer(e);
        let icon = |name: &str| crate::icons::from_web(Some(name));
        let style = &node.style;
        // One of each group, as the web's radio items.
        let types = LINE_TYPES.iter().fold(Menu::new(), |menu, (t, name)| {
            menu.radio(
                *name,
                style.line_pattern.is_none() && style.line_type == *t,
                event(Event::LineType(id.clone(), *t)),
            )
        });
        // The project's types with patterns (docs/adr/0313 §7).
        let patterns = self
            .document
            .as_ref()
            .map(|d| d.model.settings().line_types.clone())
            .unwrap_or_default();
        let types = if patterns.is_empty() {
            types
        } else {
            patterns.iter().fold(types.separator(), |menu, t| {
                menu.radio(
                    t.name.clone(),
                    style
                        .line_pattern
                        .as_deref()
                        .is_some_and(|p| kentos_commands::same_line_type_name(p, &t.name)),
                    event(Event::LinePattern(id.clone(), t.name.clone())),
                )
            })
        }
        .separator()
        .item("Çizgi tipleri…", Some(Message::Run("lineTypes.manage")));
        let weights = LINE_WEIGHTS.iter().fold(Menu::new(), |menu, w| {
            menu.radio(
                weight_text(*w),
                (style.line_weight - w).abs() < 1e-9,
                event(Event::LineWeight(id.clone(), *w)),
            )
        });
        let mut menu = Menu::new()
            .submenu("Renk", self.layer_colors(node))
            .icon(icon("color"))
            .submenu("Çizgi tipi", types)
            .icon(icon("lineType"))
            .submenu("Kalınlık", weights)
            .icon(icon("lineWeight"))
            .separator()
            .item(
                if style.renderer.is_some() {
                    "Katman stili… (özel)"
                } else {
                    "Katman stili…"
                },
                Message::LayerStyle(crate::style::layer_style::Event::Open(Some(id.clone()))),
            )
            .icon(icon("layerStyle"))
            // Etiketler (docs/adr/0212 §4): how its objects are labelled.
            .item(
                match node.style.labels.as_ref().map(|l| l.mode) {
                    Some(kentos_commands::LabelsMode::Rules) => "Etiketler… (kurallı)",
                    Some(kentos_commands::LabelsMode::Off) => "Etiketler… (etiketsiz)",
                    _ => "Etiketler…",
                },
                event(Event::Labels(id.clone())),
            )
            .icon(icon("layerLabels"))
            .separator();
        // The style files of QGIS, GeoServer and MapLibre: a map's programs' (module_surfaces.rs).
        // Stil yükle / kaydet (docs/adr/0635 §9): one file, its extension the format.
        if self.shows_layer_entry(LayerEntry::StyleFiles) {
            menu = menu
                .item("Stil yükle…", event(Event::StyleLoad(id.clone())))
                .icon(icon("styleLoad"))
                .item("Stili kaydet…", event(Event::StyleSave(id.clone())))
                .icon(icon("styleSave"))
                .separator();
        }
        // SLD (docs/adr/0516): the look out to QGIS, GeoServer, MapServer, and theirs in.
        if self.shows_layer_entry(LayerEntry::Sld) {
            menu = menu
                .item(
                    "Görünüşü SLD olarak kaydet…",
                    event(Event::SldExport(id.clone())),
                )
                .icon(icon("sldExport"))
                .item("SLD'den görünüş al…", event(Event::SldImport(id.clone())))
                .icon(icon("sldImport"));
        }
        // QML (docs/adr/0555): QGIS's own style file, out and in.
        if self.shows_layer_entry(LayerEntry::Qml) {
            menu = menu
                .item(
                    "Görünüşü QML olarak kaydet…",
                    event(Event::QmlExport(id.clone())),
                )
                .icon(icon("qmlExport"))
                .item("QML'den görünüş al…", event(Event::QmlImport(id.clone())))
                .icon(icon("qmlImport"));
        }
        // MapLibre (docs/adr/0560): the look out as a style with its GeoJSON, and a style's in.
        if self.shows_layer_entry(LayerEntry::MapLibre) {
            menu = menu
                .item(
                    "Görünüşü MapLibre stili olarak kaydet…",
                    event(Event::MaplibreExport(id.clone())),
                )
                .icon(icon("maplibreExport"))
                .item(
                    "MapLibre stilinden görünüş al…",
                    event(Event::MaplibreImport(id.clone())),
                )
                .icon(icon("maplibreImport"));
        }
        menu.separator()
            // Değişikliğin etkisi (docs/adr/0314 §4): what a change of its look would reach.
            .item(
                "Değişikliğin etkisi…",
                Message::StyleImpact(crate::style_impact::Event::Open(
                    kentos_commands::ImpactTarget::Layer,
                    id,
                )),
            )
            .icon(icon("styleImpact"))
    }

    /// Veri ▸: the layer's fields, forms and card, its filter, its time, its statistics and chart.
    pub(super) fn layer_data_menu(&self, node: &LayerNode) -> Menu<Message> {
        let id = node.id.clone();
        let event = |e: Event| Message::Layer(e);
        let icon = |name: &str| crate::icons::from_web(Some(name));
        let menu = Menu::new()
            // Alanlar (docs/adr/0199 §3): the schema of its objects' attributes.
            .item(
                if node.fields.is_empty() {
                    "Alanlar…".to_owned()
                } else {
                    format!("Alanlar… ({})", node.fields.len())
                },
                event(Event::Fields(id.clone())),
            )
            .icon(icon("layerFields"))
            // Formlar (docs/adr/0401): the layer's data entry screen.
            .item("Formlar…", event(Event::Forms(id.clone())))
            .icon(icon("forms"))
            // Bilgi kartı ve eylemler (docs/adr/0225 §4, §5): what Bilgi shows of its objects, and what they open.
            .item(
                if node.card.is_some() || !node.actions.is_empty() {
                    format!("Bilgi kartı ve eylemler… ({} eylem)", node.actions.len())
                } else {
                    "Bilgi kartı ve eylemler…".to_owned()
                },
                event(Event::Info(id.clone())),
            )
            .icon(icon("layerInfo"));
        // Veri kümesi bilgisi (docs/adr/0633): the layer as a dataset, its dictionary and lineage; a map's
        // programs' (module_surfaces.rs).
        let menu = if self.shows_layer_entry(LayerEntry::Dataset) {
            menu.item(
                if node.derivation.is_some() {
                    "Veri kümesi bilgisi… (türetilmiş)"
                } else {
                    "Veri kümesi bilgisi…"
                },
                event(Event::Dataset(id.clone())),
            )
            .icon(icon("datasetInfo"))
        } else {
            menu
        };
        let mut menu = menu
            .separator()
            // Katman süzgeci (docs/adr/0211 §4): the window, the selection's objects, taken away.
            .item(
                if node.filter.is_some() {
                    "Süzgeç… (süzgeçli)"
                } else {
                    "Süzgeç…"
                },
                event(Event::Filter(id.clone())),
            )
            .icon(icon("layerFilter"))
            .item(
                "Seçimden süzgeç",
                self.selected_on_layer(&id)
                    .then(|| event(Event::FilterFromSelection(id.clone()))),
            )
            .icon(icon("layerFilterSelection"))
            .item(
                "Süzgeci kaldır",
                node.filter
                    .is_some()
                    .then(|| event(Event::FilterClear(id.clone()))),
            )
            .icon(icon("layerFilterClear"))
            .separator();
        // Zaman ayarları (docs/adr/0210 §10): its objects' start, end and key fields; KentOS GIS's
        // (module_surfaces.rs).
        if self.shows_layer_entry(LayerEntry::Time) {
            menu = menu
                .item(
                    if node.time.is_some() {
                        "Zaman ayarları… (zamansal)"
                    } else {
                        "Zaman ayarları…"
                    },
                    Message::Gis(gis::Event::Time(crate::temporal::Event::OpenLayer(
                        id.clone(),
                    ))),
                )
                .icon(icon("timeLayer"))
                .separator();
        }
        // İstatistik and Grafik (docs/adr/0227 §8): its fields summed up, its chart; as the bottom panel's tabs.
        if self.shows_bottom_tab(crate::bottom::BottomTab::Statistics) {
            menu = menu
                .item(
                    "İstatistik",
                    Message::Gis(gis::Event::Charts(crate::charts::Event::OpenStatistics(
                        Some(id.clone()),
                    ))),
                )
                .icon(icon("fieldStatistics"));
        }
        if self.shows_bottom_tab(crate::bottom::BottomTab::Chart) {
            menu = menu
                .item(
                    "Grafik",
                    Message::Gis(gis::Event::Charts(crate::charts::Event::Open {
                        layer: Some(id),
                        kind: None,
                        spec: None,
                    })),
                )
                .icon(icon("chart"));
        }
        menu
    }
}
