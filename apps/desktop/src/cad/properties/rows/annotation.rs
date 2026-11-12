//! Texts, dimensions, tables, leaders and hatches: their scales, styles and own rows.

use super::*;

/// Ölçekli, Kendi ölçeği, Diğer ölçekler and Ayrı değerler (docs/adr/0323 §4): whether the texts, dimensions or
/// leaders are annotative, the scale their own geometry is drawn at, their other scales (one that sets something
/// apart with an asterisk; typed to add or remove) and Sıfırla; none when none of them can be (a text along a curve,
/// a linked text).
pub(super) fn scale_rows(objects: &[&Entity], slots: &[Slot], locked: bool) -> Vec<Row> {
    use kentos_commands::{annotative_of, why_not_annotative};
    if objects.iter().all(|e| why_not_annotative(e).is_some()) {
        return Vec::new();
    }
    let on = objects.iter().filter(|e| annotative_of(e).is_some()).count();
    let state = match on {
        0 => "Hayır",
        n if n == objects.len() => "Evet",
        _ => "Çeşitli",
    };
    let pick = |label: &str, value: bool| Choice::Pick {
        label: label.to_owned(),
        swatch: None,
        icon: None,
        wide: false,
        chosen: state == label,
        enabled: true,
        message: Message::Properties(Event::Annotative(slots.to_vec(), value)),
    };
    let editor = Editor::Select {
        text: state.to_owned(),
        swatch: None,
        icon: None,
        items: vec![pick("Evet", true), pick("Hayır", false)],
    };
    let mut rows = vec![Row::text("Ölçekli", state).editor((!locked).then_some(editor))];
    if on != objects.len() {
        return rows;
    }
    let common = |texts: Vec<String>| {
        if texts.iter().all(|t| *t == texts[0]) {
            texts[0].clone()
        } else {
            "Çeşitli".to_owned()
        }
    };
    let own = common(
        objects
            .iter()
            .filter_map(|e| annotative_of(e))
            .map(|a| kentos_interaction::annotative::ratio(a.at))
            .collect(),
    );
    let others = common(
        objects
            .iter()
            .filter_map(|e| kentos_interaction::annotative::others_text(e))
            .collect(),
    );
    rows.push(Row::text("Kendi ölçeği", own));
    rows.push(
        Row::text("Diğer ölçekler", others)
            .editor((!locked).then_some(Editor::Text(Field::AnnotativeScales(slots.to_vec())))),
    );
    let apart: Vec<String> = objects
        .iter()
        .filter_map(|e| annotative_of(e))
        .flat_map(|a| a.scales.iter().filter(|s| !s.is_plain()).map(|s| s.scale))
        .fold(Vec::<f64>::new(), |mut all, s| {
            if !all.contains(&s) {
                all.push(s);
            }
            all
        })
        .into_iter()
        .map(kentos_interaction::annotative::ratio)
        .collect();
    if !apart.is_empty() {
        let text = apart.join(", ");
        let editor = Editor::Select {
            text: text.clone(),
            swatch: None,
            icon: None,
            items: vec![Choice::Pick {
                label: "Sıfırla".to_owned(),
                swatch: None,
                icon: None,
                wide: false,
                chosen: false,
                enabled: true,
                message: Message::Properties(Event::AnnotativeReset(slots.to_vec())),
            }],
        };
        rows.push(Row::text("Ayrı değerler", text).editor((!locked).then_some(editor)));
    }
    rows
}

/// Öznitelikler's Yazı stili or Ölçü stili row (docs/adr/0183 §6), a CAD
/// project's: the style the objects follow by name (a link to a style the
/// project no longer has shows Standart), or “Çeşitli”; choosing one applies
/// it to them in one step “Değiştir”. On a locked layer it only shows. The
/// web's `ui/properties/styleRows.ts`.
/// Stilden ayrılan (docs/adr/0322 §5): the style fields the objects set apart by hand, common or “Çeşitli”, and
/// Stile döndür; none when no object sets one apart.
pub(super) fn override_rows(lists: &[&[String]], slots: &[Slot], locked: bool) -> Vec<Row> {
    if lists.iter().all(|l| l.is_empty()) {
        return Vec::new();
    }
    let text = if lists.iter().all(|l| *l == lists[0]) {
        kentos_interaction::properties::override_names(lists[0]).join(", ")
    } else {
        "Çeşitli".to_owned()
    };
    let editor = Editor::Select {
        text: text.clone(),
        swatch: None,
        icon: None,
        items: vec![Choice::Pick {
            label: "Stile döndür".to_owned(),
            swatch: None,
            icon: None,
            wide: false,
            chosen: false,
            enabled: true,
            message: Message::Properties(Event::RestoreStyle(slots.to_vec())),
        }],
    };
    vec![Row::text("Stilden ayrılan", text).editor((!locked).then_some(editor))]
}

