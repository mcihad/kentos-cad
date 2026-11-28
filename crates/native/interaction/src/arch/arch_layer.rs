//! Katman anahtarı (docs/adr/8008).
//!
//! A Mimari object goes on its kind's layer, named by the 2026 Dijital Hazırlama Yönetmeliği's EK-4
//! (`kentos_contracts::ek4`). A brick wall goes on MM-DUVA-TUGLA, a door on MM-KAPI-GENEL, a dimension on MM-XXXX-OLCU.
//! Before, every Mimari object went on the active layer: in the 2+1 flat all 54 were on "Çizim".
//!
//! The layer opens with the first object written to it, in that object's undo step; if the object is refused, the layer
//! goes too. A layer of the same name already in the drawing is used, whatever its id (an imported DXF's). With the key
//! off (kentos-cad; KentOS Mimari's Katman › Etkin katman), objects go on the active layer as before.

use kentos_commands::{EK4_GENERAL, LayerStyle, LineType, WallStyleDef, ek4_layer};
use kentos_domain::{NewLayer, default_style};

use crate::log::Level;
use crate::tool::Context;

/// What a Mimari object is, for its layer: EK-2 Tablo 2.3's element and EK-4 Tablo 4.1's third field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A wall, by its core's material (Tuğla); none: GENEL.
    Wall(Option<String>),
    Door,
    Window,
    /// A wall opening of no door or window (Boşluk).
    Void,
    /// A recess cut from a wall's face (Niş).
    Niche,
    /// A cavity inside a wall (Baca): EK-2's own element.
    Flue,
    Space,
    /// A slab, by its material (Betonarme); none: GENEL.
    Slab(Option<String>),
    /// A raft (docs/adr/8172 §6): a slab of the foundation, EK-2 Tablo 2.3's Temel (TEML), MM-TEML-RADYE.
    Raft,
    /// A facade moulding (docs/adr/8173): EK-2 Tablo 2.3's Cephe (CEPH), its type SILME.
    Moulding,
    /// A coping (docs/adr/8173), by its material (Taş): EK-2 Tablo 2.3's Harpuşta (HARP); none: GENEL.
    Coping(Option<String>),
    /// A column. KentOS keeps no column material yet (plan M41): GENEL.
    Column,
    Stair,
    Ramp,
    /// A beam (docs/adr/8041 §1): EK-2 Tablo 2.3 has no Kiriş; it is a Taşıyıcı (TASI), its type KIRIS. A lintel and a
    /// bond beam (docs/adr/8041 §3) are EK-2's own: Lento (LENT), Hatıl (HATL).
    Beam(kentos_commands::BeamRole),
    /// A roof (docs/adr/8044 §1), by its covering (Kiremit): EK-2 Tablo 2.3's Çatı (CATI).
    Roof(Option<String>),
    /// A free-standing railing (docs/adr/8045 §3), by its infill: EK-2 Tablo 2.3's Korkuluk (KORK), MM-KORK-CUBUK.
    Railing(kentos_commands::RailingInfill),
    /// An elevator's block (docs/adr/8045 §5): EK-2 Tablo 2.3's Asansör (ASAN), MM-ASAN-GENEL.
    Elevator,
    /// A curtain wall (docs/adr/8070 §7): EK-2 Tablo 2.3's Giydirme Cephe (GIYC), MM-GIYC-GENEL.
    CurtainWall,
    /// A post and a rail (docs/adr/8070 §8): EK-2 Tablo 2.3 has neither; Diğer (DIGR), MM-DIGR-DIKME and MM-DIGR-KAYIT.
    Post,
    Rail,
    /// A covering (docs/adr/8070 §10), by its type's EK-2 element (Asma Tavan, Zemin Kaplama …): MM-ASTV-GENEL.
    Covering(&'static str),
    CeilingDevice,
    Grid,
    /// Kitchen units (Mutfak).
    Casework,
    /// A fixture or furniture block (Tefriş), by its type (Mobilya, Vitrifiye; docs/adr/8024); none: GENEL.
    Furniture(Option<String>),
    Dimension,
    Tag,
    /// Kot, kuzey oku, kesit işareti.
    Symbol,
    /// A legend of the drawing's symbols and materials (EK-4 4.17.6).
    Legend,
    Schedule,
    /// The building's shadow on the ground (docs/adr/8177): EK-2 Tablo 2.3 has none, GENL.
    Shadow,
}

