//! An element's type in Öznitelikler (docs/adr/8106 §5; plan/bim-plan.md BIM-12): the column's, beam's, slab's,
//! roof's, railing's or stair's project type and the version it took, the values it keeps as its own and the actions
//! on its type (Tipe bağla through the Tip list, Tipten ayır, Tipe uygula, Tipe döndür, Tipi yeniden uygula, Aynı
//! tiptekileri seç, Tipe çevir). Each of its values' rows says where the value comes from: Tipten, or Örnekten (its
//! own). Each action through `cad.elementtypes.edit`, in one undo step.

use kentos_commands::{ElementType, ElementTypeKind, Entity, fields_off_type, type_link};

use super::{Choice, Editor, Row, Section};
use crate::app::Message;
use crate::document::Document;
use crate::element_types_window::Event;

fn msg(e: Event) -> Message {
    Message::Arch(crate::arch::Event::ElementTypes(e))
}

/// The rows Öznitelikler shows a type field's value in, by their names, for each kind.
fn labels(kind: ElementTypeKind, key: &str) -> &'static [&'static str] {
    use ElementTypeKind as K;
    match (kind, key) {
        (K::Column, "member") | (K::Beam, "role" | "member") => &["Tür"],
        (K::Column, "shape") => &["Biçim"],
        (K::Column, "width") => &["En", "Çap"],
        (K::Column, "depth") => &["Boy"],
        (K::Column, "legs") => &["Alt kol kalınlığı", "Sol kol kalınlığı"],
        (K::Column | K::Beam | K::Railing, "height") => &["Yükseklik"],
        (K::Column | K::Railing, "fill" | "infill") => &["Dolgu"],
        (K::Column | K::Beam | K::Slab, "material" | "materialId") => &["Malzeme"],
        (K::Column, "type") => &["Kolon türü"],
        (K::Beam | K::Stair, "width") => &["Genişlik"],
        (K::Slab, "role") => &["Döşeme türü"],
        (K::Slab, "covering") => &["Kaplama tipi"],
        (K::Slab | K::Roof, "thickness") => &["Kalınlık"],
        (K::Slab, "edge") => &["Kenar"],
        (K::Slab, "layers") => &["Katmanlar"],
        (K::Roof, "material") => &["Kaplama"],
        (K::Roof, "gutter") => &["Oluk"],
        (K::Roof, "snowGuard") => &["Kar siperliği"],
        (K::Railing, "gap") => &["Çubuk aralığı"],
        (K::Railing, "width") => &["Küpeşte eni"],
        (K::Stair, "tread") => &["Basamak"],
        (K::Stair, "landing") => &["Sahanlık"],
        (K::Stair, "winders") => &["Balansman basamağı"],
        (K::Stair, "post") => &["Merkez direği"],
        (K::Stair, "railings") => &["Korkuluk"],
        _ => &[],
    }
}

/// The element's type and its exceptions: at its type's version the fields in which it is not the type's, at an older
/// one those it keeps.
fn typed<'d>(doc: &'d Document, e: &Entity) -> Option<(&'d ElementType, Vec<String>, bool)> {
    let link = type_link(e)?;
    let t = doc.model.element_type(&link.id)?;
    let behind = link.version < t.version;
    let own = if behind { link.overrides.clone() } else { fields_off_type(e, t) };
    Some((t, own, behind))
}

/// Each row of `rows` showing a type field's value says where it comes from: Örnekten when one of the fields it shows
/// is the element's own, else Tipten.
pub(super) fn mark(doc: &Document, e: &Entity, rows: &mut [Row]) {
    let Some((t, own, _)) = typed(doc, e) else {
        return;
    };
    for f in t.kind.fields() {
        let mine = own.iter().any(|k| k == f.key);
        for label in labels(t.kind, f.key) {
            for r in rows.iter_mut().filter(|r| !r.note && r.label == *label) {
                if mine || r.source.is_none() {
                    r.source = Some(if mine { "Örnekten" } else { "Tipten" });
                }
            }
        }
    }
}

/// The type section of an element a type serves; none for another.
pub(super) fn section(doc: &Document, e: &Entity, locked: bool) -> Option<Section> {
    let kind = ElementTypeKind::of(e)?;
    let mine = typed(doc, e);
    let open = |m: Event| (!locked).then(|| msg(m));
    let shown = mine.as_ref().map_or_else(|| "Tipsiz".to_owned(), |(t, ..)| t.name.clone());
    let mut items: Vec<Choice> = doc
        .model
        .element_types()
        .iter()
        .filter(|t| t.kind == kind)
        .map(|t| Choice::Pick {
            label: t.name.clone(),
            swatch: None,
            icon: None,
            wide: false,
            chosen: mine.as_ref().is_some_and(|(x, ..)| x.id == t.id),
            enabled: !locked,
            message: msg(Event::Assign(t.id.clone())),
        })
        .collect();
    if !items.is_empty() {
        items.push(Choice::Separator);
    }
    items.push(Choice::Action {
        label: "Tipten ayır",
        icon: "elementDetach",
        message: mine.as_ref().and_then(|_| open(Event::Detach)),
    });
    items.push(Choice::Command {
        label: "Tipe çevir…",
        icon: crate::icons::from_web(Some("elementTypeFrom")),
        id: "arch.elementTypeFrom",
    });
    items.push(Choice::Command {
        label: "Eleman tipleri…",
        icon: crate::icons::from_web(Some("elementTypes")),
        id: "arch.elementTypes",
    });
    let mut rows = vec![Row::text("Tip", shown.clone()).editor(Some(Editor::Select { text: shown, swatch: None, icon: None, items }))];
    if let (Some((t, own, behind)), Some(link)) = (&mine, type_link(e)) {
        rows.push(Row::text(
            "Tip sürümü",
            if *behind {
                format!("{} · eski (tipin sürümü {})", link.version, t.version)
            } else {
                format!("{} · güncel", link.version)
            },
        ));
        let words = if own.is_empty() {
            "Yok".to_owned()
        } else {
            own.iter().filter_map(|k| t.kind.field(k)).map(|f| f.label).collect::<Vec<_>>().join(", ")
        };
        let items = vec![
            Choice::Command {
                label: "Aynı tiptekileri seç",
                icon: crate::icons::from_web(Some("elementSameType")),
                id: "arch.elementSameType",
            },
            Choice::Separator,
            Choice::Action {
                label: "Tipe uygula",
                icon: "elementPush",
                message: (!own.is_empty()).then_some(()).and_then(|()| open(Event::Push)),
            },
            Choice::Action {
                label: "Tipe döndür",
                icon: "elementReset",
                message: (!own.is_empty() || *behind).then_some(()).and_then(|()| open(Event::Reset)),
            },
            Choice::Action {
                label: "Tipi yeniden uygula",
                icon: "elementTypes",
                message: behind.then_some(()).and_then(|()| open(Event::Refresh)),
            },
        ];
        rows.push(Row::text("İstisnalar", words.clone()).editor(Some(Editor::Select {
            text: words,
            swatch: None,
            icon: None,
            items,
        })));
    } else {
        rows.push(Row::note("Değerleri bu elemanındır. Tipe çevir ile tipi yapılır ya da Tip listesinden bir tipe bağlanır."));
    }
    Some(Section {
        id: "elementType".into(),
        title: mine.map_or_else(|| "Tip".into(), |(t, ..)| format!("Tip: {}", t.name).into()),
        rows,
    })
}
