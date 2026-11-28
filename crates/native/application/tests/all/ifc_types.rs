//! IFC-04's first acceptance (docs/adr/8105; plan/ifc-uyum.md IFC-04): the declared section building's wall styles
//! written each as one `IfcWallType`, every wall of a style typed by it, its layers in order with their thickness. The
//! file is kept (fixtures/arch/v1/ifc/kesit-binasi-tipli.ifc) with the styles as the drawing says them
//! (kesit-binasi-tipli.json), for the independent reader (scripts/fixtures/ifc_types_check.py). `KENTOS_WRITE_IFC_TYPES=1`
//! writes them again. The note carries the layered slabs as the drawing says them too, for
//! scripts/fixtures/slab_layers_check.py (docs/adr/8166).

use std::path::PathBuf;

use kentos_commands::Entity;
use kentos_native_application::arch_ifc;
use serde_json::json;

#[test]
fn every_wall_style_is_one_type_and_its_walls_are_typed_by_it() {
    let doc = crate::ifc_required::declared();
    let (b, _) = arch_ifc::building(&doc, "2026-11-17T12:00:00", "KentOS Mimari");
    // The drawing's walls by style.
    let mut by_style: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for e in doc.entities() {
        if let Entity::Wall(w) = e {
            *by_style
                .entry(w.wall.style.clone().unwrap_or_default())
                .or_default() += 1;
        }
    }
    let styled: usize = by_style
        .iter()
        .filter(|(s, _)| !s.is_empty())
        .map(|(_, n)| n)
        .sum();
    assert!(styled > 0, "{by_style:?}");
    assert_eq!(
        b.types.len(),
        by_style.keys().filter(|s| !s.is_empty()).count()
    );
    assert_eq!(
        b.types.iter().map(|t| t.members.len()).sum::<usize>(),
        styled
    );
    for t in &b.types {
        assert_eq!(t.entity, "IFCWALLTYPE");
        assert!(
            t.members
                .iter()
                .all(|&i| b.elements[i].class == kentos_ifc::building::Class::Wall)
        );
        let (_, layers) = t.layer_set.as_ref().expect("its layers");
        assert!(!layers.is_empty() && layers.iter().all(|l| l.thickness > 0.0));
    }
    let text = kentos_ifc::building::write(&b);
    assert_eq!(text.matches("IFCWALLTYPE(").count(), b.types.len());
    assert_eq!(text.matches("IFCRELDEFINESBYTYPE(").count(), b.types.len());
    assert_eq!(text.matches("IFCRELDECLARES(").count(), 1);
    // The styles as the drawing says them, beside the file: each its name, its walls' tags (their ids) and its
    // components' names, materials and thicknesses at its own width.
    let styles: Vec<serde_json::Value> = doc
        .settings()
        .wall_styles
        .iter()
        .chain(kentos_commands::builtin_wall_styles().iter())
        .filter(|s| by_style.contains_key(&s.id))
        .fold(Vec::new(), |mut out: Vec<serde_json::Value>, s| {
            if out.iter().any(|o| o["id"] == s.id.as_str()) {
                return out;
            }
            let width = s.new_width();
            let thick = kentos_commands::component_thicknesses(width, &s.components).unwrap_or_default();
            let walls: Vec<String> = doc
                .entities_with_uids()
                .filter_map(|(u, e)| match e {
                    Entity::Wall(w) if w.wall.style.as_deref() == Some(s.id.as_str()) => Some(u.to_string()),
                    _ => None,
                })
                .collect();
            out.push(json!({
                "id": s.id,
                "name": doc.wall_style_name(Some(&s.id)),
                "walls": walls,
                "layers": s.components.iter().zip(thick).map(|(c, t)| json!({"name": c.name, "material": c.material, "thickness": t})).collect::<Vec<_>>(),
            }));
            out
        });
    // The layered slabs as the drawing says them (docs/adr/8166): each its id, its top over its storey and its layers
    // from the top, the structure among them, each its kind's word, its material and its thickness.
    let slabs: Vec<serde_json::Value> = doc
        .entities_with_uids()
        .filter_map(|(u, e)| match e {
            Entity::Slab(sl) if sl.slab.covering.is_none() && !sl.slab.layers.is_empty() => {
                let s = &sl.slab;
                let layer = |l: &kentos_commands::SlabLayer| {
                    json!({"name": l.kind.label(), "material": l.material, "thickness": l.thickness})
                };
                let layers: Vec<serde_json::Value> = s
                    .layers
                    .over
                    .iter()
                    .map(layer)
                    .chain(std::iter::once(
                        json!({"name": "Taşıyıcı", "material": s.material, "thickness": s.thickness}),
                    ))
                    .chain(s.layers.under.iter().map(layer))
                    .collect();
                Some(json!({"id": u.to_string(), "top": s.offset, "layers": layers}))
            }
            _ => None,
        })
        .collect();
    assert!(
        !slabs.is_empty(),
        "the declared section building's slabs are layered"
    );
    let kept = json!({"file": "kesit-binasi-tipli.ifc", "styles": styles, "slabs": slabs});
    let note = serde_json::to_string_pretty(&kept).unwrap() + "\n";
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/arch/v1/ifc");
    if std::env::var_os("KENTOS_WRITE_IFC_TYPES").is_some() {
        std::fs::write(dir.join("kesit-binasi-tipli.ifc"), &text).unwrap();
        std::fs::write(dir.join("kesit-binasi-tipli.json"), &note).unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(dir.join("kesit-binasi-tipli.ifc")).expect("the kept file"),
        text
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("kesit-binasi-tipli.json")).expect("the kept note"),
        note
    );
}