impl Kind {
    /// A wall of `style`: named by its core's material.
    pub fn wall(style: Option<&WallStyleDef>) -> Self {
        Kind::Wall(style.and_then(wall_material))
    }

    /// EK-2 Tablo 2.3's element code; an annotation of no element is Genel (XXXX).
    pub fn element(&self) -> &'static str {
        match self {
            Kind::Wall(_) | Kind::Void | Kind::Niche => "DUVA",
            Kind::Flue => "BACA",
            Kind::Door => "KAPI",
            Kind::Window => "PENC",
            Kind::Slab(_) => "DOSM",
            Kind::Raft => "TEML",
            Kind::Moulding => "CEPH",
            Kind::Coping(_) => "HARP",
            Kind::Column => "KOLN",
            Kind::Stair => "MERD",
            Kind::Ramp => "RAMP",
            Kind::Beam(kentos_commands::BeamRole::Beam) => "TASI",
            Kind::Beam(kentos_commands::BeamRole::Lintel) => "LENT",
            Kind::Beam(kentos_commands::BeamRole::BondBeam) => "HATL",
            // EK-2 Tablo 2.3: Temel (docs/adr/8172).
            Kind::Beam(kentos_commands::BeamRole::PadFooting | kentos_commands::BeamRole::StripFooting) => "TEML",
            Kind::Roof(_) => "CATI",
            Kind::Railing(_) => "KORK",
            Kind::Elevator => "ASAN",
            Kind::CurtainWall => "GIYC",
            Kind::Post | Kind::Rail => "DIGR",
            Kind::Covering(element) => element,
            Kind::CeilingDevice => "ASTV",
            Kind::Grid => "AKSS",
            Kind::Casework | Kind::Furniture(_) => "TEFR",
            Kind::Space
            | Kind::Dimension
            | Kind::Tag
            | Kind::Symbol
            | Kind::Legend
            | Kind::Schedule
            | Kind::Shadow => EK4_GENERAL,
        }
    }

    /// EK-4 Tablo 4.1's third field, malzeme/tip/gösterim: a cut element's material (4.14.3), else its type or what it
    /// shows; GENEL when it has none.
    pub fn detail(&self) -> String {
        let word = |w: &str| w.to_owned();
        match self {
            Kind::Wall(m) | Kind::Slab(m) | Kind::Roof(m) | Kind::Furniture(m) | Kind::Coping(m) => {
                m.clone().unwrap_or_default()
            }
            Kind::Void => word("BOSLUK"),
            Kind::Niche => word("NIS"),
            Kind::Casework => word("MUTFAK"),
            Kind::Space => word("MAHAL"),
            Kind::Dimension => word("OLCU"),
            Kind::Tag => word("ETIKET"),
            Kind::Symbol => word("SEMBOL"),
            Kind::Legend => word("LEJANT"),
            Kind::Schedule => word("CIZELGE"),
            Kind::Shadow => word("GOLGE"),
            Kind::Beam(kentos_commands::BeamRole::Beam) => word("KIRIS"),
            Kind::Beam(kentos_commands::BeamRole::PadFooting) => word("TEKIL"),
            Kind::Beam(kentos_commands::BeamRole::StripFooting) => word("SUREKLI"),
            Kind::Raft => word("RADYE"),
            Kind::Moulding => word("SILME"),
            Kind::Beam(_) => String::new(),
            Kind::Post => word("DIKME"),
            Kind::Rail => word("KAYIT"),
            Kind::Covering(_) => String::new(),
            Kind::CeilingDevice => word("CIHAZ"),
            Kind::Railing(infill) => word(infill.layer_word()),
            Kind::Door
            | Kind::Window
            | Kind::Flue
            | Kind::Column
            | Kind::Stair
            | Kind::Ramp
            | Kind::Elevator
            | Kind::CurtainWall
            | Kind::Grid => String::new(),
        }
    }

    /// The layer's EK-4 name: MM-DUVA-TUGLA.
    pub fn layer_name(&self) -> String {
        ek4_layer(self.element(), &self.detail())
    }

    /// The layer's id in the tree when KentOS opens it: its name in lower case (mm-duva-tugla).
    pub fn layer_id(&self) -> String {
        self.layer_name().to_ascii_lowercase()
    }

    /// An opening of `form` (Kapı, Pencere, Boşluk, Niş, Baca).
    pub fn opening(form: kentos_commands::OpeningForm) -> Self {
        use kentos_commands::OpeningForm;
        match form {
            OpeningForm::Door => Kind::Door,
            OpeningForm::Window => Kind::Window,
            OpeningForm::Opening => Kind::Void,
            OpeningForm::Niche => Kind::Niche,
            OpeningForm::Flue => Kind::Flue,
        }
    }

    /// The undo step an object of the kind is written in when its layer opens with it: the command's own name for it.
    pub fn step(&self) -> &'static str {
        match self {
            Kind::Wall(_) => "Duvar",
            Kind::Door => "Kapı",
            Kind::Window => "Pencere",
            Kind::Void => "Boşluk",
            Kind::Niche => "Niş",
            Kind::Flue => "Baca",
            Kind::Space => "Mahal",
            Kind::Slab(_) => "Döşeme",
            Kind::Raft => "Radye",
            Kind::Moulding => "Cephe silmesi",
            Kind::Coping(_) => "Duvar harpuştası",
            Kind::Column => "Kolon",
            Kind::Stair => "Merdiven",
            Kind::Ramp => "Rampa",
            Kind::Beam(role) => role.label(),
            Kind::Roof(_) => "Çatı",
            Kind::Railing(_) => "Korkuluk",
            Kind::Elevator => "Asansör",
            Kind::CurtainWall => "Giydirme cephe",
            Kind::Post => "Dikme",
            Kind::Rail => "Kayıt",
            Kind::Covering(_) => "Kaplama",
            Kind::CeilingDevice => "Tavan cihazı",
            Kind::Grid => "Aks",
            Kind::Casework => "Mutfak",
            Kind::Furniture(_) => "Donatı",
            Kind::Dimension => "Ölçü",
            Kind::Tag => "Etiket",
            Kind::Symbol => "İşaret",
            Kind::Legend => "Lejant",
            Kind::Schedule => "Çizelge",
            Kind::Shadow => "Gölge",
        }
    }

    /// The object in a sentence: “duvar”, “kapı”.
    pub fn what(&self) -> &'static str {
        match self {
            Kind::Wall(_) => "duvar",
            Kind::Door => "kapı",
            Kind::Window => "pencere",
            Kind::Void => "boşluk",
            Kind::Niche => "niş",
            Kind::Flue => "baca",
            Kind::Space => "mahal",
            Kind::Slab(_) => "döşeme",
            Kind::Raft => "radye",
            Kind::Moulding => "cephe silmesi",
            Kind::Coping(_) => "duvar harpuştası",
            Kind::Column => "kolon",
            Kind::Stair => "merdiven",
            Kind::Ramp => "rampa",
            Kind::Beam(kentos_commands::BeamRole::Beam) => "kiriş",
            Kind::Beam(kentos_commands::BeamRole::Lintel) => "lento",
            Kind::Beam(kentos_commands::BeamRole::BondBeam) => "hatıl",
            Kind::Beam(kentos_commands::BeamRole::PadFooting) => "tekil temel",
            Kind::Beam(kentos_commands::BeamRole::StripFooting) => "sürekli temel",
            Kind::Roof(_) => "çatı",
            Kind::Railing(_) => "korkuluk",
            Kind::Elevator => "asansör",
            Kind::CurtainWall => "giydirme cephe",
            Kind::Post => "dikme",
            Kind::Rail => "kayıt",
            Kind::Covering(_) => "kaplama",
            Kind::CeilingDevice => "tavan cihazı",
            Kind::Grid => "aks",
            Kind::Casework => "mutfak dolabı",
            Kind::Furniture(_) => "donatı",
            Kind::Dimension => "ölçü",
            Kind::Tag => "etiket",
            Kind::Symbol => "işaret",
            Kind::Legend => "lejant",
            Kind::Schedule => "çizelge",
            Kind::Shadow => "gölge",
        }
    }

    /// The layer's look when KentOS opens it: what is cut drawn heavy, the rest light, axes dash-dot red (EK-4 4.29),
    /// dimensions as the new project's Ölçü. Pens per cut and seen lines come with plan M10.
    pub fn style(&self) -> LayerStyle {
        let look = |color: &str, weight: f64| LayerStyle {
            color: color.to_owned(),
            line_weight: weight,
            ..default_style()
        };
        match self {
            Kind::Wall(_) | Kind::Column | Kind::Post => look("ink", 0.5),
            Kind::Slab(_) | Kind::Raft | Kind::Roof(_) | Kind::Moulding | Kind::Coping(_) => look("ink", 0.35),
            Kind::Covering(_) | Kind::CeilingDevice => look("ink", 0.18),
            Kind::Door
            | Kind::Window
            | Kind::Void
            | Kind::Niche
            | Kind::Flue
            | Kind::Stair
            | Kind::Ramp
            | Kind::Railing(_)
            | Kind::Elevator
            | Kind::CurtainWall
            | Kind::Rail
            | Kind::Beam(_) => look("ink", 0.25),
            Kind::Grid => LayerStyle {
                line_type: LineType::Dashdot,
                ..look("#E06C75", 0.18)
            },
            Kind::Casework | Kind::Furniture(_) => look("fg-dim", 0.18),
            Kind::Dimension => look("#56B6C2", 0.18),
            Kind::Space | Kind::Tag | Kind::Legend | Kind::Schedule | Kind::Shadow => look("fg", 0.18),
            Kind::Symbol => look("fg", 0.25),
        }
    }
}

