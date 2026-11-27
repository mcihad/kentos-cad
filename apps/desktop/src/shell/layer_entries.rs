//! A layer's right-click menu by module and by layer kind (docs/adr/9195 §5, CLAUDE.md §25 rule 2): every entry of
//! cad/layering/menus.rs is a [`LayerEntry`]. A module's profile lists the entries it has (module_surfaces.rs) and
//! [`LayerEntry::kinds`] says which kinds of layer each one is about; an entry shows where both say so. kentos-cad
//! with a project no module is decided for (no type, a CAD drawing without its work) shows every entry the layer's
//! kind has. What an extension adds to the menu is placed by its manifest, not here.

use kentos_commands::{LayerNode, LayerNodeType};

use crate::app::App;

/// What a layer is, for its menu: the modules' shared kinds (docs/adr/9195 §5). A raster's and a cloud's look and
/// system are their contextual tab's.
pub use kentos_launch::LayerKind;

/// An entry of a layer's right-click menu (cad/layering/menus.rs), placed by module and by layer kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerEntry {
    /// Etkin katman yap. (A data layer's edit session is no entry: it is Düzenle's Düzenleme panel's, docs/adr/9301.)
    Activate,
    /// Gizle or Göster, Yalnızca bunu göster, Görünürlük ▸ Tüm katmanları göster.
    Visibility,
    /// Kilitle or Kilidi aç.
    Lock,
    /// Görünürlük ▸ Dondur or Çöz, Bütün katmanları çöz (docs/adr/0311 §1).
    Freeze,
    /// Görünürlük ▸ Yazdır or Yazdırma (docs/adr/0260 §4).
    Plot,
    /// Görünürlük ▸ Mobilde ▸ Göster, Düzenle, Gösterme (docs/adr/9198): the node on the mobile clients.
    Mobile,
    /// Sıra ▸ (layer_order.rs).
    Order,
    /// A map service layer's own items: Servis ayarları, Yeniden yükle, Önbelleği temizle, Servisin kapsamına
    /// yakınlaştır, Saydamlık ▸ (docs/adr/0208 §14).
    Service,
    /// Yenile on a layer whose objects came from a service (docs/adr/0208 §10).
    Refresh,
    /// Nesnelerini seç.
    SelectObjects,
    /// Katmana or Gruba yakınlaştır.
    ZoomTo,
    /// Dışa aktar ▸: the layer written to a table or a GIS file (docs/adr/9106).
    Export,
    /// Veri kaynağına taşı… (docs/adr/9107 §8).
    Migrate,
    /// Nesne sayısını göster (docs/adr/9105 §4).
    Counts,
    /// Görünüş ▸ Renk.
    Color,
    /// Görünüş ▸ Çizgi tipi.
    LineType,
    /// Görünüş ▸ Kalınlık.
    LineWeight,
    /// Görünüş ▸ Katman stili… (docs/adr/0213).
    Style,
    /// Görünüş ▸ Etiketler… (docs/adr/0212 §4).
    Labels,
    /// Görünüş ▸ Stil yükle…, Stili kaydet… (docs/adr/0635 §9).
    StyleFiles,
    /// Görünüş ▸ SLD (docs/adr/0516).
    Sld,
    /// Görünüş ▸ QML (docs/adr/0555).
    Qml,
    /// Görünüş ▸ MapLibre (docs/adr/0560).
    MapLibre,
    /// Görünüş ▸ Değişikliğin etkisi… (docs/adr/0314 §4).
    Impact,
    /// Veri ▸ Alanlar… (docs/adr/0199 §3).
    Fields,
    /// Veri ▸ Formlar… (docs/adr/0401).
    Forms,
    /// Veri ▸ Bilgi kartı ve eylemler… (docs/adr/0225 §4, §5).
    Info,
    /// Veri ▸ Veri kümesi bilgisi… (docs/adr/0633).
    Dataset,
    /// Veri ▸ Süzgeç…, Seçimden süzgeç, Süzgeci kaldır (docs/adr/0211 §4).
    Filter,
    /// Veri ▸ Zaman ayarları… (docs/adr/0210 §10).
    Time,
    /// Veri ▸ İstatistik (docs/adr/0227 §8): where the bottom panel has its tab.
    Statistics,
    /// Veri ▸ Grafik (docs/adr/0227 §8): where the bottom panel has its tab.
    Chart,
    /// Katman türü ▸ (docs/adr/0622 §5).
    Kind,
    /// Koordinat sistemi ▸: Tanımla, Dönüştür (docs/adr/0583).
    Crs,
    /// Kaynak ▸ on a bound layer (docs/adr/0503).
    Source,
    /// Kenet ▸ (docs/adr/0163 §4).
    Snap,
    /// Senaryo ▸ on a scenario group (docs/adr/0210 §10).
    Scenario,
    /// Yeniden adlandır.
    Rename,
    /// Yanına or İçine yeni katman.
    Add,
    /// Kopyasını oluştur (docs/adr/0177 §3).
    Duplicate,
    /// Başka katmanlarla birleştir… (docs/adr/0177 §3).
    Merge,
    /// Sil.
    Remove,
    /// Katman özellikleri… (docs/adr/9350): the layer's summary for reading, the menu's last entry.
    Properties,
}

