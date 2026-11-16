//! A roof's layers (docs/adr/8071 §8) in their own section: each kind top down with its thickness, typed to add or set
//! it, empty or 0 to take it away; right under each layer it has, its Malzeme; their sum, the roof's thickness.

use super::*;

/// The Çatı katmanları section of a roof, each row written through `cad.arch.edit`.
pub(super) fn section(k: &Kit, x: &kentos_commands::RoofFields) -> Section {
    let f = k.f;
    let mut rows = Vec::new();
    for (i, kind) in kentos_commands::RoofLayerKind::ALL.iter().enumerate() {
        let at = u8::try_from(i).unwrap_or(u8::MAX);
        let layer = x.layers.iter().find(|l| l.kind == *kind);
        let row = match layer {
            Some(l) => Row::figure(kind.label(), f.length_bare(l.thickness)).unit(f.length_unit_label()),
            None => Row::text(kind.label(), "Yok"),
        };
        rows.push(row.editor(k.number(Field::Arch(k.slot, ArchField::RoofLayer(at)))));
        if let Some(l) = layer {
            rows.push(
                Row::text("Malzeme", l.material.clone().unwrap_or_default())
                    .editor(k.edit(Editor::Text(Field::Arch(k.slot, ArchField::RoofLayerMaterial(at))))),
            );
            // The covering's material, not given, is the roof's covering's (its IFC layer's too).
            if l.material.is_none() && *kind == kentos_commands::RoofLayerKind::Covering {
                rows.push(Row::note(&format!("Boş: çatının kaplaması, {}", x.material_name())));
            }
        }
    }
    if x.layers.is_empty() {
        rows.push(Row::note("Kalınlığı yazılan katman eklenir; 0 ya da boş katmanı kaldırır."));
    } else {
        rows.push(k.metres("Toplam kalınlık", x.thickness_m()));
    }
    Section {
        id: "roofLayers".into(),
        title: "Çatı katmanları".into(),
        rows,
    }
}