/// KentOS's own styles whose name of material is not their core's: a drywall partition is named by its boards.
const STYLE_MATERIALS: &[(&str, &str)] = &[("kentos-bolme-10", "Alçıpan")];

/// The material a wall of `style` is named by: the component first in priority (the core; KentOS's styles keep the
/// masonry and concrete at 1, insulation at 3, finishes at 5), its material or else its name. A drywall partition of
/// KentOS's is ALCIPAN. None for a style of no components.
pub fn wall_material(style: &WallStyleDef) -> Option<String> {
    if let Some((_, m)) = STYLE_MATERIALS.iter().find(|(id, _)| *id == style.id) {
        return Some((*m).to_owned());
    }
    style
        .components
        .iter()
        .min_by_key(|c| c.priority.unwrap_or(u8::MAX))
        .map(|c| c.material.clone().unwrap_or_else(|| c.name.clone()))
        .filter(|m| !m.trim().is_empty())
}

/// The drawing's layer of `kind`'s name, by id, else by name (case aside): an imported drawing's MM-DUVA-TUGLA.
fn existing(kind: &Kind, cx: &Context<'_>) -> Option<String> {
    let layers = cx.doc.layers();
    let id = kind.layer_id();
    if layers.get(&id).is_some() {
        return Some(id);
    }
    let name = kind.layer_name();
    layers
        .leaves()
        .into_iter()
        .find(|l| l.name.eq_ignore_ascii_case(&name))
        .map(|l| l.id.clone())
}

