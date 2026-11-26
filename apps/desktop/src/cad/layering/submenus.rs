//! A layer's Görünüş ▸ and Veri ▸ (docs/adr/0213): each entry where the module and the layer's kind have it
//! (layer_entries.rs, docs/adr/9195 §5).

use super::*;
use crate::gis;
use crate::layer_entries::LayerKind;

impl App {
    /// Görünüş ▸: the layer's colour, line type and weight, its style, labels, style files and what a change would
    /// reach; each where the module and the layer's `kind` have it (layer_entries.rs).
    pub(super) fn layer_look_menu(&self, node: &LayerNode, kind: LayerKind) -> Menu<Message> {
        let id = node.id.clone();
        let event = |e: Event| Message::Layer(e);
        let icon = |name: &str| crate::icons::from_web(Some(name));
        let has = |entry| self.layer_entry_on(entry, kind);
        let style = &node.style;
        let mut menu = Menu::new();
        if has(LayerEntry::Color) {
            menu = menu
                .submenu("Renk", self.layer_colors(node))
                .icon(icon("color"));
        }
        if has(LayerEntry::LineType) {
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
            menu = menu.submenu("Çizgi tipi", types).icon(icon("lineType"));
        }
        if has(LayerEntry::LineWeight) {
            let weights = LINE_WEIGHTS.iter().fold(Menu::new(), |menu, w| {
                menu.radio(
                    weight_text(*w),
                    (style.line_weight - w).abs() < 1e-9,
                    event(Event::LineWeight(id.clone(), *w)),
                )
            });
            menu = menu.submenu("Kalınlık", weights).icon(icon("lineWeight"));
        }
        menu = menu.separator();
        if has(LayerEntry::Style) {
            menu = menu
                .item(
                    if style.renderer.is_some() {
                        "Katman stili… (özel)"
                    } else {
                        "Katman stili…"
                    },
                    Message::LayerStyle(crate::style::layer_style::Event::Open(Some(id.clone()))),
                )
                .icon(icon("layerStyle"));
        }
        // Etiketler (docs/adr/0212 §4): how its objects are labelled.
        if has(LayerEntry::Labels) {
            menu = menu
                .item(
                    match node.style.labels.as_ref().map(|l| l.mode) {
                        Some(kentos_commands::LabelsMode::Rules) => "Etiketler… (kurallı)",
                        _ if !node.style.labels_on() => "Etiketler… (etiketsiz)",
                        _ => "Etiketler…",
                    },
                    event(Event::Labels(id.clone())),
                )
                .icon(icon("layerLabels"));
        }
        menu = menu.separator();
        // The style files other map programs and servers read and write.
        // Stil yükle / kaydet (docs/adr/0635 §9): one file, its extension the format.
        if has(LayerEntry::StyleFiles) {
            menu = menu
                .item("Stil yükle…", event(Event::StyleLoad(id.clone())))
                .icon(icon("styleLoad"))
                .item("Stili kaydet…", event(Event::StyleSave(id.clone())))
                .icon(icon("styleSave"))
                .separator();
        }
        // SLD (docs/adr/0516): the look out to map servers and desktop GIS, and theirs in.
        if has(LayerEntry::Sld) {
            menu = menu
                .item(
                    "Görünüşü SLD olarak kaydet…",
                    event(Event::SldExport(id.clone())),
                )
                .icon(icon("sldExport"))
                .item("SLD'den görünüş al…", event(Event::SldImport(id.clone())))
                .icon(icon("sldImport"));
        }
        // QML (docs/adr/0555): a desktop GIS's own style file, out and in.
        if has(LayerEntry::Qml) {
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
        if has(LayerEntry::MapLibre) {
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
        // Değişikliğin etkisi (docs/adr/0314 §4): what a change of its look would reach.
        if has(LayerEntry::Impact) {
            menu = menu
                .separator()
                .item(
                    "Değişikliğin etkisi…",
                    Message::StyleImpact(crate::style_impact::Event::Open(
                        kentos_commands::ImpactTarget::Layer,
                        id,
                    )),
                )
                .icon(icon("styleImpact"));
        }
        menu
    }

    /// Veri ▸: the layer's fields, forms and card, its filter, its time, its statistics and chart; each where the module
    /// and the layer's `kind` have it (layer_entries.rs).
    pub(super) fn layer_data_menu(&self, node: &LayerNode, kind: LayerKind) -> Menu<Message> {
        let id = node.id.clone();
        let event = |e: Event| Message::Layer(e);
        let icon = |name: &str| crate::icons::from_web(Some(name));
        let has = |entry| self.layer_entry_on(entry, kind);
        let mut menu = Menu::new();
        // Alanlar (docs/adr/0199 §3): the schema of its objects' attributes.
        if has(LayerEntry::Fields) {
            menu = menu
                .item(
                    if node.fields.is_empty() {
                        "Alanlar…".to_owned()
                    } else {
                        format!("Alanlar… ({})", node.fields.len())
                    },
                    event(Event::Fields(id.clone())),
                )
                .icon(icon("layerFields"));
        }
        // Formlar (docs/adr/0401): the layer's data entry screen.
        if has(LayerEntry::Forms) {
            menu = menu
                .item("Formlar…", event(Event::Forms(id.clone())))
                .icon(icon("forms"));
        }
        // Bilgi kartı ve eylemler (docs/adr/0225 §4, §5): what Bilgi shows of its objects, and what they open.
        if has(LayerEntry::Info) {
            menu = menu
                .item(
                    if node.card.is_some() || !node.actions.is_empty() {
                        format!("Bilgi kartı ve eylemler… ({} eylem)", node.actions.len())
                    } else {
                        "Bilgi kartı ve eylemler…".to_owned()
                    },
                    event(Event::Info(id.clone())),
                )
                .icon(icon("layerInfo"));
        }
        // Veri kümesi bilgisi (docs/adr/0633): the layer as a dataset, its dictionary and lineage.
        if has(LayerEntry::Dataset) {
            menu = menu
                .item(
                    if node.derivation.is_some() {
                        "Veri kümesi bilgisi… (türetilmiş)"
                    } else {
                        "Veri kümesi bilgisi…"
                    },
                    event(Event::Dataset(id.clone())),
                )
                .icon(icon("datasetInfo"));
        }
        menu = menu.separator();
        // Katman süzgeci (docs/adr/0211 §4): the window, the selection's objects, taken away.
        if has(LayerEntry::Filter) {
            menu = menu
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
        }
        // Zaman ayarları (docs/adr/0210 §10): its objects' start, end and key fields.
        if has(LayerEntry::Time) {
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
        if has(LayerEntry::Statistics) {
            menu = menu
                .item(
                    "İstatistik",
                    Message::Gis(gis::Event::Charts(crate::charts::Event::OpenStatistics(
                        Some(id.clone()),
                    ))),
                )
                .icon(icon("fieldStatistics"));
        }
        if has(LayerEntry::Chart) {
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