/// Tablo stili ▾ (docs/adr/1127 §3): the table style the table follows (Yok: none, its own look), chosen among the
/// project's; Tablo stilleri… opens their window. None in a project without table styles whose table follows none.
pub(super) fn table_style_row(
    settings: &kentos_commands::ProjectSettings,
    t: &kentos_commands::TableEntity,
    slots: &[Slot],
    locked: bool,
) -> Vec<Row> {
    if settings.table_styles.is_empty() && t.table_style.is_none() {
        return Vec::new();
    }
    let none = "Yok";
    let value = match t.table_style.as_deref() {
        None => none.to_owned(),
        Some(id) => settings
            .table_styles
            .iter()
            .find(|s| s.id == id)
            .map_or_else(|| format!("{id} (projede yok)"), |s| s.name.clone()),
    };
    let pick = |label: String, id: Option<String>| Choice::Pick {
        chosen: value == label,
        label,
        swatch: None,
        icon: None,
        wide: false,
        enabled: true,
        message: Message::Properties(Event::TableStyle(slots.to_vec(), id)),
    };
    let items = std::iter::once(pick(none.to_owned(), None))
        .chain(settings.table_styles.iter().map(|s| pick(s.name.clone(), Some(s.id.clone()))))
        .chain(std::iter::once(Choice::Action {
            label: "Tablo stilleri…",
            icon: "settings",
            message: Some(Message::TableStyleDefs(crate::table_style_window::Event::Open)),
        }))
        .collect();
    let editor = Editor::Select { text: value.clone(), swatch: None, icon: None, items };
    vec![Row::text("Tablo stili", value).editor((!locked).then_some(editor))]
}

pub(super) fn style_rows(
    settings: &kentos_commands::ProjectSettings,
    text: bool,
    ids: &[Option<&String>],
    slots: &[Slot],
    locked: bool,
) -> Vec<Row> {
    if !kentos_interaction::styles::shown(settings) || ids.is_empty() {
        return Vec::new();
    }
    let styles: Vec<(String, String)> = if text {
        settings
            .text_styles
            .iter()
            .map(|s| (s.id.clone(), s.name.clone()))
            .collect()
    } else {
        settings
            .dimension_styles
            .iter()
            .map(|s| (s.id.clone(), s.name.clone()))
            .collect()
    };
    let name_of = |id: Option<&String>| {
        id.and_then(|id| styles.iter().find(|(i, _)| i == id))
            .map_or(kentos_commands::STANDARD_STYLE.to_owned(), |(_, n)| {
                n.clone()
            })
    };
    let names: Vec<String> = ids.iter().map(|id| name_of(*id)).collect();
    let value = if names.iter().all(|n| *n == names[0]) {
        names[0].clone()
    } else {
        "Çeşitli".to_owned()
    };
    let pick = |label: String, id: Option<String>| Choice::Pick {
        chosen: value == label,
        label,
        swatch: None,
        icon: None,
        wide: false,
        enabled: true,
        message: Message::Properties(if text {
            Event::TextStyle(slots.to_vec(), id)
        } else {
            Event::DimensionStyle(slots.to_vec(), id)
        }),
    };
    let items = std::iter::once(pick(kentos_commands::STANDARD_STYLE.to_owned(), None))
        .chain(
            styles
                .iter()
                .map(|(id, name)| pick(name.clone(), Some(id.clone()))),
        )
        .collect();
    let editor = Editor::Select {
        text: value.clone(),
        swatch: None,
        icon: None,
        items,
    };
    vec![
        Row::text(if text { "Yazı stili" } else { "Ölçü stili" }, value)
            .editor((!locked).then_some(editor)),
    ]
}

