//! An object's look in the panel: its colour, line weight, line type and its scale, transparency, mask,
//! layer and symbol, with their editors.

use super::*;

/// A colour as the panel names it: “Çeşitli” for a mixed selection, “Katmana göre”, or its name.
pub(super) fn color_text(current: Option<Option<&str>>) -> String {
    match current {
        None => "Çeşitli".to_owned(),
        Some(None) => "Katmana göre".to_owned(),
        Some(Some(value)) => DRAW_COLORS
            .iter()
            .find(|(_, v)| *v == value)
            .map_or_else(|| value.to_owned(), |(name, _)| (*name).to_owned()),
    }
}

/// A line weight as the panel names it: “Çeşitli” for a mixed selection, “Katmana göre”, or “0.35 mm”.
pub(super) fn weight_text(current: Option<Option<f64>>) -> String {
    match current {
        None => "Çeşitli".to_owned(),
        Some(None) => "Katmana göre".to_owned(),
        Some(Some(w)) => weight_label(w),
    }
}

/// Kalınlık ▾: by layer, then the weights, an imported one not in the list
/// among them (the web's `weightEditor`, docs/adr/0139).
pub(super) fn weight_editor(ids: &[Slot], current: Option<Option<f64>>) -> Editor {
    let set = |w: Option<f64>| Message::Properties(Event::Weight(ids.to_vec(), w));
    let mut weights = LINE_WEIGHTS.to_vec();
    if let Some(Some(w)) = current
        && !weights.contains(&w)
    {
        weights.push(w);
        weights.sort_by(f64::total_cmp);
    }
    let mut items = vec![
        Choice::Pick {
            label: "Katmana göre".to_owned(),
            swatch: None,
            icon: None,
            wide: false,
            chosen: current == Some(None),
            enabled: true,
            message: set(None),
        },
        Choice::Separator,
    ];
    items.extend(weights.into_iter().map(|w| Choice::Pick {
        label: weight_label(w),
        swatch: None,
        icon: None,
        wide: false,
        chosen: current == Some(Some(w)),
        enabled: true,
        message: set(Some(w)),
    }));
    Editor::Select {
        text: weight_text(current),
        swatch: None,
        icon: None,
        items,
    }
}

/// A pick of a look's menu.
pub(super) fn pick(label: String, chosen: bool, message: Message) -> Choice {
    Choice::Pick {
        label,
        swatch: None,
        icon: None,
        wide: false,
        chosen,
        enabled: true,
        message,
    }
}

/// Çizgi tipi ▾ (docs/adr/0260 §2, 0313 §7): by layer, then the four types, then the project's types with patterns;
/// `pattern` the objects' own pattern (none: various).
pub(super) fn line_type_row(
    doc: &Document,
    ids: &[Slot],
    current: Option<Option<kentos_commands::LineType>>,
    pattern: Option<Option<&str>>,
) -> Row {
    let text = match (current, pattern) {
        (_, Some(Some(name))) => name.to_owned(),
        (None, _) | (_, None) => "Çeşitli".to_owned(),
        (Some(t), Some(None)) => crate::ribbon_panels::line_type_text(t).to_owned(),
    };
    let plain = pattern == Some(None);
    let set = |t| Message::Properties(Event::LineType(ids.to_vec(), t));
    let mut items = vec![
        pick(
            "Katmana göre".into(),
            plain && current == Some(None),
            set(None),
        ),
        Choice::Separator,
    ];
    items.extend(kentos_commands::LineType::ALL.into_iter().map(|t| {
        pick(
            crate::ribbon_panels::line_type_text(Some(t)).to_owned(),
            plain && current == Some(Some(t)),
            set(Some(t)),
        )
    }));
    let types = &doc.model.settings().line_types;
    if !types.is_empty() {
        items.push(Choice::Separator);
        items.extend(types.iter().map(|t| {
            pick(
                t.name.clone(),
                pattern
                    .flatten()
                    .is_some_and(|p| kentos_commands::same_line_type_name(p, &t.name)),
                Message::Properties(Event::LinePattern(ids.to_vec(), Some(t.name.clone()))),
            )
        }));
    }
    Row::text("Çizgi tipi", text.clone()).editor(Some(Editor::Select {
        text,
        swatch: None,
        icon: None,
        items,
    }))
}

