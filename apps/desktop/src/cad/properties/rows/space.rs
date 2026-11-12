//! A space (docs/adr/0342 §5): its number, name, usage, method, level, phase, height, finishes, measures and
//! boundary, each written through `cad.spaces.edit`; why it lost its walls.

use super::*;

/// A space's own rows, under the general ones: what `entity_sections` shows for it.
pub(super) fn rows(k: &Kit, sp: &kentos_commands::SpaceEntity) -> Vec<Row> {
    let doc = k.doc;
    let slot = k.slot;
    let e = k.e;
    let f = k.f;
    let decimals = k.decimals;
    let edit = |editor: Editor| k.edit(editor);
    let number = |field: Field| k.number(field);
    let metres = |label: &'static str, value: f64| k.metres(label, value);
    let area = |m2: f64| k.area(m2);
    let mut geo = Vec::new();
    use kentos_commands::{SpaceChange, SpaceColumns, SpaceMethod, SpaceScope, SpacesOperation};
    let x = &sp.space;
    let set = |c: SpaceChange| Message::Properties(Event::Space(slot, Box::new(SpacesOperation::Set(c))));
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
    let words = |label: &'static str, v: &Option<String>, field: Field| {
        Row::text(label, v.clone().unwrap_or_default()).editor(edit(Editor::Text(field)))
    };
    geo.push(words("Numara", &x.number, Field::SpaceNumber(slot)));
    geo.push(words("Ad", &x.name, Field::SpaceName(slot)));
    geo.push(words("Kullanım", &x.usage, Field::SpaceUsage(slot)));
    if let Some(arch) = &doc.settings().arch {
        for (label, values, current, is_name) in [
            ("Ad listesi", &arch.space_names, &x.name, true),
            ("Kullanım listesi", &arch.space_usages, &x.usage, false),
        ] {
            if values.is_empty() {
                continue;
            }
            let items = values
                .iter()
                .map(|value| {
                    pick(
                        value.clone(),
                        current.as_ref() == Some(value),
                        set(if is_name {
                            SpaceChange {
                                name: Some(Some(value.clone())),
                                ..Default::default()
                            }
                        } else {
                            SpaceChange {
                                usage: Some(Some(value.clone())),
                                ..Default::default()
                            }
                        }),
                        None,
                    )
                })
                .collect();
            let shown = current
                .clone()
                .filter(|v| values.contains(v))
                .unwrap_or_else(|| "Listeden seç".into());
            geo.push(Row::text(label, shown.clone()).editor(select(shown, items)));
        }
    }
    // Bölüm (docs/adr/0773 §1): the building program's group; Tasarım hedefi and its difference, read.
    let program = doc.model.program();
    if !program.units.is_empty() || x.unit.is_some() {
        let named = |u: &kentos_commands::ProgramUnit| match &u.name {
            Some(n) => format!("{} {n}", u.number),
            None => u.number.clone(),
        };
        let shown = match x.unit.as_deref() {
            Some(id) => program.unit(id).map_or_else(|| format!("{id} (programda yok)"), named),
            None => "Bölümsüz".to_owned(),
        };
        let mut items: Vec<Choice> = program
            .units
            .iter()
            .map(|u| {
                pick(
                    named(u),
                    x.unit.as_deref() == Some(u.id.as_str()),
                    set(SpaceChange { unit: Some(Some(u.id.clone())), ..Default::default() }),
                    Some("archProgram"),
                )
            })
            .collect();
        items.push(pick(
            "Bölümsüz".into(),
            x.unit.is_none(),
            set(SpaceChange { unit: Some(None), ..Default::default() }),
            None,
        ));
        geo.push(Row::text("Bölüm", shown.clone()).editor(select(shown, items)));
    }
    for (label, place) in [
        ("Tasarım hedefi", "target"),
        ("Hedef farkı", "targetDiff"),
        ("Bölümün payı", "share"),
        ("Bölümün brütü", "gross"),
    ] {
        if let Some(v) = doc.model.arch_value(e, place, None) {
            geo.push(Row::text(label, doc.model.arch_value_text(&v, true)));
        }
    }
    // Boşluk (docs/adr/0708): a shaft or an opening in the floor, drawn crossed, its area summed apart.
    let void_words = if x.void { "Evet (şaft)" } else { "Hayır (oda)" };
    let voids = [(true, "Evet (şaft)"), (false, "Hayır (oda)")]
        .into_iter()
        .map(|(v, label)| pick(label.into(), v == x.void, set(SpaceChange { void: Some(v), ..Default::default() }), None))
        .collect();
    geo.push(Row::text("Döşeme boşluğu", void_words).editor(select(void_words.into(), voids)));
    let methods = SpaceMethod::ALL
        .iter()
        .map(|&m| {
            pick(
                m.label().to_owned(),
                m == x.method,
                set(SpaceChange { method: Some(m), ..Default::default() }),
                Some(kentos_interaction::space_tools::method_icon(Some(m))),
            )
        })
        .collect();
    // Alan yöntemi, as the Mahal tool names it (docs/adr/8003).
    geo.push(Row::text("Alan yöntemi", x.method.label()).editor(select(x.method.label().to_owned(), methods)));
    if x.seed.is_some() {
        let items=SpaceColumns::ALL.iter().map(|&m|pick(m.choice().into(),m==x.columns,
            set(SpaceChange {columns:Some(m),..Default::default()}),None)).collect();
        geo.push(Row::text("Kolon alanı",x.columns.choice()).editor(select(x.columns.choice().into(),items)));
    } else {
        geo.push(Row::text("Kolon alanı","Çizilen sınır korunur"));
    }
    let levels = doc.settings().arch.as_ref().map(|a| a.levels.clone()).unwrap_or_default();
    let level_name = x
        .arch
        .level
        .as_deref()
        .map(|id| levels.iter().find(|l| l.id == id).map_or_else(|| id.to_owned(), |l| l.name.clone()))
        .unwrap_or_else(|| if x.scope==SpaceScope::Unassigned { "Atanmamış".into() }
            else if x.seed.is_some() { "Tüm katlar".into() } else { "Yok".into() });
    let mut level_items = vec![pick(
        if x.seed.is_some() { "Tüm katlar".into() } else { "Yok".into() },
        x.arch.level.is_none() && x.scope.is_project(),
        set(SpaceChange { level: Some(None), scope:Some(SpaceScope::Project), ..Default::default() }),
        None,
    )];
    if x.seed.is_some() {
        level_items.push(pick("Yalnız atanmamış".into(),x.scope==SpaceScope::Unassigned,
            set(SpaceChange {level:Some(None),scope:Some(SpaceScope::Unassigned),..Default::default()}),None));
    }
    level_items.extend(levels.iter().map(|l| {
        pick(
            l.name.clone(),
            x.arch.level.as_deref() == Some(l.id.as_str()),
            set(SpaceChange { level: Some(Some(l.id.clone())), scope:Some(SpaceScope::Project), ..Default::default() }),
            None,
        )
    }));
    level_items.push(Choice::Separator);
    level_items.push(Choice::Command {
        label: "Katları düzenle…",
        icon: crate::icons::from_web(Some("archSettings")),
        id: "arch.settings",
    });
    geo.push(Row::text("Kat", level_name.clone()).editor(select(level_name, level_items)));
    let phase_name = x.arch.phase.map_or("Yok", kentos_commands::ArchPhase::label);
    let mut phases = vec![pick(
        "Yok".to_owned(),
        x.arch.phase.is_none(),
        set(SpaceChange { phase: Some(None), ..Default::default() }),
        None,
    )];
    phases.extend(kentos_commands::ArchPhase::ALL.iter().map(|&p| {
        pick(
            p.label().to_owned(),
            x.arch.phase == Some(p),
            set(SpaceChange { phase: Some(Some(p)), ..Default::default() }),
            None,
        )
    }));
    geo.push(
        Row::text("Tadilat durumu", phase_name)
            .editor(select(phase_name.to_owned(), phases)),
    );
    // Its floor, its asma tavan and its inside height (docs/adr/8040 §3; PAİY 4(1)(ıı): from the floor's finish
    // to the ceiling's): each number in its cell, where it comes from in a note under it (docs/adr/0687: a
    // decorated number does not fit a numeric cell).
    let (floor, floor_from) = doc.model.room_floor(x);
    let storey_floor = x
        .arch
        .level
        .as_deref()
        .and_then(|id| levels.iter().find(|l| l.id == id))
        .map_or(0.0, |l| l.elevation);
    geo.push(
        Row::figure("Döşeme kot farkı", f.length_bare(floor - storey_floor))
            .unit(f.length_unit_label())
            .editor(number(Field::SpaceFloorOffset(slot))),
    );
    if floor_from == kentos_domain::FloorFrom::Slab {
        geo.push(Row::note("Altındaki döşemenin üstü"));
    }
    geo.push(Row::figure("Döşeme kotu", f.length_bare(floor)).unit(f.length_unit_label()));
    geo.push(
        Row::figure(
            "Asma tavan",
            x.suspended_ceiling
                .map_or_else(|| "Yok".to_owned(), |c| f.length_bare(c)),
        )
        .unit(if x.suspended_ceiling.is_some() {
            f.length_unit_label()
        } else {
            ""
        })
        .editor(number(Field::SpaceSuspended(slot))),
    );
    let room = doc.model.room_height(x);
    let height = room.map(|h| h.inside());
    geo.push(
        Row::figure(
            "İç yükseklik",
            height.map_or_else(|| "Yok".to_owned(), |h| f.length_bare(h)),
        )
        .unit(if height.is_some() {
            f.length_unit_label()
        } else {
            ""
        })
        .editor(number(Field::SpaceHeight(slot))),
    );
    if let Some(from) = room.and_then(|h| match h.ceiling_from {
        kentos_domain::CeilingFrom::Suspended => Some("Asma tavana kadar"),
        kentos_domain::CeilingFrom::Own => None,
        kentos_domain::CeilingFrom::Slab => Some("Üst döşemenin altına kadar"),
        kentos_domain::CeilingFrom::Storey => Some("Katlar arası"),
    }) {
        geo.push(Row::note(from));
    }
    // The finishes as the room list writes them (docs/adr/8038; PAİY 57(29)): “Döşeme”, the regulation's word
    // and the schedules', not “Zemin” (the ground's).
    geo.push(words(
        "Döşeme kaplaması",
        &x.finishes.floor,
        Field::SpaceFloor(slot),
    ));
    geo.push(words(
        "Süpürgelik",
        &x.finishes.base,
        Field::SpaceBase(slot),
    ));
    geo.push(words(
        "Duvar kaplaması",
        &x.finishes.wall,
        Field::SpaceWall(slot),
    ));
    geo.push(words(
        "Tavan kaplaması",
        &x.finishes.ceiling,
        Field::SpaceCeiling(slot),
    ));
    geo.push(words(
        "Denizlik ve parapet",
        &x.finishes.sill,
        Field::SpaceSill(slot),
    ));
    geo.push(words(
        "Basamak ve rıht",
        &x.finishes.stair,
        Field::SpaceStair(slot),
    ));
    geo.push(words(
        "Küpeşte ve korkuluk",
        &x.finishes.railing,
        Field::SpaceRailing(slot),
    ));
    let a = kentos_native_application::spaces::area_of(&x.parts);
    geo.extend(area(a));
    geo.push(metres(
        "Çevre",
        kentos_native_application::spaces::perimeter_of(&x.parts),
    ));
    if let Some(h) = height {
        geo.push(Row::figure("Hacim", fixed(a * h, decimals)).unit("m³"));
    }
    let bound = if x.seed.is_some() { "Duvarlardan" } else { "Çizilmiş" };
    let source_items=vec![Choice::Command {label:"Kaynakları seç…",icon:crate::icons::from_web(Some("space")),id:"tool.spaceSources"}];
    let source_summary: String = x.sources.as_ref().map_or_else(
        || "Tüm uygun kaynaklar".into(),
        |ids| format!("Seçilen {} kaynak", ids.len()),
    );
    geo.push(Row::text("Kaynaklar", source_summary.clone())
        .editor(select(source_summary, source_items)));
    let rebound = vec![Choice::Command {label:"Mahali yeniden sınırla…",icon:crate::icons::from_web(Some("space")),id:"tool.spaceRebound"},pick(
        "Duvarlardan yeniden bul".to_owned(),
        false,
        Message::Properties(Event::Space(slot, Box::new(SpacesOperation::Rebound { seed: None }))),
        Some("space"),
    )];
    geo.push(Row::text("Sınır", bound).editor(select(bound.to_owned(), rebound)));
    if let Some(b) = x.broken {
        match b {
            kentos_commands::SpaceBreak::LostEnclosure if !x.scope.is_project() => {
                geo.push(Row::text("Durum","Oda bulunamadı"));
                let mut note=Row::note("Seçilen kaynaklarda mahalın iç noktasını çevreleyen kullanılabilir oda bulunamadı. Son sınır korunuyor. Duvarları ve iç noktayı kontrol edin.");
                note.warn=true;
                geo.push(note);
            }
            kentos_commands::SpaceBreak::LostEnclosure => geo.push(Row::text("Durum", b.words())),
            kentos_commands::SpaceBreak::NoFreeArea | kentos_commands::SpaceBreak::InvalidBoundarySource | kentos_commands::SpaceBreak::SourceUnavailable => {
                geo.push(Row::text("Durum", if b == kentos_commands::SpaceBreak::NoFreeArea { "Alan kalmadı" } else { "Kaynak hatalı" }));
                let mut note = Row::note(b.words());
                note.warn = true;
                geo.push(note);
            }
        }
    }
    geo
}