/// A value that reads as a number: `12`, `-3,5` (the web's `/^-?\d+([.,]\d+)?$/`).
/// A text's Hiza, Genişlik çarpanı, Kutu genişliği, Satır aralığı (docs/adr/0182
/// §4) and Zemin rows (docs/adr/0145 §6), for one
/// text or the texts of a selection: their common value, or “Çeşitli”. A new
/// alignment keeps each text where it is; the texts are written in one step
/// “Değiştir”. On a locked layer they only show. The web's `textRows`.
pub(super) fn text_rows(
    texts: &[&kentos_commands::TextEntity],
    slots: &[Slot],
    locked: bool,
) -> Vec<Row> {
    const MIXED: &str = "Çeşitli";
    // Every value below is written from a text's width factor and paragraph only: a text whose two are the first's
    // writes the first's, so only the others are written out (docs/adr/1066 §3; a large selection's texts mostly share
    // them; equal by value, ±0 aside, which no width or spacing takes). The text written decides, as before.
    let first_input = (texts[0].width_factor.map(f64::to_bits), &texts[0].paragraph);
    let as_first: Vec<bool> = texts
        .iter()
        .map(|t| t.width_factor.map(f64::to_bits) == first_input.0 && t.paragraph == *first_input.1)
        .collect();
    let common = |of: &dyn Fn(&kentos_commands::TextEntity) -> String| {
        let first = of(texts[0]);
        texts
            .iter()
            .zip(&as_first)
            .all(|(t, same)| *same || of(t) == first)
            .then_some(first)
    };
    let align = texts
        .iter()
        .all(|t| t.align == texts[0].align)
        .then_some(texts[0].align);
    let factor = common(&|t| text::width_factor_text(t.width_factor.unwrap_or(1.0)));
    let mask = texts
        .iter()
        .all(|t| t.mask == texts[0].mask)
        .then_some(texts[0].mask);
    let on_off = |on: bool| if on { "Açık" } else { "Kapalı" };
    let align_text = align.map_or_else(|| MIXED.to_owned(), |a| text::align_label(a).to_owned());
    let mask_text = mask.map_or(MIXED, on_off).to_owned();
    let edit = |editor: Editor| (!locked).then_some(editor);
    let aligns = Editor::Select {
        text: align_text.clone(),
        swatch: None,
        icon: align.map(text::align_icon),
        items: text::ALIGNS
            .iter()
            .map(|&(a, _, label, icon)| Choice::Pick {
                label: label.to_owned(),
                swatch: None,
                icon: Some(icon),
                wide: false,
                chosen: align == Some(a),
                enabled: true,
                message: Message::Properties(Event::TextAlign(slots.to_vec(), a)),
            })
            .collect(),
    };
    let masks = Editor::Select {
        text: mask_text.clone(),
        swatch: None,
        icon: None,
        items: [true, false]
            .into_iter()
            .map(|on| Choice::Pick {
                label: on_off(on).to_owned(),
                swatch: None,
                icon: None,
                wide: false,
                chosen: mask == Some(on),
                enabled: true,
                message: Message::Properties(Event::TextMask(slots.to_vec(), on)),
            })
            .collect(),
    };
    let factor_row = match factor {
        Some(value) => Row::figure("Genişlik çarpanı", value),
        None => Row::text("Genişlik çarpanı", MIXED),
    };
    // A multi-line text's box (none: “Kutusuz”) and line spacing (docs/adr/0182 §4).
    let box_row = match common(&|t| match t.paragraph.box_width {
        Some(w) => crate::crs::js_number(fixed_number(w, 3)),
        None => "Kutusuz".to_owned(),
    }) {
        Some(value) if value == "Kutusuz" => Row::text("Kutu genişliği", value),
        Some(value) => Row::figure("Kutu genişliği", value),
        None => Row::text("Kutu genişliği", MIXED),
    };
    let spacing_row = match common(&|t| {
        crate::crs::js_number(fixed_number(t.paragraph.line_spacing.unwrap_or(1.0), 4))
    }) {
        Some(value) => Row::figure("Satır aralığı", value),
        None => Row::text("Satır aralığı", MIXED),
    };
    // Its indents, tab stops and columns (docs/adr/0324 §3), the indents and gap in its height.
    let layout = |t: &kentos_commands::TextEntity| t.paragraph.layout.as_deref().cloned().unwrap_or_default();
    let heights = |label: &'static str, value: Option<String>| match value {
        Some(v) => Row::figure(label, v).unit("× yükseklik"),
        None => Row::text(label, MIXED),
    };
    let number = |v: f64| crate::crs::js_number(fixed_number(v, 3));
    let indent_row = heights("Girinti", common(&|t| number(layout(t).indent.unwrap_or(0.0))));
    let left_row = heights("Sol girinti", common(&|t| number(layout(t).left_indent.unwrap_or(0.0))));
    let tabs_row = Row::text(
        "Sekmeler",
        common(&|t| kentos_interaction::text_layout::tabs_text(&layout(t).tabs)).unwrap_or_else(|| MIXED.to_owned()),
    );
    let columns = common(&|t| layout(t).columns.map_or(1, |c| c.count).to_string());
    let columns_row = match &columns {
        Some(n) => Row::figure("Sütunlar", n.clone()),
        None => Row::text("Sütunlar", MIXED),
    };
    let mut rows = vec![
        Row::text("Hiza", align_text).editor(edit(aligns)),
        factor_row.editor(edit(Editor::Number(Field::TextWidth(slots.to_vec())))),
        box_row.editor(edit(Editor::Number(Field::TextBox(slots.to_vec())))),
        spacing_row.editor(edit(Editor::Number(Field::TextSpacing(slots.to_vec())))),
        indent_row.editor(edit(Editor::Number(Field::TextIndent(slots.to_vec())))),
        left_row.editor(edit(Editor::Number(Field::TextLeftIndent(slots.to_vec())))),
        tabs_row.editor(edit(Editor::Text(Field::TextTabs(slots.to_vec())))),
        columns_row.editor(edit(Editor::Number(Field::TextColumns(slots.to_vec())))),
    ];
    // In columns: their height, metres, and the gap between them.
    if columns.as_deref().is_some_and(|n| n != "1") {
        let tall = common(&|t| layout(t).columns.map_or(String::new(), |c| number(c.height)));
        rows.push(
            match tall {
                Some(v) => Row::figure("Sütun yüksekliği", v).unit("m"),
                None => Row::text("Sütun yüksekliği", MIXED),
            }
            .editor(edit(Editor::Number(Field::TextColumnHeight(slots.to_vec())))),
        );
        rows.push(
            heights("Sütun aralığı", common(&|t| layout(t).columns.map_or(String::new(), |c| number(c.gap))))
                .editor(edit(Editor::Number(Field::TextColumnGap(slots.to_vec())))),
        );
    }
    let engine = |text: &kentos_commands::TextEntity| layout(text).engine;
    let engine_label = |engine: Option<kentos_commands::TextLayoutEngine>| {
        if engine.is_some() {
            "Unicode"
        } else {
            "Eski görünüm"
        }
    };
    let engine_text =
        common(&|text| engine_label(engine(text)).to_owned()).unwrap_or_else(|| MIXED.to_owned());
    let editor = if let [slot] = slots {
        edit(Editor::Select {
            text: engine_text.clone(),
            swatch: None,
            icon: None,
            items: [
                None,
                Some(kentos_geometry_core::text::paragraph::Engine::UnicodeV1),
            ]
            .into_iter()
            .map(|value| Choice::Pick {
                label: format!(
                    "{} · çizimde önizle",
                    engine_label(value.map(|_| kentos_commands::TextLayoutEngine::UnicodeV1))
                ),
                swatch: None,
                icon: None,
                wide: false,
                chosen: engine(texts[0]).is_some() == value.is_some(),
                enabled: true,
                message: Message::Properties(Event::TextLayoutPreview(*slot, value)),
            })
            .collect(),
        })
    } else {
        None
    };
    rows.insert(0, Row::text("Yazı yerleşimi", engine_text).editor(editor));

    rows.push(Row::text("Zemin", mask_text).editor(edit(masks)));
    rows
}