/// Çizgi tipi ölçeği ▾ (docs/adr/0260 §2): the pattern's factor, 1 its absence.
pub(super) fn line_type_scale_row(ids: &[Slot], current: Option<Option<f64>>) -> Row {
    let shown = plain_number;
    let text = match current {
        None => "Çeşitli".to_owned(),
        Some(s) => shown(s.unwrap_or(1.0)),
    };
    let set =
        |s: f64| Message::Properties(Event::LineTypeScale(ids.to_vec(), (s != 1.0).then_some(s)));
    let mut scales = vec![0.25, 0.5, 1.0, 2.0, 5.0, 10.0];
    if let Some(Some(s)) = current
        && !scales.contains(&s)
    {
        scales.push(s);
        scales.sort_by(f64::total_cmp);
    }
    let items = scales
        .into_iter()
        .map(|s| {
            pick(
                shown(s),
                current.map(|c| c.unwrap_or(1.0)) == Some(s),
                set(s),
            )
        })
        .collect();
    Row::text("Çizgi tipi ölçeği", text.clone()).editor(Some(Editor::Select {
        text,
        swatch: None,
        icon: None,
        items,
    }))
}

/// Saydamlık ▾ (docs/adr/0260 §3): by layer, then whole percents up to 90.
pub(super) fn transparency_row(ids: &[Slot], current: Option<Option<f64>>) -> Row {
    let percent = |t: f64| format!("%{}", (t * 100.0).round());
    let text = match current {
        None => "Çeşitli".to_owned(),
        Some(None) => "Katmana göre".to_owned(),
        Some(Some(t)) => percent(t),
    };
    let set = |t| Message::Properties(Event::Transparency(ids.to_vec(), t));
    let mut items = vec![
        pick("Katmana göre".into(), current == Some(None), set(None)),
        Choice::Separator,
    ];
    let mut steps = vec![0.0, 0.1, 0.25, 0.5, 0.75, 0.9];
    if let Some(Some(t)) = current
        && !steps.contains(&t)
    {
        steps.push(t);
        steps.sort_by(f64::total_cmp);
    }
    items.extend(
        steps
            .into_iter()
            .map(|t| pick(percent(t), current == Some(Some(t)), set(Some(t)))),
    );
    Row::text("Saydamlık", text.clone()).editor(Some(Editor::Select {
        text,
        swatch: None,
        icon: None,
        items,
    }))
}

/// Maske ▾ of areas (docs/adr/0310 §5): none, framed or frameless; `current` none when they differ.
pub(super) fn wipeout_row(ids: &[Slot], current: Option<Option<kentos_commands::Wipeout>>) -> Row {
    use kentos_commands::Wipeout;
    let name = |w: Option<Wipeout>| match w {
        None => "Yok",
        Some(Wipeout { frame: true }) => "Çerçeveli",
        Some(Wipeout { frame: false }) => "Çerçevesiz",
    };
    let text = current.map_or("Çeşitli", name).to_owned();
    let set = |w| Message::Properties(Event::Wipeout(ids.to_vec(), w));
    let items = [
        None,
        Some(Wipeout { frame: true }),
        Some(Wipeout { frame: false }),
    ]
    .into_iter()
    .map(|w| pick(name(w).into(), current == Some(w), set(w)))
    .collect();
    Row::text("Maske", text.clone()).editor(Some(Editor::Select {
        text,
        swatch: None,
        icon: None,
        items,
    }))
}

