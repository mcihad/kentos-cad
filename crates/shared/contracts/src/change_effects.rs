//! What a change of looks and styles does (docs/adr/0314 §2; plan/cad.md CAD-STYLE-13): one table for every host,
//! Python and an agent. Each row names the change, the command that makes it, whether it is an undo step or an edit
//! outside the steps, whether it makes the drawing unsaved, and the parts of the project it writes; a part says how
//! a database project carries it to the cloud ([`EffectPart::cloud`]). The catalog lists the rows (`effects`), and
//! the application's test runs each row's command and holds the table to what the document does.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// Whether undo takes a change back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum EffectUndo {
    /// An undo step of the drawing.
    Step,
    /// An edit of a project part outside the undo steps: the drawing is unsaved, undo does not take it back.
    Outside,
    /// Not the drawing's: the user's or the device's preference.
    None,
}

/// A part of the project a change writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum EffectPart {
    /// The drawing's objects.
    Objects,
    /// The block definitions.
    Blocks,
    /// The layer tree: its nodes' looks, renderers and flags.
    Layers,
    /// The project's settings: its styles, line types, layer states, scale and units.
    Settings,
    /// The project's style library.
    Styles,
    /// The sheet book.
    Sheets,
    /// The user's or the device's preferences, not the project's.
    Preferences,
}

impl EffectPart {
    /// How a database project carries the part to the cloud (docs/adr/0026, 0144 §5, 0251).
    pub fn cloud(self) -> &'static str {
        match self {
            EffectPart::Objects => "project.changes: nesne başına sürümle",
            EffectPart::Blocks => "project.changes: blok başına sürümle (block:<id>)",
            EffectPart::Layers => "ProjectPatch.layers: katman ağacı bütün olarak",
            EffectPart::Settings => "ProjectPatch.settings: proje ayarları bütün olarak",
            EffectPart::Styles => "ProjectPatch.styles: projenin kitaplığı bütün olarak",
            EffectPart::Sheets => "ProjectPatch.sheets: pafta kitabı bütün olarak",
            EffectPart::Preferences => "eşitlenmez: kullanıcının ya da cihazın tercihi",
        }
    }
}

/// One row of the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChangeEffect {
    pub id: &'static str,
    /// The change, Turkish.
    pub change: &'static str,
    /// The command that makes it; none for a preference.
    pub command: Option<&'static str>,
    pub undo: EffectUndo,
    /// Whether the drawing is unsaved after it.
    pub dirty: bool,
    pub parts: &'static [EffectPart],
}

use EffectPart as P;
use EffectUndo as U;