/// A leader's Not, Yükseklik, Dönüş, Ok and Zemin rows (docs/adr/0146 §7),
/// for one leader or the leaders of a selection: their common value, or
/// “Çeşitli”; each change is one step “Değiştir”. The web's `leaderRows`.
/// A linked text's Bağlı nesne row (docs/adr/0175 §4): for one text its
/// object's kind and label, with Nesneyi seç and Bağı kopar; for several,
/// how many are linked, with Bağı kopar for them all. None when no text of
/// them is linked; on a locked layer only Nesneyi seç. The web's `linkRow`
/// (ui/properties/textRows.ts) is the same.
pub(super) fn link_rows(
    model: &kentos_domain::Document,
    texts: &[&kentos_commands::TextEntity],
    slots: &[Slot],
    locked: bool,
) -> Vec<Row> {
    let linked: Vec<Slot> = texts
        .iter()
        .zip(slots)
        .filter(|(t, _)| t.label_of.is_some())
        .map(|(_, slot)| *slot)
        .collect();
    if linked.is_empty() {
        return Vec::new();
    }
    let object = match texts {
        [one] => one
            .label_of
            .and_then(|id| model.slot_of(kentos_domain::Uuid::from_bytes(id.0)))
            .and_then(|slot| Some((slot, model.get(slot)?))),
        _ => None,
    };
    let value = match (texts.len(), object) {
        (1, Some((_, e))) => match e.base().label.as_deref().filter(|l| !l.is_empty()) {
            Some(label) => format!("{} “{label}”", kind_title(e.kind())),
            None => kind_title(e.kind()).to_owned(),
        },
        (1, None) => "Çizimde yok".to_owned(),
        (_, _) => format!("{} yazı bağlı", linked.len()),
    };
    let action = |label: &str, event: Event| Choice::Pick {
        label: label.to_owned(),
        swatch: None,
        icon: None,
        wide: false,
        chosen: false,
        enabled: true,
        message: Message::Properties(event),
    };
    let mut items = Vec::new();
    if let Some((slot, _)) = object {
        items.push(action("Nesneyi seç", Event::SelectObject(slot)));
    }
    if !locked {
        items.push(action("Bağı kopar", Event::Unlink(linked)));
    }
    let editor = (!items.is_empty()).then(|| Editor::Select {
        text: value.clone(),
        swatch: None,
        icon: None,
        items,
    });
    vec![Row::text("Bağlı nesne", value).editor(editor)]
}

