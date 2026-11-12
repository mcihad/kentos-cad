//! Several objects: what they share, and their common attributes (docs/adr/0286).

use super::*;

/// Several objects: what they share, editable unless one is on a locked
/// layer, and their totals (the web's `multiSections`).
pub(super) fn many_sections(
    doc: &Document,
    objects: &[&Entity],
    (length, area): (f64, f64),
    shown: Shown,
) -> Vec<Section> {
    let ids: Vec<Slot> = objects.iter().map(|e| Slot(e.base().id)).collect();
    let first = objects[0].base();
    let same = |f: &dyn Fn(&kentos_commands::EntityBase) -> Option<&str>| {
        objects
            .iter()
            .all(|e| f(e.base()) == f(first))
            .then(|| f(first))
    };
    let layer = objects
        .iter()
        .all(|e| e.base().layer_id == first.layer_id)
        .then_some(first.layer_id.as_str());
    let color = same(&|b| b.color.as_deref());
    let symbol = same(&|b| b.symbol.as_deref());
    let layers = doc.model.layers();
    // Each layer looked up once: a large selection has many objects on few layers.
    let mut locked_layers: std::collections::HashMap<&str, bool> = std::collections::HashMap::new();
    for e in objects {
        let layer = e.base().layer_id.as_str();
        locked_layers
            .entry(layer)
            .or_insert_with(|| layers.is_locked(layer));
    }
    let is_locked = |e: &Entity| {
        locked_layers
            .get(e.base().layer_id.as_str())
            .copied()
            .unwrap_or(false)
    };
    let locked_count = objects.iter().filter(|e| is_locked(e)).count();
    // The objects on locked layers are selected but not changed (docs/adr/0278, 0286): the set commands skip
    // them and count them; the edits of their geometry and texts are given the others only.
    let all_locked = locked_count == objects.len();
    let open: std::collections::HashSet<u32> = if locked_count == 0 {
        objects.iter().map(|e| e.base().id).collect()
    } else {
        objects
            .iter()
            .filter(|e| !is_locked(e))
            .map(|e| e.base().id)
            .collect()
    };
    let any_locked = all_locked;
    let edit = |editor: Editor| (!all_locked).then_some(editor);

    // The weight of the objects drawn with lines; the others have none to show.
    let lined: Vec<&&Entity> = objects.iter().filter(|e| e.draws_lines()).collect();
    let weight = lined
        .iter()
        .all(|e| e.base().line_weight == lined[0].base().line_weight)
        .then(|| lined.first().and_then(|e| e.base().line_weight));
    let layer_text = layer.map_or_else(|| "Çeşitli".to_owned(), |l| layer_path(layers, l));
    let mut rows = vec![
        Row::text("Katman", layer_text).editor(edit(layer_editor(doc, &ids, layer))),
        Row::text("Renk", color_text(color)).editor(edit(color_editor(&ids, color))),
    ];
    if !lined.is_empty() {
        let lined_ids: Vec<Slot> = lined.iter().map(|e| Slot(e.base().id)).collect();
        rows.push(
            Row::text("Çizgi kalınlığı", weight_text(weight))
                .editor(edit(weight_editor(&lined_ids, weight))),
        );
        // The look's parts (docs/adr/0260): the shared value, else Çeşitli. Compared as their written forms were: the
        // line type and its pattern by value (no number in them), the scale by its bits, a value's written form being
        // its bits' (docs/adr/1066 §3: a million objects were each written out three times).
        let looks: Vec<_> = lined.iter().map(|e| &e.base().look).collect();
        let first = looks[0];
        let line_type = looks.iter().all(|l| l.line_type == first.line_type).then(|| first.line_type);
        let scale = looks
            .iter()
            .all(|l| {
                l.line_type_scale.map(f64::to_bits) == first.line_type_scale.map(f64::to_bits)
                    || format!("{:?}", l.line_type_scale) == format!("{:?}", first.line_type_scale)
            })
            .then(|| first.line_type_scale);
        let pattern = looks
            .iter()
            .all(|l| l.line_pattern == first.line_pattern)
            .then(|| first.line_pattern.as_deref());
        rows.push(line_type_row(doc, &lined_ids, line_type, pattern).editor_if(!any_locked));
        rows.push(line_type_scale_row(&lined_ids, scale).editor_if(!any_locked));
    }
    {
        let all: Vec<Slot> = objects.iter().map(|e| Slot(e.base().id)).collect();
        let t = objects
            .iter()
            .all(|e| e.base().look.transparency == objects[0].base().look.transparency)
            .then(|| objects[0].base().look.transparency);
        rows.push(transparency_row(&all, t).editor_if(!any_locked));
        // Areas' masks (docs/adr/0310 §5): when every object is an area, where the program has masks.
        if shown.mask && objects.iter().all(|e| matches!(e, Entity::Polygon(_))) {
            let w = objects
                .iter()
                .all(|e| e.base().look.wipeout == objects[0].base().look.wipeout)
                .then(|| objects[0].base().look.wipeout);
            rows.push(wipeout_row(&all, w).editor_if(!any_locked));
        }
    }
    rows.push(
        Row::text("Sembol", symbol_text(doc, symbol)).editor(edit(symbol_editor(doc, symbol))),
    );
    // Kat of several architectural objects, of one kind or of several (docs/adr/0717 §2): their shared level, else
    // Çeşitli; a level chosen goes to them all in one step “Kat ver”.
    let levels = doc.model.settings().arch.as_ref().map(|a| a.levels.clone()).unwrap_or_default();
    if !levels.is_empty() && objects.iter().all(|e| kentos_native_application::levels_assign::levelled(e)) {
        let first = objects[0].own_level();
        let shared = objects.iter().all(|e| e.own_level() == first);
        let name = |id: &str| levels.iter().find(|l| l.id == id).map_or_else(|| id.to_owned(), |l| l.name.clone());
        let shown = if shared { first.map_or_else(|| "Yok".to_owned(), name) } else { "Çeşitli".to_owned() };
        let items = levels
            .iter()
            .map(|l| {
                pick(
                    l.name.clone(),
                    shared && first == Some(l.id.as_str()),
                    Message::Properties(Event::Levels(ids.clone(), l.id.clone())),
                )
            })
            .collect();
        rows.push(Row::text("Kat", shown.clone()).editor(edit(Editor::Select {
            text: shown,
            swatch: None,
            icon: None,
            items,
        })));
    }
    let f = Format::of(doc.settings());
    // The vertices' elevations of the lines, polylines, areas and points (docs/adr/0142), all
    // together as one list, as the web's row takes them: the value when every vertex has the
    // same, `kot yok` when none has one, else Çeşitli. What takes no elevation is left out.
    let takers: Vec<Slot> = objects
        .iter()
        .filter(|e| elevation::takes(e) && open.contains(&e.base().id))
        .map(|e| Slot(e.base().id))
        .collect();
    if !takers.is_empty() {
        let summary = Elevations::of_objects(objects.iter().copied());
        let editor = edit(Editor::Number(Field::Elevations(takers)));
        match summary {
            Elevations::Value(_) | Elevations::None => {
                rows.extend(elevation_row("Kot", summary, &f, editor));
            }
            _ => rows.push(Row::text("Kot", "Çeşitli").editor(editor)),
        }
    }
    if locked_count > 0 {
        let line = if all_locked {
            format!("{locked_count} nesnenin hepsi kilitli katmanda: değişmez.")
        } else {
            format!("{locked_count} nesne kilitli katmanda: atlanır.")
        };
        // First: it says what every row below does to the selection.
        rows.insert(
            0,
            Row {
                warn: true,
                ..Row::note(&line)
            },
        );
    }
    let mut sections = vec![Section {
        id: "general".into(),
        title: "Ortak özellikler".into(),
        rows,
    }];
    if let Some(attrs) = shared_attributes(objects, &ids, all_locked) {
        sections.push(attrs);
    }
    // Ölçekli açıklamalar (docs/adr/0323 §4): the selection's texts, dimensions and leaders together.
    let scaled: Vec<&Entity> = objects
        .iter()
        .copied()
        .filter(|e| {
            matches!(
                e,
                Entity::Text(_) | Entity::Dimension(_) | Entity::Leader(_) | Entity::Hatch(_)
            )
        })
        .collect();
    if !scaled.is_empty() {
        let slots: Vec<Slot> = scaled
            .iter()
            .filter(|e| open.contains(&e.base().id))
            .map(|e| Slot(e.base().id))
            .collect();
        let rows = scale_rows(&scaled, &slots, slots.is_empty());
        if !rows.is_empty() {
            sections.push(Section {
                id: "scales".into(),
                title: "Ölçekli açıklama".into(),
                rows,
            });
        }
    }
    // The selection's texts: their Hiza, Genişlik çarpanı and Zemin, common or Çeşitli (docs/adr/0145 §6).
    let texts: Vec<&kentos_commands::TextEntity> = objects
        .iter()
        .filter_map(|e| match e {
            Entity::Text(t) => Some(t),
            _ => None,
        })
        .collect();
    if !texts.is_empty() {
        let slots: Vec<Slot> = texts
            .iter()
            .filter(|t| open.contains(&t.base.id))
            .map(|t| Slot(t.base.id))
            .collect();
        let height = if texts.iter().all(|t| t.height == texts[0].height) {
            Row::figure("Yükseklik", f.length_bare(texts[0].height))
        } else {
            Row::text("Yükseklik", "Çeşitli")
        }.unit(f.length_unit_label()).editor(edit(Editor::Number(Field::TextHeights(slots.clone()))));
        sections.push(Section {
            id: "texts".into(),
            title: if texts.len() == objects.len() {
                "Yazı".into()
            } else {
                format!("Yazılar ({})", texts.len()).into()
            },
            rows: [
                vec![height],
                style_rows(
                    doc.model.settings(),
                    true,
                    &texts
                        .iter()
                        .map(|t| t.face.text_style.as_ref())
                        .collect::<Vec<_>>(),
                    &slots,
                    any_locked,
                ),
                override_rows(
                    &texts
                        .iter()
                        .map(|t| t.face.overrides.as_slice())
                        .collect::<Vec<_>>(),
                    &slots,
                    any_locked,
                ),
                text_rows(&texts, &slots, any_locked),
                link_rows(&doc.model, &texts, &slots, any_locked),
            ]
            .concat(),
        });
    }
    // The selection's leaders: their note, height, turn, arrowhead and mask, common or “Çeşitli” (docs/adr/0146 §7).
    let leaders: Vec<&kentos_commands::LeaderEntity> = objects
        .iter()
        .filter_map(|e| match e {
            Entity::Leader(l) => Some(l),
            _ => None,
        })
        .collect();
    if !leaders.is_empty() {
        let slots: Vec<Slot> = leaders
            .iter()
            .filter(|l| open.contains(&l.base.id))
            .map(|l| Slot(l.base.id))
            .collect();
        sections.push(Section {
            id: "leaders".into(),
            title: if leaders.len() == objects.len() {
                "Kılavuz".into()
            } else {
                format!("Kılavuzlar ({})", leaders.len()).into()
            },
            rows: [leader_rows(&leaders, &slots, any_locked, &f), multileader_rows(&doc.model, &leaders, &slots, any_locked)].concat(),
        });
    }
    // The selection's dimensions: their Zemin, and an ordinate's axis, a slope's elevations, common or “Çeşitli” (docs/adr/0147 §7).
    let dims: Vec<&kentos_commands::DimensionEntity> = objects
        .iter()
        .filter_map(|e| match e {
            Entity::Dimension(d) => Some(d),
            _ => None,
        })
        .collect();
    if !dims.is_empty() {
        let slots: Vec<Slot> = dims
            .iter()
            .filter(|d| open.contains(&d.base.id))
            .map(|d| Slot(d.base.id))
            .collect();
        if let Some(&first) = slots.first()
            && let Some((set, _)) = doc
                .model
                .uid(first)
                .and_then(|uid| doc.model.dimension_set_of_item(uid))
            && slots.iter().all(|slot| {
                doc.model
                    .uid(*slot)
                    .and_then(|uid| doc.model.dimension_set_of_item(uid))
                    .is_some_and(|(other, _)| other.id == set.id)
            })
        {
            let mut rows = dimension_set::rows(doc, first, any_locked);
            // A chain selection can contain both rows; its first member is not the whole selection's row.
            rows.retain(|row| row.label != "Sıra");
            sections.push(Section {
                id: "dimension-set".into(),
                title: "Duvar ölçüleri".into(),
                rows,
            });
        }
        sections.push(Section {
            id: "dimensions".into(),
            title: if dims.len() == objects.len() {
                "Ölçü".into()
            } else {
                format!("Ölçüler ({})", dims.len()).into()
            },
            rows: [
                style_rows(
                    doc.model.settings(),
                    false,
                    &dims
                        .iter()
                        .map(|d| d.look.dim_style.as_ref())
                        .collect::<Vec<_>>(),
                    &slots,
                    any_locked,
                ),
                override_rows(
                    &dims
                        .iter()
                        .map(|d| d.look.overrides.as_slice())
                        .collect::<Vec<_>>(),
                    &slots,
                    any_locked,
                ),
                dimension::rows(
                    &dims,
                    &slots,
                    any_locked,
                    &f,
                    doc.model.settings().arch.as_ref().map_or(&[][..], |a| a.levels.as_slice()),
                ),
            ]
            .concat(),
        });
    }
    let mut totals = Vec::new();
    if length > 0.0 {
        totals
            .push(Row::figure("Toplam uzunluk", f.length_bare(length)).unit(f.length_unit_label()));
    }
    if area > 0.0 {
        totals.push(Row::figure("Toplam alan", f.area_bare(area)).unit(f.area_unit_label()));
    }
    if !totals.is_empty() {
        sections.push(Section {
            id: "totals".into(),
            title: "Toplamlar".into(),
            rows: totals,
        });
    }
    sections
}

