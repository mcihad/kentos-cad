//! Kat planı paftaları (docs/adr/0718): one sheet per level of the project from one template, each map drawing its
//! level (docs/adr/0716), every map on the same ground (the levels' objects' box, so the plans lie over each other on
//! paper as they do in the building), numbered on from a first sheet number. A level a sheet's map shows already is
//! left out unless asked again (Revit's New Plan, “Do not duplicate existing views”). The designer's Yeni ▾ and
//! `cad.sheets.levels` make the same operations.

use std::collections::BTreeSet;

use crate::{
    ItemKind, MapView, Result, SheetError,
    create,
    model::SheetBook,
    ops::{Op, SetItemProps},
    template::{AssetWithBytes, InstanceIds, InstanceOptions, Template},
};

/// The first sheet number when none is given.
pub const FIRST_NUMBER: &str = "A-101";
/// The hierarchy the sheets' drawing information puts them in.
pub const SUBSET: [&str; 2] = ["Mimari", "Kat planları"];

/// How the sheets are made.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LevelSheetsOptions {
    /// Alt kat soluk on every map.
    pub below: bool,
    /// Levels a sheet's map shows already are made again.
    pub again: bool,
    /// The first sheet's number; its last digits count on (`A-101`, `A-102` …); none: [`FIRST_NUMBER`].
    pub first: Option<String>,
    /// The ground every map shows, metres (min east, min north, max east, max north): the levels' objects' box.
    pub bounds: Option<[f64; 4]>,
}

/// One sheet made.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelSheet {
    pub sheet: String,
    pub level: String,
    pub number: String,
}

/// The operations, in order, and what they make.
#[derive(Clone, Debug)]
pub struct LevelSheets {
    pub ops: Vec<Op>,
    pub pictures: Vec<AssetWithBytes>,
    pub made: Vec<LevelSheet>,
    /// The levels left out: a sheet's map shows them.
    pub kept: Vec<String>,
}

/// The levels some map of `book` draws.
pub fn shown_levels(book: &SheetBook) -> BTreeSet<String> {
    book.sheets
        .iter()
        .flat_map(|s| s.items.iter())
        .chain(book.masters.iter().flat_map(|m| m.items.iter()))
        .filter_map(|i| match &i.kind {
            ItemKind::Map(m) => m.level.as_ref().map(|l| l.id.clone()),
            _ => None,
        })
        .collect()
}

/// `first` cut at its last run of digits: what comes before, the number, its width, what comes after.
fn numbered(first: &str) -> Option<(String, u64, usize, String)> {
    let chars: Vec<char> = first.chars().collect();
    let end = chars.iter().rposition(char::is_ascii_digit)? + 1;
    let start = chars[..end]
        .iter()
        .rposition(|c| !c.is_ascii_digit())
        .map_or(0, |i| i + 1);
    let digits: String = chars[start..end].iter().collect();
    let n = digits.parse::<u64>().ok()?;
    Some((
        chars[..start].iter().collect(),
        n,
        digits.len(),
        chars[end..].iter().collect(),
    ))
}