/// A text's fields (docs/adr/0325 §4): each its source and value, or why it is broken; its menu selects the source
/// object and takes the field out (its letters stay as text).
pub(super) fn text_field_rows(
    model: &kentos_domain::Document,
    t: &kentos_commands::TextEntity,
    slot: Slot,
    locked: bool,
) -> Vec<Row> {
    use kentos_commands::FieldSource;
    t.paragraph
        .fields
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let object = f
                .source
                .object()
                .and_then(|id| model.slot_of(kentos_domain::Uuid::from_bytes(id.0)))
                .and_then(|s| Some((s, model.get(s)?)));
            let named = |what: &str, name: &Option<String>| match name {
                Some(n) => format!("{what} “{n}”"),
                None => what.to_owned(),
            };
            let source = match &f.source {
                FieldSource::Object { value, name, .. } => {
                    let of = object.map_or("silinmiş nesne", |(_, e)| kind_title(e.kind()));
                    format!("{of}: {}", named(value.label(), name))
                }
                FieldSource::Project { value, name } => format!("Proje: {}", named(value.label(), name)),
            };
            let letters: String = t.text.chars().skip(f.start as usize).take((f.end - f.start) as usize).collect();
            // The value first (what the row has room for), the source in the menu's title.
            let value = match f.broken {
                Some(b) => format!("Kopuk: {}", b.words()),
                None => letters,
            };
            let action = |label: &str, message: Message| Choice::Pick {
                label: label.to_owned(),
                swatch: None,
                icon: None,
                wide: false,
                chosen: false,
                enabled: true,
                message,
            };
            // The source, as a title the menu does not act on.
            let mut items = vec![Choice::Pick {
                label: source,
                swatch: None,
                icon: None,
                wide: false,
                chosen: true,
                enabled: false,
                message: Message::TextTools(crate::text_tools::Event::Close),
            }];
            if let Some((s, _)) = object {
                items.push(action("Kaynağı seç", Message::Properties(Event::SelectObject(s))));
            }
            if !locked {
                items.push(action("Alanı kaldır", Message::TextTools(crate::text_tools::Event::Remove(slot, i as u32))));
            }
            let editor = (!items.is_empty()).then(|| Editor::Select { text: value.clone(), swatch: None, icon: None, items });
            let mut row = Row::text(format!("Alan {}", i + 1), value).editor(editor);
            row.warn = f.broken.is_some();
            row
        })
        .collect()
}