impl LayerEntry {
    /// Every entry, in the menu's order.
    pub const ALL: [LayerEntry; 43] = [
        LayerEntry::Activate,
        LayerEntry::Visibility,
        LayerEntry::Lock,
        LayerEntry::Freeze,
        LayerEntry::Plot,
        LayerEntry::Mobile,
        LayerEntry::Order,
        LayerEntry::Service,
        LayerEntry::Refresh,
        LayerEntry::SelectObjects,
        LayerEntry::ZoomTo,
        LayerEntry::Export,
        LayerEntry::Migrate,
        LayerEntry::Counts,
        LayerEntry::Color,
        LayerEntry::LineType,
        LayerEntry::LineWeight,
        LayerEntry::Style,
        LayerEntry::Labels,
        LayerEntry::StyleFiles,
        LayerEntry::Sld,
        LayerEntry::Qml,
        LayerEntry::MapLibre,
        LayerEntry::Impact,
        LayerEntry::Fields,
        LayerEntry::Forms,
        LayerEntry::Info,
        LayerEntry::Dataset,
        LayerEntry::Filter,
        LayerEntry::Time,
        LayerEntry::Statistics,
        LayerEntry::Chart,
        LayerEntry::Kind,
        LayerEntry::Crs,
        LayerEntry::Source,
        LayerEntry::Snap,
        LayerEntry::Scenario,
        LayerEntry::Rename,
        LayerEntry::Add,
        LayerEntry::Duplicate,
        LayerEntry::Merge,
        LayerEntry::Remove,
        LayerEntry::Properties,
    ];

    /// The kinds of layer the entry is about (docs/adr/9195 §5). No arm stands for the rest: a new entry is placed
    /// before the program builds.
    pub fn kinds(self) -> &'static [LayerKind] {
        use LayerKind::{Annotation, Bound, Data, Drawing, Group, PointCloud, Raster, Service};
        match self {
            // What every node has: shown, ordered, printed or not, on the phone or not, renamed, removed.
            LayerEntry::Visibility
            | LayerEntry::Plot
            | LayerEntry::Mobile
            | LayerEntry::Order
            | LayerEntry::Rename
            | LayerEntry::Remove => &LayerKind::ALL,
            // A layer that takes objects: a service's takes none, a group is no layer.
            LayerEntry::Activate => &[Drawing, Annotation, Data, Bound, Raster, PointCloud],
            // A service's takes no edit either.
            LayerEntry::Lock => &[Drawing, Annotation, Data, Bound, Raster, PointCloud, Group],
            LayerEntry::Add | LayerEntry::ZoomTo => {
                &[Drawing, Annotation, Data, Bound, Raster, PointCloud, Group]
            }
            // Its rows are its table's: no freeze, and nothing of its own to select, copy or merge (a copy of a
            // data layer would be an empty drawing layer).
            LayerEntry::Freeze | LayerEntry::SelectObjects => {
                &[Drawing, Annotation, Bound, Raster, PointCloud, Group]
            }
            LayerEntry::Duplicate | LayerEntry::Merge => {
                &[Drawing, Annotation, Bound, Raster, PointCloud]
            }
            LayerEntry::Service => &[Service],
            // A layer whose objects a service fed.
            LayerEntry::Refresh => &[Drawing],
            // Its objects into a table: the annotation stays in the project file (docs/adr/9196 §2).
            LayerEntry::Migrate => &[Drawing],
            // The objects' colour, the CAD's line type and weight; a raster's and a cloud's look is their tab's.
            LayerEntry::Color | LayerEntry::Impact => &[Drawing, Annotation, Data, Bound],
            LayerEntry::LineType | LayerEntry::LineWeight => &[Drawing, Annotation, Bound],
            // A feature layer's look, labels and data: the annotation is texts already, its look its objects' own.
            LayerEntry::Style
            | LayerEntry::Labels
            | LayerEntry::StyleFiles
            | LayerEntry::Sld
            | LayerEntry::Qml
            | LayerEntry::MapLibre
            | LayerEntry::Export
            | LayerEntry::Counts
            | LayerEntry::Fields
            | LayerEntry::Forms
            | LayerEntry::Info
            | LayerEntry::Dataset
            | LayerEntry::Filter
            | LayerEntry::Time
            | LayerEntry::Statistics
            | LayerEntry::Chart => &[Drawing, Data, Bound],
            // A table's type and system are its own.
            LayerEntry::Kind => &[Drawing, Annotation],
            LayerEntry::Crs => &[Drawing, Annotation, Bound],
            LayerEntry::Source => &[Bound],
            // The drawing tools snap to a data layer's rows too (docs/adr/9103 §5); nothing snaps to a raster or a
            // cloud.
            LayerEntry::Snap => &[Drawing, Annotation, Data, Bound, Group],
            LayerEntry::Scenario => &[Group],
            // A layer's summary: a group has no source, style or fields of its own.
            LayerEntry::Properties => &[Drawing, Annotation, Data, Bound, Service, Raster, PointCloud],
        }
    }
}

impl App {
    /// What `node` is, for its menu: the shared answer (`kentos_native_application::layer_kind`, docs/adr/9197 §2.4)
    /// every client and the CBS project's save ask.
    pub fn layer_kind(&self, node: &LayerNode) -> LayerKind {
        match &self.document {
            Some(doc) => kentos_native_application::layer_kind::of(&doc.model, node),
            None if node.kind == LayerNodeType::Group => LayerKind::Group,
            None => LayerKind::Drawing,
        }
    }

    /// `raster` or `pointcloud` when `layer` holds those alone: its first object decides unless it is one of them,
    /// so a large layer of lines costs one object (the active layer's contextual tab asks the same, modes.rs).
    pub(crate) fn layer_holds_alone(&self, layer: &str) -> Option<&'static str> {
        kentos_native_application::layer_kind::alone(&self.document.as_ref()?.model, layer)
    }

    /// Whether a layer of `kind`'s menu has `entry` here: the active module has it and it is about that kind.
    pub fn layer_entry_on(&self, entry: LayerEntry, kind: LayerKind) -> bool {
        entry.kinds().contains(&kind) && self.shows_layer_entry(entry)
    }
}