/// The layer an object of `kind` goes on as the tool stands, for a preview: its layer when the key is on and the
/// drawing has it, else the active one (the key's layer opens when the object is written).
pub fn planned(kind: &Kind, cx: &Context<'_>) -> String {
    if cx.memory.layer_keys
        && let Some(id) = existing(kind, cx)
    {
        return id;
    }
    cx.doc.layers().active().to_owned()
}

/// The panel's Katman: the layer's name an object of `kind` goes on (opened with the first one when missing), or the
/// active layer's with the key off.
pub fn shown(kind: &Kind, cx: &Context<'_>) -> String {
    if cx.memory.layer_keys {
        return existing(kind, cx)
            .and_then(|id| cx.doc.layers().get(&id).map(|l| l.name.clone()))
            .unwrap_or_else(|| kind.layer_name());
    }
    let active = cx.doc.layers().active();
    cx.doc
        .layers()
        .get(active)
        .map_or_else(|| active.to_owned(), |l| l.name.clone())
}

/// The layer a tool with its own layer choice (Duvar ölçüleri's Ölçü katmanı) starts on: `kind`'s with the key on (its
/// id when the drawing has it yet, else the id it opens with), else the active one.
pub fn chosen_at_start(kind: &Kind, cx: &Context<'_>) -> String {
    if cx.memory.layer_keys {
        return existing(kind, cx).unwrap_or_else(|| kind.layer_id());
    }
    cx.doc.layers().active().to_owned()
}