/// A leader's style, arms, tips and content blocks (docs/adr/0326): Kılavuz stili (a menu of the project's leader
/// styles, Yok first), and for one leader its arm count, each bound tip with Bağı çöz, its content blocks' count.
pub(super) fn multileader_rows(
    model: &kentos_domain::Document,
    leaders: &[&kentos_commands::LeaderEntity],
    slots: &[Slot],
    locked: bool,
) -> Vec<Row> {
    use crate::leader_tools::Event as L;
    let styles = &model.settings().leader_styles;
    let mut rows = Vec::new();
    let common = leaders.first().map(|l| l.leader_style.clone()).filter(|s| leaders.iter().all(|l| &l.leader_style == s));
    let name_of = |id: &Option<String>| match id {
        Some(id) => styles.iter().find(|s| &s.id == id).map_or("Silinmiş stil".to_owned(), |s| s.name.clone()),
        None => "Yok".to_owned(),
    };
    let value = common.as_ref().map_or_else(|| "Çeşitli".to_owned(), name_of);
    if !styles.is_empty() || leaders.iter().any(|l| l.leader_style.is_some()) {
        let pick = |label: String, id: Option<String>| Choice::Pick {
            label,
            swatch: None,
            icon: None,
            wide: false,
            chosen: common.as_ref() == Some(&id),
            enabled: !locked,
            message: Message::LeaderTools(L::Style(slots.to_vec(), id)),
        };
        let mut items = vec![pick("Yok".to_owned(), None)];
        items.extend(styles.iter().map(|s| pick(s.name.clone(), Some(s.id.clone()))));
        rows.push(Row::text("Kılavuz stili", value.clone()).editor(Some(Editor::Select { text: value, swatch: None, icon: None, items })));
    }
    let [l] = leaders else {
        return rows;
    };
    let slot = Slot(l.base.id);
    rows.push(Row::figure("Kollar", (l.arms.len() + 1).to_string()));
    let tips = std::iter::once(l.tip.as_ref()).chain(l.arms.iter().map(|a| a.tip.as_ref()));
    for (k, t) in tips.enumerate() {
        let Some(t) = t else { continue };
        let of = model.slot_of(kentos_domain::Uuid::from_bytes(t.uid.0)).and_then(|s| model.get(s));
        let what = match (&t.broken, of) {
            (Some(why), _) => format!("Kopuk: {why}"),
            (None, Some(e)) => format!("Bağlı: {}", kind_title(e.kind())),
            (None, None) => "Kopuk".to_owned(),
        };
        let items = vec![Choice::Pick {
            label: "Bağı çöz".to_owned(),
            swatch: None,
            icon: None,
            wide: false,
            chosen: false,
            enabled: !locked,
            message: Message::LeaderTools(L::Unbind(slot, k as u32)),
        }];
        let label = if k == 0 { "Ucu".to_owned() } else { format!("{}. kolun ucu", k + 1) };
        let mut row = Row::text(label, what.clone()).editor(Some(Editor::Select { text: what, swatch: None, icon: None, items }));
        row.warn = t.broken.is_some();
        rows.push(row);
    }
    if !l.blocks.is_empty() {
        rows.push(Row::figure("Blok içerik", l.blocks.len().to_string()));
    }
    rows
}

