//! A slab (docs/adr/0722 §6): its role, thickness, top's offset and elevation, edge and its width, material,
//! level, phase and whether Metraj sums it, each written through `cad.slabs.edit`; its area, perimeter, volume
//! and edge's length.

use super::*;

/// A slab's own rows, under the general ones: what `entity_sections` shows for it.
pub(super) fn rows(k: &Kit, sl: &kentos_commands::SlabEntity) -> Vec<Row> {
    let doc = k.doc;
    let slot = k.slot;
    let f = k.f;
    let decimals = k.decimals;
    let edit = |editor: Editor| k.edit(editor);
    let number = |field: Field| k.number(field);
    let metres = |label: &'static str, value: f64| k.metres(label, value);
    let area = |m2: f64| k.area(m2);
    let mut geo = Vec::new();
    use kentos_commands::{NewSlabEdge, SlabChange, SlabEdgeKind, SlabRole, SlabsOperation};
    let x = &sl.slab;
    let set = |c: SlabChange| Message::Properties(Event::Slab(slot, SlabsOperation::Set(c)));
    let pick = |label: String, chosen: bool, message: Message, icon: Option<&'static str>| Choice::Pick {
        label,
        swatch: None,
        icon,
        wide: false,
        chosen,
        enabled: true,
        message,
    };
    let select = |text: String, items: Vec<Choice>| {
        edit(Editor::Select {
            text,
            swatch: None,
            icon: None,
            items,
        })
    };
    let roles = SlabRole::ALL
        .iter()
        .map(|&r| pick(r.label().into(), r == x.role, set(SlabChange { role: Some(r), ..Default::default() }), Some("slab")))
        .collect();
    geo.push(Row::text("Döşeme türü", x.role.label()).editor(select(x.role.label().into(), roles)));
    // Sınır (docs/adr/0771 §4, 0791): an outline of its own, bound to its walls or to a space of its level.
    let spaces = super::super::wall_parts::space_choices(&doc.model, x);
    let bound_now = super::super::wall_parts::bound_text(x, &spaces);
    let bounds = super::super::wall_parts::bound_choices(x, &spaces)
        .into_iter()
        .map(|(label, chosen, op)| Choice::Pick {
            label,
            swatch: None,
            icon: Some("slab"),
            wide: false,
            chosen,
            enabled: op.is_some(),
            message: Message::Properties(Event::Slab(slot, op.unwrap_or(SlabsOperation::Detach))),
        })
        .collect();
    geo.push(Row::text("Sınır", bound_now.clone()).editor(select(bound_now, bounds)));
    if let Some(b) = &x.bound {
        if let Some(why) = b.broken {
            geo.push(Row::text("Kopukluk", why.words()));
        }
        if !b.uids.is_empty() {
            geo.push(Row::text("Sınırlayan duvar", b.uids.len().to_string()));
        }
    }
    geo.push(
        Row::figure("Kalınlık", f.length_bare(x.thickness))
            .unit(f.length_unit_label())
            .editor(number(Field::SlabThickness(slot))),
    );
    geo.push(
        Row::figure("Kot farkı", f.length_bare(x.offset))
            .unit(f.length_unit_label())
            .editor(number(Field::SlabOffset(slot))),
    );
    let levels = doc.settings().arch.as_ref().map(|a| a.levels.clone()).unwrap_or_default();
    let level = x.arch.level.as_deref().and_then(|id| levels.iter().find(|l| l.id == id));
    if let Some(l) = level {
        geo.push(Row::figure("Üst kot", f.length_bare(l.elevation + x.offset)).unit(f.length_unit_label()));
    }
    // Its layers (docs/adr/8040 §2): a build-up picked whole, then each layer's thickness from the top, and
    // the whole thickness.
    let build_ups = kentos_commands::slab_build_ups();
    let named = build_ups
        .iter()
        .find(|(_, l)| *l == x.layers)
        .map(|(n, _)| *n);
    let layer = |l: &kentos_commands::SlabLayer| {
        format!("{} {}", l.kind.label(), f.length_bare(l.thickness))
    };
    let summary = named.map_or_else(
        || {
            let structure = format!("Taşıyıcı {}", f.length_bare(x.thickness));
            x.layers
                .over
                .iter()
                .map(layer)
                .chain(std::iter::once(structure))
                .chain(x.layers.under.iter().map(layer))
                .collect::<Vec<_>>()
                .join(" · ")
        },
        str::to_owned,
    );
    let builds = build_ups
        .iter()
        .map(|(name, layers)| {
            pick(
                (*name).to_owned(),
                *layers == x.layers,
                set(SlabChange {
                    layers: Some(layers.clone()),
                    ..Default::default()
                }),
                Some("slab"),
            )
        })
        .collect();
    geo.push(Row::text("Katmanlar", summary.clone()).editor(select(summary, builds)));
    for (over, list) in [(true, &x.layers.over), (false, &x.layers.under)] {
        for (k, l) in list.iter().enumerate() {
            geo.push(
                Row::figure(l.kind.label(), f.length_bare(l.thickness))
                    .unit(f.length_unit_label())
                    .editor(number(Field::SlabLayer(slot, over, k))),
            );
        }
    }
    if !x.layers.is_empty() {
        geo.push(metres("Toplam kalınlık", x.whole_thickness()));
    }
    let edge_name = x.edge.as_ref().map_or("Yok", |e| e.kind.label());
    let edges = std::iter::once(None)
        .chain(SlabEdgeKind::ALL.iter().map(|k| Some(*k)))
        .map(|k| {
            pick(
                k.map_or("Yok", SlabEdgeKind::label).into(),
                k == x.edge.as_ref().map(|e| e.kind),
                set(SlabChange { edge: Some(k.map(|kind| NewSlabEdge { kind, width: None })), ..Default::default() }),
                kentos_interaction::slab_tools::edge_icon(k),
            )
        })
        .collect();
    geo.push(Row::text("Kenar", edge_name).editor(select(edge_name.into(), edges)));
    if let Some(e) = &x.edge {
        geo.push(
            Row::figure("Kenar eni", f.length_bare(e.width))
                .unit(f.length_unit_label())
                .editor(number(Field::SlabEdgeWidth(slot))),
        );
        let run = kentos_interaction::slab_tools::edge_length(x);
        if run > 0.0 {
            geo.push(metres("Kenar uzunluğu", run));
        } else {
            geo.push(Row::text("Kenar uzunluğu", "Serbest kenar yok"));
        }
    }
    geo.push(
        Row::text("Malzeme", x.material.clone().unwrap_or_default()).editor(edit(Editor::Text(Field::SlabMaterial(slot)))),
    );
    let level_name = level.map_or_else(|| x.arch.level.clone().unwrap_or_else(|| "Yok".into()), |l| l.name.clone());
    let mut level_items = vec![pick(
        "Yok".into(),
        x.arch.level.is_none(),
        set(SlabChange { level: Some(None), ..Default::default() }),
        None,
    )];
    level_items.extend(levels.iter().map(|l| {
        pick(
            l.name.clone(),
            x.arch.level.as_deref() == Some(l.id.as_str()),
            set(SlabChange { level: Some(Some(l.id.clone())), ..Default::default() }),
            None,
        )
    }));
    level_items.push(Choice::Separator);
    level_items.push(Choice::Command { label: "Katları düzenle…", icon: crate::icons::from_web(Some("archSettings")), id: "arch.settings" });
    geo.push(Row::text("Kat", level_name.clone()).editor(select(level_name, level_items)));
    let phase_name = x.arch.phase.map_or("Yok", kentos_commands::ArchPhase::label);
    let mut phases =
        vec![pick("Yok".to_owned(), x.arch.phase.is_none(), set(SlabChange { phase: Some(None), ..Default::default() }), None)];
    phases.extend(kentos_commands::ArchPhase::ALL.iter().map(|&p| {
        pick(p.label().to_owned(), x.arch.phase == Some(p), set(SlabChange { phase: Some(Some(p)), ..Default::default() }), None)
    }));
    geo.push(Row::text("Tadilat durumu", phase_name).editor(select(phase_name.to_owned(), phases)));
    let summed = if x.excluded { "Dışında" } else { "Toplamda" };
    let sums = [(false, "Toplamda"), (true, "Dışında")]
        .into_iter()
        .map(|(v, label)| pick(label.into(), v == x.excluded, set(SlabChange { excluded: Some(v), ..Default::default() }), None))
        .collect();
    geo.push(Row::text("Metraj", summed).editor(select(summed.into(), sums)));
    let a = kentos_native_application::spaces::area_of(&x.parts);
    geo.extend(area(a));
    geo.push(metres("Çevre", kentos_native_application::spaces::perimeter_of(&x.parts)));
    geo.push(Row::figure("Hacim", fixed(a * x.thickness, decimals)).unit("m³"));
    geo
}