/// The choice a tool's layer menu adds for `kind`'s layer while the drawing lacks it: “MM-XXXX-OLCU (ilk ölçü
/// yazılınca açılır)”. None with the key off or the layer there.
pub fn missing_choice(kind: &Kind, cx: &Context<'_>) -> Option<(String, String)> {
    (cx.memory.layer_keys && existing(kind, cx).is_none()).then(|| {
        (
            kind.layer_id(),
            format!(
                "{} (ilk {} yazılınca açılır)",
                kind.layer_name(),
                kind.what()
            ),
        )
    })
}

/// Writes on `layer`, a tool's own choice: `kind`'s layer still to open is opened as [`writing`] does; any other is
/// written on as it is.
pub fn writing_on(
    kind: &Kind,
    layer: &str,
    cx: &mut Context<'_>,
    write: impl FnOnce(&str, &mut Context<'_>) -> bool,
) -> bool {
    if layer == kind.layer_id() && cx.doc.layers().get(layer).is_none() {
        let keyed = cx.memory.layer_keys;
        cx.memory.layer_keys = true;
        let done = writing(kind, cx, write);
        cx.memory.layer_keys = keyed;
        return done;
    }
    write(layer, cx)
}

/// The revision a command's preview was made at, carried over the layer [`writing`] opened just before it: the
/// drawing changed only by that, so the preview still holds. `expected` taken at `before`; else as it was.
pub fn carried(expected: Option<String>, before: u64, cx: &Context<'_>) -> Option<String> {
    match expected {
        Some(e) if e == before.to_string() => Some(cx.doc.revision().to_string()),
        other => other,
    }
}

/// Writes an object of `kind` with `write`, which gets the layer's id and says whether it wrote. With the key on, the
/// layer is the kind's, opened first when the drawing lacks it, in the object's undo step; the object refused, the
/// layer goes too. With the key off, the active layer.
pub fn writing(
    kind: &Kind,
    cx: &mut Context<'_>,
    write: impl FnOnce(&str, &mut Context<'_>) -> bool,
) -> bool {
    if !cx.memory.layer_keys {
        let active = cx.doc.layers().active().to_owned();
        return write(&active, cx);
    }
    if let Some(id) = existing(kind, cx) {
        return write(&id, cx);
    }
    let id = kind.layer_id();
    let name = kind.layer_name();
    let group = cx.doc.begin_group(kind.step());
    let layer = NewLayer {
        id: Some(id.clone()),
        style: kind.style(),
        ..NewLayer::layer(name.clone())
    };
    // The rooms' layer stays under the others (docs/adr/8176 §3.4): a colour plan fills the rooms where their layer is,
    // over what lies on a layer under it. A new kind's layer opens over the rooms' when the drawing has it at the top.
    let rooms = (*kind != Kind::Space)
        .then(|| {
            let name = Kind::Space.layer_name();
            cx.doc.layers().leaves().into_iter().find(|l| l.name == name).map(|l| l.id.clone())
        })
        .flatten()
        .and_then(|id| cx.doc.layers().place_of(&id))
        .and_then(|(group, at)| group.is_none().then_some(at));
    let added = match rooms {
        Some(at) => cx.doc.add_layer_at(layer, None, Some(at), None, false),
        None => cx.doc.add_layer(layer, None, false),
    };
    if let Err(refusal) = added {
        cx.doc.cancel_group(group);
        cx.say(Level::Warn, refusal.to_string());
        return false;
    }
    if write(&id, cx) {
        cx.doc.end_group(group);
        cx.say(
            Level::Info,
            format!("“{name}” katmanı {} için açıldı (EK-4).", kind.what()),
        );
        true
    } else {
        cx.doc.cancel_group(group);
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_commands::{builtin_wall_styles, is_ek4_layer};

    #[test]
    fn every_kind_has_an_ek4_name() {
        let kinds = [
            Kind::Wall(Some("Tuğla".into())),
            Kind::Wall(None),
            Kind::Door,
            Kind::Window,
            Kind::Void,
            Kind::Niche,
            Kind::Flue,
            Kind::Space,
            Kind::Slab(Some("Betonarme".into())),
            Kind::Column,
            Kind::Stair,
            Kind::Ramp,
            Kind::Grid,
            Kind::Casework,
            Kind::Furniture(Some("Vitrifiye".into())),
            Kind::Dimension,
            Kind::Tag,
            Kind::Symbol,
            Kind::Legend,
            Kind::Schedule,
            Kind::Shadow,
            Kind::Beam(kentos_commands::BeamRole::Beam),
            Kind::Beam(kentos_commands::BeamRole::Lintel),
            Kind::Beam(kentos_commands::BeamRole::BondBeam),
            Kind::Roof(Some("Kiremit".into())),
            Kind::Roof(None),
            Kind::Railing(kentos_commands::RailingInfill::Balusters),
            Kind::Railing(kentos_commands::RailingInfill::Glass),
            Kind::Railing(kentos_commands::RailingInfill::Solid),
            Kind::Elevator,
            Kind::CurtainWall,
            Kind::Post,
            Kind::Rail,
            Kind::Covering("ASTV"),
            // The foundations (docs/adr/8172): EK-2's Temel, by their kind.
            Kind::Beam(kentos_commands::BeamRole::PadFooting),
            Kind::Beam(kentos_commands::BeamRole::StripFooting),
            Kind::Raft,
            Kind::Moulding,
            Kind::Coping(Some("Taş".into())),
        ];
        let names: Vec<String> = kinds.iter().map(Kind::layer_name).collect();
        for n in &names {
            assert!(is_ek4_layer(n), "{n}");
        }
        assert_eq!(names[0], "MM-DUVA-TUGLA");
        assert_eq!(names[1], "MM-DUVA-GENEL");
        assert_eq!(names[2], "MM-KAPI-GENEL");
        assert_eq!(names[5], "MM-DUVA-NIS");
        assert_eq!(names[6], "MM-BACA-GENEL");
        assert_eq!(names[7], "MM-XXXX-MAHAL");
        assert_eq!(names[8], "MM-DOSM-BETONARME");
        assert_eq!(names[13], "MM-TEFR-MUTFAK");
        assert_eq!(names[14], "MM-TEFR-VITRIFIYE");
        assert_eq!(names[15], "MM-XXXX-OLCU");
        assert_eq!(names[20], "MM-TASI-KIRIS");
        // A lintel and a bond beam (docs/adr/8041 §3): EK-2's own elements.
        assert_eq!(
            (names[21].as_str(), names[22].as_str()),
            ("MM-LENT-GENEL", "MM-HATL-GENEL")
        );
        // A roof (docs/adr/8044 §1): EK-2's Çatı, by its covering.
        assert_eq!(
            (names[23].as_str(), names[24].as_str()),
            ("MM-CATI-KIREMIT", "MM-CATI-GENEL")
        );
        // A railing (docs/adr/8045 §3): EK-2's Korkuluk, by its infill; an elevator's block (§5) its Asansör; a curtain
        // wall (docs/adr/8070 §7) its Giydirme Cephe; a post and a rail (§8) Diğer; the foundations (docs/adr/8172)
        // Temel.
        assert_eq!(
            &names[25..],
            [
                "MM-KORK-CUBUK",
                "MM-KORK-CAM",
                "MM-KORK-DOLU",
                "MM-ASAN-GENEL",
                "MM-GIYC-GENEL",
                "MM-DIGR-DIKME",
                "MM-DIGR-KAYIT",
                "MM-ASTV-GENEL",
                "MM-TEML-TEKIL",
                "MM-TEML-SUREKLI",
                "MM-TEML-RADYE",
                "MM-CEPH-SILME",
                "MM-HARP-TAS"
            ]
        );
        assert_eq!(Kind::Wall(Some("Tuğla".into())).layer_id(), "mm-duva-tugla");
    }

    #[test]
    fn kentos_wall_styles_are_named_by_their_core() {
        let named: Vec<(String, String)> = builtin_wall_styles()
            .iter()
            .map(|s| (s.name.clone(), Kind::wall(Some(s)).layer_name()))
            .collect();
        let want = [
            ("Dış duvar 25", "MM-DUVA-TUGLA"),
            ("İç duvar 15", "MM-DUVA-TUGLA"),
            ("Gazbeton 20", "MM-DUVA-GAZBETON"),
            ("Betonarme perde 25", "MM-DUVA-BETONARME"),
            ("Bölme duvar 10", "MM-DUVA-ALCIPAN"),
        ];
        for ((name, layer), (want_name, want_layer)) in named.iter().zip(want) {
            assert_eq!((name.as_str(), layer.as_str()), (want_name, want_layer));
        }
        assert_eq!(Kind::wall(None).layer_name(), "MM-DUVA-GENEL");
    }
}
