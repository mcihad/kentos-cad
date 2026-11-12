//! A wall (docs/adr/0339 §6): its width, justification, offset, style, ends, height, level and phase, each
//! written through `cad.walls.edit`; its length along its path and its body's area.

use super::*;

/// A wall's own rows, under the general ones: what `entity_sections` shows for it.
pub(super) fn rows(k: &Kit, w: &kentos_commands::WallEntity) -> Vec<Row> {
    let doc = k.doc;
    let slot = k.slot;
    let f = k.f;
    let area_of = k.area_of;
    let length_of = k.length_of;
    let edit = |editor: Editor| k.edit(editor);
    let number = |field: Field| k.number(field);
    let metres = |label: &'static str, value: f64| k.metres(label, value);
    let area = |m2: f64| k.area(m2);
    let mut geo = Vec::new();
    let x = &w.wall;
    let change = |c: kentos_commands::WallChange| Message::Properties(Event::Wall(slot, c));
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
    let fixed = kentos_commands::fixed_width(&x.components).is_some();
    // A wall's width is its Kalınlık, as the tool and its panel say (docs/adr/8003); a line's weight is Çizgi kalınlığı.
    geo.push(metres("Kalınlık", x.width).editor(if fixed { None } else { number(Field::WallWidth(slot)) }));
    // Kalınlıkta sabit (docs/adr/0771 §2): the line a width or style change keeps, this session's.
    let keep_now = super::super::wall_parts::keep();
    let keeps = kentos_commands::WallReference::ALL
        .iter()
        .map(|&r| pick(r.label().to_owned(), r == keep_now, Message::Properties(Event::WallKeep(r)), None))
        .collect();
    geo.push(Row::text("Sabit kalan çizgi", keep_now.label()).editor(select(keep_now.label().to_owned(), keeps)));
    let justify = kentos_commands::WallJustify::ALL
        .iter()
        .map(|&j| {
            let icon = match j {
                kentos_commands::WallJustify::Left => "wallLeft",
                kentos_commands::WallJustify::Center => "wallCenter",
                kentos_commands::WallJustify::Right => "wallRight",
            };
            pick(
                j.label().to_owned(),
                j == x.justify,
                change(kentos_commands::WallChange {
                    justify: Some(j),
                    ..Default::default()
                }),
                Some(icon),
            )
        })
        .collect();
    geo.push(Row::text("Hiza", x.justify.label()).editor(select(x.justify.label().to_owned(), justify)));
    geo.push(metres("Öteleme", x.offset).editor(number(Field::WallOffset(slot))));
    let offered = kentos_interaction::wall::offered(doc.settings());
    let style_name = x
        .style
        .as_deref()
        .and_then(|id| offered.iter().find(|s| s.id == id))
        .map_or_else(|| kentos_interaction::wall::NO_STYLE.to_owned(), |s| s.name.clone());
    let mut styles = vec![pick(
        kentos_interaction::wall::NO_STYLE.to_owned(),
        x.style.is_none(),
        change(kentos_commands::WallChange {
            style: Some(None),
            ..Default::default()
        }),
        None,
    )];
    styles.extend(offered.iter().map(|s| {
        pick(
            s.name.clone(),
            x.style.as_deref() == Some(s.id.as_str()),
            change(kentos_commands::WallChange {
                style: Some(Some(s.id.clone())),
                ..Default::default()
            }),
            None,
        )
    }));
    styles.push(Choice::Separator);
    styles.push(Choice::Command {
        label: kentos_interaction::wall::WALL_STYLES_ENTRY,
        icon: crate::icons::from_web(Some("wallStyles")),
        id: kentos_interaction::wall::WALL_STYLES,
    });
    geo.push(Row::text("Stil", style_name.clone()).editor(select(style_name, styles)));
    let open = x.caps == kentos_commands::WallCaps::Open;
    let caps = [(false, "Kapalı"), (true, "Açık")]
        .into_iter()
        .map(|(o, l)| {
            let caps = if o { kentos_commands::WallCaps::Open } else { kentos_commands::WallCaps::Closed };
            pick(
                l.to_owned(),
                o == open,
                change(kentos_commands::WallChange {
                    caps: Some(caps),
                    ..Default::default()
                }),
                None,
            )
        })
        .collect();
    let caps_text = if open { "Açık" } else { "Kapalı" };
    geo.push(Row::text("Uçlar", caps_text).editor(select(caps_text.to_owned(), caps)));
    // Its ends' joins and its cleanup (docs/adr/0340 §6).
    let op = |o: kentos_commands::WallsOperation| Message::Properties(Event::WallOp(slot, o));
    let join_icon = |j: kentos_commands::WallJoin| match j {
        kentos_commands::WallJoin::Auto => "wallJoinAuto",
        kentos_commands::WallJoin::None => "wallJoinNone",
        kentos_commands::WallJoin::Miter => "wallJoinMiter",
        kentos_commands::WallJoin::Butt => "wallJoinButt",
        kentos_commands::WallJoin::Through => "wallJoinThrough",
    };
    if !x.closed {
        for (title, now, at_end) in [("Başlangıç birleşimi", x.joins.start, false), ("Son birleşimi", x.joins.end, true)] {
            let items = kentos_commands::WallJoin::ALL
                .iter()
                .map(|&j| {
                    let (start, end) = if at_end { (None, Some(j)) } else { (Some(j), None) };
                    pick(
                        j.label().to_owned(),
                        j == now,
                        op(kentos_commands::WallsOperation::Joins { start, end, cleanup: None, start_pass: None, end_pass: None }),
                        Some(join_icon(j)),
                    )
                })
                .collect();
            geo.push(Row::text(title, now.label()).editor(select(now.label().to_owned(), items)));
        }
        // Its ends' node decisions (docs/adr/0771 §1): the user's, kept on the end, or the priorities'.
        for (title, pass, at_end) in
            [("Baş düğümü", x.joins.start_pass.as_ref(), false), ("Son düğümü", x.joins.end_pass.as_ref(), true)]
        {
            let now = super::super::wall_parts::node_text(pass);
            let items = super::super::wall_parts::node_choices(x)
                .into_iter()
                .map(|(label, p)| {
                    let (start_pass, end_pass) = if at_end { (None, Some(p.clone())) } else { (Some(p.clone()), None) };
                    pick(
                        label.to_owned(),
                        super::super::wall_parts::node_text(p.as_ref()) == now,
                        op(kentos_commands::WallsOperation::Joins { start: None, end: None, cleanup: None, start_pass, end_pass }),
                        Some(if p.is_some() { "wallJoinButt" } else { "wallJoinAuto" }),
                    )
                })
                .collect();
            geo.push(Row::text(title, now.clone()).editor(select(now, items)));
        }
    }
    let clean = [(true, "Önceliğe göre"), (false, "Kapalı")]
        .into_iter()
        .map(|(c, l)| {
            pick(
                l.to_owned(),
                c == x.cleanup,
                op(kentos_commands::WallsOperation::Joins { start: None, end: None, cleanup: Some(c), start_pass: None, end_pass: None }),
                None,
            )
        })
        .collect();
    let clean_text = if x.cleanup {
        "Önceliğe göre"
    } else {
        "Kapalı"
    };
    geo.push(
        Row::text("Temizlik", clean_text).editor(select(clean_text.to_owned(), clean)),
    );
    // Its own; else up to the slab over it within its storey, else its storey's (docs/adr/8040 §1): the number in
    // its cell, where it comes from in a note under it (docs/adr/0687: a decorated number does not fit a
    // numeric cell at a large text size).
    let height = doc.model.wall_height(x);
    geo.push(
        Row::figure(
            "Yükseklik",
            height.map_or_else(|| "Yok".to_owned(), |h| f.length_bare(h)),
        )
        .unit(if height.is_some() {
            f.length_unit_label()
        } else {
            ""
        })
        .editor(number(Field::WallHeight(slot))),
    );
    if x.height.is_none() && height.is_some() {
        geo.push(Row::note(match doc.model.wall_top_from(x) {
            Some(kentos_domain::WallTop::Slab) => "Üst döşemenin altına kadar",
            // Docs/adr/8041 §1: a beam over it.
            Some(kentos_domain::WallTop::Beam) => "Kirişin altına kadar",
            None => "Katın yüksekliği",
        }));
    }
    let levels = doc
        .settings()
        .arch
        .as_ref()
        .map(|a| a.levels.clone())
        .unwrap_or_default();
    let level_name = x
        .arch
        .level
        .as_deref()
        .map(|id| levels.iter().find(|l| l.id == id).map_or_else(|| id.to_owned(), |l| l.name.clone()))
        .unwrap_or_else(|| "Yok".to_owned());
    let mut level_items = vec![pick(
        "Yok".to_owned(),
        x.arch.level.is_none(),
        change(kentos_commands::WallChange {
            level: Some(None),
            ..Default::default()
        }),
        None,
    )];
    level_items.extend(levels.iter().map(|l| {
        pick(
            l.name.clone(),
            x.arch.level.as_deref() == Some(l.id.as_str()),
            change(kentos_commands::WallChange {
                level: Some(Some(l.id.clone())),
                ..Default::default()
            }),
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
        change(kentos_commands::WallChange {
            phase: Some(None),
            ..Default::default()
        }),
        None,
    )];
    phases.extend(kentos_commands::ArchPhase::ALL.iter().map(|&p| {
        pick(
            p.label().to_owned(),
            x.arch.phase == Some(p),
            change(kentos_commands::WallChange {
                phase: Some(Some(p)),
                ..Default::default()
            }),
            None,
        )
    }));
    geo.push(Row::text("Tadilat durumu", phase_name).editor(select(phase_name.to_owned(), phases)));
    geo.push(metres("Uzunluk", length_of.unwrap_or(0.0)));
    geo.extend(area(area_of.unwrap_or(0.0)));
    geo
}