/// The table (docs/adr/0314 §2), in the order a person meets the changes.
pub const CHANGE_EFFECTS: [ChangeEffect; 17] = [
    ChangeEffect {
        id: "object-look",
        change: "Nesnenin rengi, çizgi tipi, deseni, ölçeği, kalınlığı, saydamlığı ya da sembolü",
        command: Some("cad.entities.set"),
        undo: U::Step,
        dirty: true,
        parts: &[P::Objects],
    },
    ChangeEffect {
        id: "annotative-scales",
        change: "Açıklamanın ölçekleri: ölçekli yapma, ölçek ekleme ve kaldırma, ölçekteki yer",
        command: Some("cad.annotations.scales"),
        undo: U::Step,
        dirty: true,
        parts: &[P::Objects],
    },
    ChangeEffect {
        id: "object-order",
        change: "Nesnenin çizim sırası",
        command: Some("cad.entities.order"),
        undo: U::Step,
        dirty: true,
        parts: &[P::Objects],
    },
    ChangeEffect {
        id: "match",
        change: "Özellik kopyala",
        command: Some("cad.entities.match"),
        undo: U::Step,
        dirty: true,
        parts: &[P::Objects],
    },
    ChangeEffect {
        id: "layer-look",
        change: "Katmanın rengi, çizgi tipi, deseni ya da kalınlığı",
        command: Some("cad.layers.edit"),
        undo: U::Step,
        dirty: true,
        parts: &[P::Layers],
    },
    ChangeEffect {
        id: "layer-renderer",
        change: "Katmanın işleyicisi (Katman stili)",
        command: Some("cad.layers.renderer"),
        undo: U::Step,
        dirty: true,
        parts: &[P::Layers],
    },
    ChangeEffect {
        id: "layer-flags",
        change: "Katmanın görünürlüğü, kilidi, dondurulması, yazdırılması ya da adı",
        command: Some("cad.layers.set"),
        undo: U::Outside,
        dirty: true,
        parts: &[P::Layers],
    },
    ChangeEffect {
        id: "layer-states",
        change: "Katman durumunu kaydetmek",
        command: Some("cad.layers.states"),
        undo: U::Outside,
        dirty: true,
        parts: &[P::Settings],
    },
    ChangeEffect {
        id: "annotation-styles",
        change: "Yazı ve ölçü stilleri (stile uyan nesneler aynı adımda izler)",
        command: Some("cad.styles.edit"),
        undo: U::Step,
        dirty: true,
        parts: &[P::Objects, P::Settings],
    },
    ChangeEffect {
        id: "line-types",
        change: "Çizgi tipleri ve çizgi tipi ölçeği (LTSCALE)",
        command: Some("cad.linetypes.edit"),
        undo: U::Outside,
        dirty: true,
        parts: &[P::Settings],
    },
    ChangeEffect {
        id: "standards",
        change: "Standardı uygula (katmanlar, nesnelerin görünüşü ve yazı ve ölçü stilleri)",
        command: Some("cad.standards.apply"),
        undo: U::Step,
        dirty: true,
        parts: &[P::Objects, P::Layers, P::Settings],
    },
    ChangeEffect {
        id: "library",
        change: "Projenin stil kitaplığı (sembol, çizim, şablon)",
        command: Some("cad.library.edit"),
        undo: U::Outside,
        dirty: true,
        parts: &[P::Styles],
    },
    ChangeEffect {
        id: "library-assign",
        change: "Kitaplıktan sembolü nesnelere vermek",
        command: Some("cad.library.edit"),
        undo: U::Step,
        dirty: true,
        parts: &[P::Objects],
    },
    ChangeEffect {
        id: "blocks",
        change: "Blok tanımları (ad, içerik, taban noktası, kopya, içe alma)",
        command: Some("cad.blocks.edit"),
        undo: U::Step,
        dirty: true,
        parts: &[P::Blocks],
    },
    ChangeEffect {
        id: "sheet-maps",
        change: "Paftanın haritasında katman durumu ve katmanların geçersiz kılınması",
        command: Some("cad.sheets.edit"),
        undo: U::Outside,
        dirty: true,
        parts: &[P::Sheets],
    },
    ChangeEffect {
        id: "settings",
        change: "Proje ayarları (çizim ölçeği, birimler, yazı yükseklikleri)",
        command: Some("cad.settings.set"),
        undo: U::Outside,
        dirty: true,
        parts: &[P::Settings],
    },
    ChangeEffect {
        id: "view-modes",
        change: "Görünüm kipleri: Renk kipi, Dolgular, Alan sınırları, Saydamlık, vurgu",
        command: None,
        undo: U::None,
        dirty: false,
        parts: &[P::Preferences],
    },
];

/// A row as the catalog lists it (`effects`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CatalogEffect {
    pub id: String,
    pub change: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub command: Option<String>,
    pub undo: EffectUndo,
    pub dirty: bool,
    pub parts: Vec<EffectPart>,
    /// How a database project carries each part, in the parts' order.
    pub cloud: Vec<String>,
}

impl From<&ChangeEffect> for CatalogEffect {
    fn from(e: &ChangeEffect) -> Self {
        CatalogEffect {
            id: e.id.into(),
            change: e.change.into(),
            command: e.command.map(str::to_owned),
            undo: e.undo,
            dirty: e.dirty,
            parts: e.parts.to_vec(),
            cloud: e.parts.iter().map(|p| p.cloud().to_owned()).collect(),
        }
    }
}