/// Ortak öznitelikler of several objects (docs/adr/0286): every key any of them has, in the order first met, its
/// value when every object has that same value, else “Çeşitli” with how many have it; a cell writes the value to
/// them all through `cad.entities.set` (the locked ones skipped and counted). At most 200 keys; none, `None`.
pub(super) fn shared_attributes(
    objects: &[&Entity],
    ids: &[Slot],
    locked: bool,
) -> Option<Section> {
    const MOST: usize = 200;
    // Each key: its first value, whether every one met so far is the same, how many objects have it.
    let mut keys: Vec<(&str, &str, bool, usize)> = Vec::new();
    let mut at: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for e in objects {
        for (key, value) in &e.base().attrs {
            match at.get(key.as_str()) {
                Some(&i) => {
                    let k = &mut keys[i];
                    k.2 &= k.1 == value.as_str();
                    k.3 += 1;
                }
                None if keys.len() < MOST => {
                    at.insert(key, keys.len());
                    keys.push((key, value, true, 1));
                }
                None => {}
            }
        }
    }
    if keys.is_empty() {
        return None;
    }
    let n = objects.len();
    let rows = keys
        .into_iter()
        .map(|(key, value, same, count)| {
            let shown = if same && count == n {
                value.to_owned()
            } else if same {
                format!("Çeşitli ({count}/{n} nesnede {value})")
            } else {
                format!("Çeşitli ({count}/{n} nesnede)")
            };
            Row {
                label: Cow::Owned(key.to_owned()),
                numeric: same && count == n && looks_numeric(value),
                editor: (!locked)
                    .then(|| Editor::Text(Field::Attributes(ids.to_vec(), key.to_owned()))),
                ..Row::text("", shown)
            }
        })
        .collect();
    Some(Section {
        id: "sharedAttrs".into(),
        title: "Ortak öznitelikler".into(),
        rows,
    })
}