/// A hatch's rows (docs/adr/0186 §7): Desen (every choice with its icon,
/// its Ölçek and Açı carried over), Açı, Ölçek (a pattern's, on the paper),
/// Aralık (user lines'), İkinci renk, Degrade biçimi and Ters (a
/// gradient's), İlişkili with İlişkiyi kopar. The web's `hatchRows`.
pub(super) fn hatch_rows(
    h: &kentos_commands::HatchEntity,
    slot: Slot,
    locked: bool,
    plot_scale: f64,
    f: &Format,
) -> Vec<Row> {
    use kentos_interaction::hatch_options::{choice_of, icon_name};
    let edit = |editor: Editor| (!locked).then_some(editor);
    let number = |field: Field| edit(Editor::Number(field));
    let p = &h.pattern;
    let list = kentos_geometry_core::tools::hatch::choices();
    let now = choice_of(p);
    let name = match (now.and_then(|k| list.get(k)), &p.name) {
        (Some(c), _) => c.name.clone(),
        (None, Some(n)) => n.clone(),
        (None, None) => "Desen".to_owned(),
    };
    let pick =
        |label: String, icon: Option<&'static str>, chosen: bool, event: Event| Choice::Pick {
            label,
            swatch: None,
            icon,
            wide: false,
            chosen,
            enabled: true,
            message: Message::Properties(event),
        };
    let patterns = Editor::Select {
        text: name.clone(),
        swatch: None,
        icon: now.and_then(|k| list.get(k)).map(|c| icon_name(&c.icon)),
        items: list
            .iter()
            .enumerate()
            .map(|(k, c)| {
                pick(
                    c.label.clone(),
                    Some(icon_name(&c.icon)),
                    now == Some(k),
                    Event::Pattern(slot, k),
                )
                .previewed(icon_name(&c.preview))
            })
            .collect(),
    };
    let mut rows = vec![
        Row::text("Desen", name).editor(edit(patterns)),
        Row::figure("Açı", fixed(p.angle, 2))
            .unit("°")
            .editor(number(Field::HatchAngle(slot))),
    ];
    match p.kind {
        HatchPatternType::Pattern => {
            let paper = p.scale.unwrap_or(0.0) * 1000.0 / plot_scale;
            rows.push(
                Row::figure("Ölçek", fixed(paper, 3)).editor(number(Field::HatchScale(slot))),
            );
        }
        HatchPatternType::Lines | HatchPatternType::Cross => rows.push(
            Row::figure("Aralık", f.length(p.spacing))
                .unit(f.length_unit_label())
                .editor(number(Field::HatchSpacing(slot))),
        ),
        _ => {}
    }
    if let Some(g) = &p.gradient {
        let colours = Editor::Select {
            text: g.color2.clone(),
            swatch: Some(g.color2.clone()),
            icon: None,
            items: kentos_geometry_core::tools::hatch::COLOURS
                .iter()
                .map(|(n, hex)| Choice::Pick {
                    label: (*n).to_owned(),
                    swatch: Some((*hex).to_owned()),
                    icon: None,
                    wide: false,
                    chosen: g.color2 == *hex,
                    enabled: true,
                    message: Message::Properties(Event::GradientColour(slot, (*hex).to_owned())),
                })
                .collect(),
        };
        const SHAPES: [(GradientShape, &str, &str); 3] = [
            (GradientShape::Linear, "Doğrusal", "hatchGradientLinear"),
            (GradientShape::Cylinder, "Silindir", "hatchGradientCylinder"),
            (GradientShape::Spherical, "Küre", "hatchGradientSpherical"),
        ];
        let shape = SHAPES
            .iter()
            .find(|s| s.0 == g.shape)
            .copied()
            .unwrap_or(SHAPES[0]);
        let shapes = Editor::Select {
            text: shape.1.to_owned(),
            swatch: None,
            icon: Some(shape.2),
            items: SHAPES
                .iter()
                .map(|&(s, label, icon)| {
                    pick(
                        label.to_owned(),
                        Some(icon),
                        s == g.shape,
                        Event::GradientShape(slot, s),
                    )
                })
                .collect(),
        };
        let yes_no = |b: bool| if b { "Evet" } else { "Hayır" };
        let inverted = Editor::Select {
            text: yes_no(g.inverted).to_owned(),
            swatch: None,
            icon: None,
            items: [true, false]
                .into_iter()
                .map(|b| {
                    pick(
                        yes_no(b).to_owned(),
                        None,
                        b == g.inverted,
                        Event::GradientInverted(slot, b),
                    )
                })
                .collect(),
        };
        rows.extend([
            Row::text("İkinci renk", g.color2.clone()).editor(edit(colours)),
            Row::text("Degrade biçimi", shape.1).editor(edit(shapes)),
            Row::text("Ters", yes_no(g.inverted)).editor(edit(inverted)),
        ]);
    }
    let tie = match &h.assoc {
        Some(a) => format!("Evet ({} nesne)", a.objects().count()),
        None => "Hayır".to_owned(),
    };
    let untie = (h.assoc.is_some() && !locked).then(|| Editor::Select {
        text: tie.clone(),
        swatch: None,
        icon: Some("hatchAssoc"),
        items: vec![pick(
            "İlişkiyi kopar".to_owned(),
            None,
            false,
            Event::Unlink(vec![slot]),
        )],
    });
    rows.push(Row::text("İlişkili", tie).editor(untie));
    rows.push(Row::text("Sınır izlemesi", if h.assoc.is_some() {"Kaynakları izler"} else {"Bağ yok; mevcut geometri sabit"}));
    // Sınır ▾ (docs/adr/0329 §4): exact when it holds an arc or a curve; written again as objects.
    let exact = if h.boundary.is_some() { "Kesin" } else { "Çizilen halka" };
    let recreate = (!locked).then(|| Editor::Select {
        text: exact.to_owned(),
        swatch: None,
        icon: Some("hatchRecreate"),
        items: vec![
            pick(
                "Sınırı yeniden oluştur: alan".to_owned(),
                None,
                false,
                Event::RecreateBoundary(slot, kentos_commands::BoundaryForm::Area),
            ),
            pick(
                "Sınırı yeniden oluştur: çoklu çizgi".to_owned(),
                None,
                false,
                Event::RecreateBoundary(slot, kentos_commands::BoundaryForm::Polyline),
            ),
        ],
    });
    rows.push(Row::text("Sınır", exact).editor(recreate));
    // Where its lines are anchored (docs/adr/0331 §2): the world's origin or its own point.
    if !matches!(p.kind, HatchPatternType::Solid | HatchPatternType::Gradient) {
        let origin = p.origin.map_or_else(
            || "Dünya başlangıcı".to_owned(),
            |o| f.point(kentos_geometry_core::Vec2::new(o.x, o.y)),
        );
        rows.push(Row::text("Başlangıç", origin));
    }
    rows
}