/// A number as the panel writes a factor: no trailing zeros (`2.5`, `10`).
pub(crate) fn plain_number(x: f64) -> String {
    let s = format!("{x:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// A symbol as the panel names it: “Çeşitli”, “Katman stiline göre”, or its
/// name among the project's styles; one of the system library, which the
/// desktop does not hold yet, by its id (docs/adr/0063).
pub(super) fn symbol_text(doc: &Document, current: Option<Option<&str>>) -> String {
    match current {
        None => "Çeşitli".to_owned(),
        Some(None) => "Katman stiline göre".to_owned(),
        Some(Some(id)) => doc
            .model
            .styles()
            .items
            .iter()
            .find(|item| item.get("id").and_then(|v| v.as_str()) == Some(id))
            .and_then(|item| item.get("name").and_then(|v| v.as_str()))
            .unwrap_or(id)
            .to_owned(),
    }
}

/// Katman ▾: every layer by its path, the locked ones not to be chosen (the web's `layerEditor`).
pub(super) fn layer_editor(doc: &Document, ids: &[Slot], current: Option<&str>) -> Editor {
    let layers = doc.model.layers();
    let shown = current.and_then(|id| layers.get(id));
    Editor::Select {
        text: shown.map_or_else(|| "Çeşitli".to_owned(), |n| n.name.clone()),
        swatch: shown.map(|n| n.style.color.clone()),
        icon: None,
        items: layers
            .leaves()
            .into_iter()
            .map(|l| Choice::Pick {
                label: layer_path(layers, &l.id),
                swatch: Some(l.style.color.clone()),
                icon: None,
                wide: false,
                chosen: Some(l.id.as_str()) == current,
                enabled: !layers.is_locked(&l.id),
                message: Message::Properties(Event::Layer(ids.to_vec(), l.id.clone())),
            })
            .collect(),
    }
}

/// Renk ▾: by layer, then the colours (the web's `colorEditor`).
pub(super) fn color_editor(ids: &[Slot], current: Option<Option<&str>>) -> Editor {
    let known = current
        .flatten()
        .and_then(|value| DRAW_COLORS.iter().find(|(_, v)| *v == value));
    let set = |color: Option<&str>| {
        Message::Properties(Event::Color(ids.to_vec(), color.map(str::to_owned)))
    };
    let mut items = vec![
        Choice::Pick {
            label: "Katmana göre".to_owned(),
            swatch: None,
            icon: None,
            wide: false,
            chosen: current == Some(None),
            enabled: true,
            message: set(None),
        },
        Choice::Separator,
    ];
    items.extend(DRAW_COLORS.iter().map(|(name, value)| Choice::Pick {
        label: (*name).to_owned(),
        swatch: Some((*value).to_owned()),
        icon: None,
        wide: false,
        chosen: current == Some(Some(*value)),
        enabled: true,
        message: set(Some(value)),
    }));
    Editor::Select {
        text: known.map_or_else(|| color_text(current), |(name, _)| (*name).to_owned()),
        swatch: known.map(|(_, value)| (*value).to_owned()),
        icon: None,
        items,
    }
}

/// Sembol ▾: the layer's style, or one from the library (the web's `symbolEditor`).
pub(super) fn symbol_editor(doc: &Document, current: Option<Option<&str>>) -> Editor {
    Editor::Select {
        text: symbol_text(doc, current),
        swatch: None,
        icon: None,
        items: vec![
            Choice::Pick {
                label: "Katman stiline göre".to_owned(),
                swatch: None,
                icon: None,
                wide: false,
                chosen: current == Some(None),
                enabled: true,
                message: Message::Run("style.clearSymbol"),
            },
            Choice::Separator,
            Choice::Command {
                label: "Kitaplıktan seç…",
                icon: crate::icons::from_web(Some("styles")),
                id: "style.assign",
            },
            Choice::Command {
                label: "Katman stili…",
                icon: crate::icons::from_web(Some("layerStyle")),
                id: "style.layerStyle",
            },
        ],
    }
}

/// Whether an object takes a symbol of its own: all but texts and dimensions
/// (the web's `geometryClassOf`).
pub(super) fn takes_symbol(e: &Entity) -> bool {
    !matches!(e, Entity::Text(_) | Entity::Dimension(_))
}