/// The sheets of `levels` (id, name) made from `template` with `options` (its paper, scale and answers; the name is
/// each sheet's own), `ids` giving each sheet its ids from a suggested sheet id. The operations make them in one change.
pub fn level_sheets(
    book: &SheetBook,
    template: &Template,
    options: &InstanceOptions,
    levels: &[(String, String)],
    how: &LevelSheetsOptions,
    mut ids: impl FnMut(&str) -> InstanceIds,
) -> Result<LevelSheets> {
    let shown = shown_levels(book);
    let (todo, kept): (Vec<_>, Vec<_>) = levels
        .iter()
        .partition(|(id, _)| how.again || !shown.contains(id));
    if todo.is_empty() {
        return Err(SheetError::at(
            "levels_have_sheets",
            "levels",
            "Seçilen katların hepsinin planı bir paftada var; yeniden yapmak için “Yeniden” seçin.",
        ));
    }
    let map_at = template
        .sheet
        .items
        .iter()
        .position(|i| matches!(i.kind, ItemKind::Map(_)))
        .ok_or_else(|| {
            SheetError::at(
                "not_fixed_map",
                "source",
                "Şablonun paftasında görünüm penceresi yok; kat planı haritasız yerleşmez.",
            )
        })?;
    let first = how.first.as_deref().unwrap_or(FIRST_NUMBER);
    let (before, start, width, after) = numbered(first).ok_or_else(|| {
        SheetError::at(
            "bad_drawing_info",
            "first",
            format!("“{first}” bir sayıyla bitmiyor; ilk pafta numarası A-101 gibi olmalı."),
        )
    })?;
    let mut used: BTreeSet<String> = book
        .sheets
        .iter()
        .filter_map(|s| s.drawing_info.as_ref())
        .map(|i| crate::expr::fold(&i.number))
        .collect();
    let mut taken: BTreeSet<String> = book.sheets.iter().map(|s| s.id.clone()).collect();
    let mut next = start;
    let mut out = LevelSheets {
        ops: Vec::new(),
        pictures: Vec::new(),
        made: Vec::new(),
        kept: kept.into_iter().map(|(id, _)| id.clone()).collect(),
    };
    for (level, name) in todo {
        let number = loop {
            let n = format!("{before}{next:0width$}{after}");
            next += 1;
            if used.insert(crate::expr::fold(&n)) {
                break n;
            }
        };
        let mut sheet = format!("kat-plani-{level}");
        let mut k = 2;
        while taken.contains(&sheet) {
            sheet = format!("kat-plani-{level}-{k}");
            k += 1;
        }
        let ids = ids(&sheet);
        taken.insert(ids.sheet.clone());
        let title = format!("{name} planı");
        let mut own = options.clone();
        own.name = Some(format!("{number} {title}"));
        if own.center.is_none()
            && let Some(b) = how.bounds
        {
            own.center = Some(crate::GroundPoint {
                x: b[0] + (b[2] - b[0]) / 2.,
                y: b[1] + (b[3] - b[1]) / 2.,
            });
        }
        let made = create::from_template(template, &ids, &own, None)?;
        out.ops.extend(made.ops);
        for p in made.pictures {
            if !out.pictures.iter().any(|q| q.meta.sha256 == p.meta.sha256) {
                out.pictures.push(p);
            }
        }
        // The map: its level, and the ground every sheet shares at a standard scale when none is given.
        let item = crate::template::instantiate(template, &ids, &own)?
            .sheet
            .items
            .swap_remove(map_at);
        let level_patch = serde_json::json!({"id": level, "below": how.below});
        let fits = match &item.kind {
            ItemKind::Map(m) => {
                !item.locked && !m.view_locked && matches!(m.view, MapView::Fixed(_))
            }
            _ => false,
        };
        let mut patch = match how.bounds {
            Some(b) if fits && options.scale.is_none() && options.center.is_none() => {
                match crate::fit::map(&item, b, 5, true)? {
                    Op::SetItemProps(p) => p.patch,
                    _ => serde_json::json!({"kind": {}}),
                }
            }
            _ => serde_json::json!({"kind": {}}),
        };
        patch["kind"]["level"] = level_patch;
        out.ops.push(Op::SetItemProps(SetItemProps {
            id: item.id.clone(),
            patch,
        }));
        out.ops.push(Op::SetDrawingInfo(crate::drawing_set::SetDrawingInfo {
            id: ids.sheet.clone(),
            info: Some(crate::drawing_set::DrawingInfo {
                number: number.clone(),
                title,
                revision: String::new(),
                status: String::new(),
                subset: SUBSET.iter().map(|s| (*s).to_owned()).collect(),
            }),
        }));
        out.made.push(LevelSheet {
            sheet: ids.sheet,
            level: level.clone(),
            number,
        });
    }
    Ok(out)
}