pub(super) fn leader_rows(
    leaders: &[&kentos_commands::LeaderEntity],
    slots: &[Slot],
    locked: bool,
    f: &Format,
) -> Vec<Row> {
    const MIXED: &str = "Çeşitli";
    let first = leaders[0];
    let same = |of: &dyn Fn(&kentos_commands::LeaderEntity) -> bool| leaders.iter().all(|l| of(l));
    let note = same(&|l| l.text == first.text).then(|| first.text.clone().unwrap_or_default());
    let height = same(&|l| l.height == first.height).then_some(first.height);
    let turn = same(&|l| l.rotation == first.rotation).then_some(first.rotation);
    let arrow = same(&|l| l.arrow == first.arrow).then_some(first.arrow);
    let size = |l: &kentos_commands::LeaderEntity| l.arrow_size.unwrap_or(1.0);
    let arrow_size = same(&|l| size(l) == size(first)).then_some(size(first));
    let mask = same(&|l| l.mask == first.mask).then_some(first.mask);
    let on_off = |on: bool| if on { "Açık" } else { "Kapalı" };
    let edit = |editor: Editor| (!locked).then_some(editor);
    let arrow_text = arrow.map_or_else(|| MIXED.to_owned(), |a| leader::arrow_label(a).to_owned());
    let arrows = Editor::Select {
        text: arrow_text.clone(),
        swatch: None,
        icon: arrow.map(leader::arrow_icon),
        items: leader::ARROWS
            .iter()
            .map(|&(a, _, label, icon)| Choice::Pick {
                label: label.to_owned(),
                swatch: None,
                icon: Some(icon),
                wide: false,
                chosen: arrow == Some(a),
                enabled: true,
                message: Message::Properties(Event::LeaderArrow(slots.to_vec(), a)),
            })
            .collect(),
    };
    let mask_text = mask.map_or(MIXED, on_off).to_owned();
    let masks = Editor::Select {
        text: mask_text.clone(),
        swatch: None,
        icon: None,
        items: [true, false]
            .into_iter()
            .map(|on| Choice::Pick {
                label: on_off(on).to_owned(),
                swatch: None,
                icon: None,
                wide: false,
                chosen: mask == Some(on),
                enabled: true,
                message: Message::Properties(Event::LeaderMask(slots.to_vec(), on)),
            })
            .collect(),
    };
    let height_row = match height {
        Some(h) => Row::figure("Yükseklik", f.length_bare(h)).unit(f.length_unit_label()),
        None => Row::text("Yükseklik", MIXED),
    };
    let turn_row = match turn {
        Some(t) => Row::figure("Dönüş", fixed(t, 2)).unit("°"),
        None => Row::text("Dönüş", MIXED),
    };
    // The arrowhead's length in the note's height (docs/adr/0205 §7).
    let size_row = match arrow_size {
        Some(s) => Row::figure("Ok boyu", fixed(s, 2)).unit("× yükseklik"),
        None => Row::text("Ok boyu", MIXED),
    };
    vec![
        Row::text("Not", note.unwrap_or_else(|| MIXED.to_owned()))
            .editor(edit(Editor::Text(Field::LeaderNote(slots.to_vec())))),
        height_row.editor(edit(Editor::Number(Field::LeaderHeight(slots.to_vec())))),
        turn_row.editor(edit(Editor::Number(Field::LeaderTurn(slots.to_vec())))),
        Row::text("Ok", arrow_text).editor(edit(arrows)),
        size_row.editor(edit(Editor::Number(Field::LeaderArrowSize(slots.to_vec())))),
        Row::text("Zemin", mask_text).editor(edit(masks)),
    ]
}

pub(super) fn looks_numeric(value: &str) -> bool {
    let digits = value.strip_prefix('-').unwrap_or(value);
    let mut parts = digits.splitn(2, ['.', ',']);
    let whole = parts.next().unwrap_or_default();
    let all_digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    all_digits(whole) && parts.next().is_none_or(all_digits)
}
