//! A layer's right-click menu (docs/adr/0213), each entry placed by module and by layer kind (layer_entries.rs,
//! docs/adr/9195 §5); its Görünüş ▸ and Veri ▸ are submenus.rs's.

use super::*;
use crate::gis;

impl App {
    /// A row's context menu: what is done most at its top (make active, hide, lock, isolate, select, zoom), the rest
    /// in submenus by what it is about (Görünürlük, Görünüş, Veri, Kenet, Senaryo, Sıra), then the node itself
    /// (rename, add, copy, merge) and Sil. Each entry shows where the active module has it and the layer's kind is one
    /// it is about (layer_entries.rs, docs/adr/9195 §5): a data layer has no freeze, a service its own items, a raster's
    /// and a cloud's look is their tab's. A data layer's edit session is Düzenle's Düzenleme panel's (docs/adr/9301).
    pub fn layer_menu(&self, node: &LayerNode, active: bool) -> Menu<Message> {
        let id = node.id.clone();
        let event = |e: Event| Message::Layer(e);
        let is_layer = node.kind == LayerNodeType::Layer;
        let icon = |name: &str| crate::icons::from_web(Some(name));
        let kind = self.layer_kind(node);
        let has = |entry| self.layer_entry_on(entry, kind);
        let mut menu = Menu::new();
        if has(LayerEntry::Activate) {
            menu = menu
                .item(
                    "Etkin katman yap",
                    (!active).then(|| event(Event::Activate(id.clone()))),
                )
                .icon(Icon::Check);
        }
        if has(LayerEntry::Visibility) {
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
                .shortcut("Boşluk");
        }
        if has(LayerEntry::Lock) {
            menu = menu
                .item(
                    if node.locked { "Kilidi aç" } else { "Kilitle" },
                    Message::LayerLocked(id.clone()),
                )
                .icon(if node.locked {
                    Icon::Unlock
                } else {
                    Icon::Lock
                });
        }
        if has(LayerEntry::Visibility) {
            menu = menu
                .item("Yalnızca bunu göster", event(Event::Isolate(id.clone())))
                .icon(icon("layerIsolate"));
        }
        menu = menu.separator();
        // Görünürlük ▸: the whole tree's visibility, the freeze and printing (docs/adr/0311 §1, 0260 §4).
        let mut visibility = Menu::new();
        if has(LayerEntry::Visibility) {
            visibility = visibility
                .item("Tüm katmanları göster", Message::Run("layer.showAll"))
                .icon(icon("layersShowAll"))
                .separator();
        }
        if has(LayerEntry::Freeze) {
            visibility = visibility
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
                .separator();
        }
        if has(LayerEntry::Plot) {
            visibility = visibility
                .item(
                    if node.plot == Some(false) {
                        "Yazdır"
                    } else {
                        "Yazdırma"
                    },
                    Message::LayerPlot(id.clone()),
                )
                .icon(Icon::Print);
        }
        // Mobilde ▸ (docs/adr/9198): shown read only, edited by the mobile app (a CBS data layer's alone) or left out.
        if has(LayerEntry::Mobile) {
            let current = node.mobile.unwrap_or(kentos_commands::LayerMobile::Show);
            let mobile = self
                .mobile_choices(node)
                .iter()
                .fold(Menu::new(), |menu, m| {
                    let choice = match m {
                        kentos_launch::Mobile::Show => kentos_commands::LayerMobile::Show,
                        kentos_launch::Mobile::Edit => kentos_commands::LayerMobile::Edit,
                        kentos_launch::Mobile::Hide => kentos_commands::LayerMobile::Hide,
                    };
                    menu.radio(
                        m.title(),
                        choice == current,
                        event(Event::Mobile(id.clone(), choice)),
                    )
                });
            visibility = visibility.submenu("Mobilde", mobile);
        }
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
        if has(LayerEntry::Service) {
            menu = self.service_menu(node, menu);
        }
        if has(LayerEntry::SelectObjects) {
            menu = menu
                .item("Nesnelerini seç", event(Event::SelectObjects(id.clone())))
                .icon(icon("select"));
        }
        if has(LayerEntry::ZoomTo) {
            menu = menu
                .item(
                    if is_layer {
                        "Katmana yakınlaştır"
                    } else {
                        "Gruba yakınlaştır"
                    },
                    // A data layer frames its table's box (docs/adr/9101 §5): it holds no objects. Offered while the box
                    // is read too (T16): the click waits for it, or says why there is none.
                    (self.objects_below(node) > 0
                        || self.data_box_below(node)
                        || self.data_layer_below(node))
                    .then(|| event(Event::ZoomTo(id.clone()))),
                )
                .icon(icon("zoomSelection"));
        }
        // Dışa aktar ▸ (docs/adr/9106): the layer written to a table or a file.
        if has(LayerEntry::Export) {
            let export = |selected: bool| {
                Message::Gis(crate::gis::Event::LayerExport(
                    crate::layer_export::Event::Open(id.clone(), selected),
                ))
            };
            menu = menu
                .submenu(
                    "Dışa aktar",
                    Menu::new()
                        .item("Farklı kaydet…", export(false))
                        .icon(icon("saveAs"))
                        .item(
                            "Seçilenleri farklı kaydet…",
                            self.layer_has_selection(&id).then(|| export(true)),
                        )
                        .icon(icon("select")),
                )
                .icon(icon("export"));
        }
        // Veri kaynağına taşı… (docs/adr/9107 §8): a CBS project's drawing layer holding objects, into a table.
        if has(LayerEntry::Migrate) && self.migrates(node) {
            menu = menu
                .item(
                    "Veri kaynağına taşı…",
                    Message::Gis(crate::gis::Event::LayerMigrate(
                        crate::layer_migrate::Event::Open(id.clone()),
                    )),
                )
                .icon(icon("import"));
        }
        // Nesne sayısını göster (docs/adr/9105 §4): the layer's objects and its classes' after their names.
        if has(LayerEntry::Counts) {
            menu = menu.check(
                "Nesne sayısını göster",
                node.class_counts == Some(true),
                Message::Gis(crate::gis::Event::LayerClasses(
                    crate::layer_classes::Event::Counts(id.clone()),
                )),
            );
        }
        // A layer whose objects came from a service: taken again (docs/adr/0208 §10).
        if has(LayerEntry::Refresh) && node.feed.is_some() {
            menu = menu
                .item(
                    "Yenile",
                    Message::Services(crate::services::app::Event::FeedRefresh(id.clone())),
                )
                .icon(icon("feedRefresh"));
        }
        menu = menu.separator();
        if !visibility.is_empty() {
            menu = menu.submenu("Görünürlük", visibility).icon(Icon::Eye);
        }
        let look = self.layer_look_menu(node, kind);
        if !look.is_empty() {
            menu = menu.submenu("Görünüş", look).icon(icon("layerStyle"));
        }
        let data = self.layer_data_menu(node, kind);
        if !data.is_empty() {
            menu = menu.submenu("Veri", data).icon(icon("layerFields"));
        }
        // What it holds (layer_types.rs, docs/adr/0622 §5).
        if has(LayerEntry::Kind) {
            menu = menu
                .submenu("Katman türü", self.layer_type_menu(node))
                .icon(crate::layer_types::glyph(node.geometry));
        }
        // Where its objects came from, and moving them (docs/adr/0583 §1–§2).
        if has(LayerEntry::Crs) {
            menu = menu
                .submenu("Koordinat sistemi", self.layer_crs_menu(node))
                .icon(icon("crs"));
        }
        // Kaynak ▸ on a bound layer (bound.rs, docs/adr/0503).
        if has(LayerEntry::Source)
            && let Some(link) = self.bound_link(&id)
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
        if has(LayerEntry::Snap)
            && let Some(doc) = &self.document
        {
            menu = menu
                .submenu("Kenet", self.layer_snap_menu(doc.model.layers(), node))
                .icon(icon("magnet"));
        }
        // Senaryo ▸ on a scenario group (docs/adr/0210 §10): shown, compared, applied.
        if has(LayerEntry::Scenario) && node.scenario.is_some() {
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
        if has(LayerEntry::Order) {
            menu = menu.submenu("Sıra", order_menu).icon(icon("layerOrder"));
        }
        menu = menu.separator();
        if has(LayerEntry::Rename) {
            menu = menu
                .item("Yeniden adlandır", event(Event::Rename(id.clone())))
                .icon(icon("edit"))
                .shortcut("F2");
        }
        if has(LayerEntry::Add) {
            menu = menu
                .item(
                    if is_layer {
                        "Yanına yeni katman"
                    } else {
                        "İçine yeni katman"
                    },
                    event(Event::AddBeside(id.clone())),
                )
                .icon(icon("layerAdd"));
        }
        // Kopyasını oluştur and Başka katmanlarla birleştir… (docs/adr/0177 §3).
        if has(LayerEntry::Duplicate) {
            menu = menu
                .item("Kopyasını oluştur", event(Event::Duplicate(id.clone())))
                .icon(icon("layerDuplicate"));
        }
        if has(LayerEntry::Merge) {
            menu = menu
                .item(
                    "Başka katmanlarla birleştir…",
                    event(Event::MergeInto(id.clone())),
                )
                .icon(icon("layerMerge"));
        }
        // What the extensions add to a layer's menu (extensions/ribbon.rs, docs/adr/8057 §3): their manifests place
        // them. A group's menu gives its commands no layer.
        let mut menu = self.extension_menu(
            kentos_extensions::manifest::MenuTarget::Layer,
            is_layer.then_some(node.id.as_str()),
            menu,
        );
        if has(LayerEntry::Remove) {
            menu = menu
                .separator()
                // Always offered: what cannot go says why (the web's).
                .item("Sil", event(Event::Remove(id)))
                .icon(icon("trash"))
                // The web's `formatChord`: Delete is “Del”, Space “Boşluk”.
                .shortcut("Del");
        }
        // Katman özellikleri… last (the owner's word of 27 November 2026, docs/adr/9350): the layer's summary.
        if has(LayerEntry::Properties) && is_layer {
            menu = menu
                .separator()
                .item(
                    "Katman özellikleri…",
                    crate::layer_properties::open(node.id.clone()),
                )
                .icon(icon("info"));
        }
        menu
    }
}
