//! `road.sheets.sections` v1 (plan/road.md Y10.5b, docs/adr/0908 §11; ROAD-SEC-11): a sample line set's section views
//! laid on sheets as Civil 3D's production section views and Create Section Sheets do. A set's views are all alike in
//! size (`road.sections.draw`: the swath wide, the tallest elevation range high), so a sheet takes as many columns and
//! rows of them as its map holds on the ground at the set's scale; the views are drawn group by group, each group's
//! upper left a map's height and a gap under the one before, and each sheet's map shows its group. The set is kept in
//! the road settings with the sample line set's revision; the views, the set and the sheets are one undo step
//! “Enkesit paftaları”. The shared cases are `fixtures/commands/v1/road.sheets.sections.json`.

use kentos_commands::{
    CommandResult, CommandWarning, RoadSheetDef, RoadSheetLaid, RoadSheetSetDef, RoadSheetsLaid, RoadSheetsSections,
    SectionsDraw, SheetPicture, SheetsEdit,
};
use kentos_domain::Document;
use kentos_sheet::{GroundPoint, ItemKind, MapView, create, road_sheets as core, template};

use crate::checks::{self, Stop, error};
use crate::road_sheets::{Laid, bad, not_fixed, template_of, write};
use crate::{ExecutionContext, road_sections, road_sections_draw};

/// The template when none is given: KentOS's technical sheet, one large map.
pub const DEFAULT_TEMPLATE: &str = "sys:cad-teknik";
const MOST: usize = kentos_commands::MAX_ROAD_SHEETS;

fn prepare(doc: &Document, input: &RoadSheetsSections) -> Result<Laid, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    road_sections::road_project(doc)?;
    let (_, def) = road_sections::find_set(doc.road_sections(), &input.set, "set")?;
    let asked = |stations: Vec<f64>, at: [f64; 2], columns: Option<u32>| SectionsDraw {
        set: def.id.clone(),
        layer_id: input.layer_id.clone(),
        at,
        stations,
        columns,
        exaggeration: input.exaggeration,
        height: input.height,
        expected_revision: None,
    };
    let (stations, [across, down], warnings) = road_sections_draw::pitch(doc, &asked(input.stations.clone(), input.at, Some(1)))?;
    let source = input.source.clone().unwrap_or(kentos_commands::SheetTemplateSource::System { id: DEFAULT_TEMPLATE.into() });
    let template = template_of(Some(&source), false)?;
    let options: template::InstanceOptions = serde_json::from_value(input.options.clone()).map_err(|e| {
        Stop::failed(error("invalid_options", format!("Şablon seçenekleri okunamadı: {e}"), Some("options".into())))
    })?;
    for (field, given) in [("name", options.name.is_some()), ("center", options.center.is_some()), ("scale", options.scale.is_some())] {
        if given {
            return Err(Stop::failed(error(
                "invalid_options",
                format!("Enkesit paftalarının adı, ortası ve ölçeği kesitlerinden ve scale'den gelir; options.{field} verilmez."),
                Some(format!("options.{field}")),
            )));
        }
    }
    let maps = core::template_maps(&template);
    let Some(&first_map) = maps.first() else {
        return Err(not_fixed("Şablonun paftasında görünüm penceresi yok; enkesit paftası haritasız yerleşmez."));
    };
    let sample = template::instantiate(&template, &create::ids_for(&template, "olcu"), &options)
        .map_err(|e| Stop::failed(error(&e.code, e.message, e.path)))?;
    let item = &sample.sheet.items[first_map];
    let ItemKind::Map(m) = &item.kind else { return Err(not_fixed("Şablonun ilk haritası harita değil.")) };
    let MapView::Fixed(view) = &m.view else {
        return Err(not_fixed("Şablonun görünüm penceresi sabit görünümlü değil; enkesit paftası haritayı kendisi yerleştirir."));
    };
    let scale = input.scale.unwrap_or(view.scale);
    if !(1..=1_000_000).contains(&scale) {
        return Err(bad("scale", "Ölçek 1 ile 1 000 000 arası olmalı.".into()));
    }
    let ground = |um: i32| f64::from(um) / 1_000_000.0 * f64::from(scale);
    let (width, height) = (ground(item.frame.width), ground(item.frame.height));
    let columns = (width / across).floor() as usize;
    let rows = (height / down).floor() as usize;
    if columns == 0 || rows == 0 {
        return Err(bad(
            "scale",
            format!(
                "Bir enkesit görünümü ({:.1} × {:.1} m) 1:{scale} ölçekte haritaya ({:.1} × {:.1} m) sığmıyor; ölçeğin paydasını büyütün.",
                across, down, width, height
            ),
        ));
    }
    let per = columns * rows;
    let groups: Vec<&[f64]> = stations.chunks(per).collect();
    if groups.len() > MOST {
        return Err(bad("scale", format!("Bu ölçekle {MOST}'den çok pafta çıkar; ölçeği büyütün.")));
    }
    let sets = doc.road_sheet_sets();
    let name = input.name.as_deref().map(str::trim).filter(|n| !n.is_empty()).map_or_else(|| format!("{} paftaları", def.name), str::to_owned);
    if sets.iter().any(|s| s.name.to_lowercase() == name.to_lowercase()) {
        return Err(bad("name", format!("“{name}” adında bir pafta takımı var.")));
    }
    let id = (1..).map(|n| format!("ps-{n}")).find(|i| sets.iter().all(|s| s.id != *i)).unwrap_or_default();
    // Each group: its views drawn in rows from its region's upper left, its sheet's map on the region.
    let gap = 0.25 * down;
    let mut objects = Vec::new();
    let mut frames = Vec::new();
    let mut laid = Vec::new();
    for (k, group) in groups.iter().enumerate() {
        let top = input.at[1] - k as f64 * (height + gap);
        let (made, drawn, _) = road_sections_draw::view(doc, &asked(group.to_vec(), [input.at[0], top], Some(columns as u32)))?;
        objects.extend(made);
        let (a, b) = (drawn.views.first(), drawn.views.last());
        let title = match (a, b) {
            (Some(a), Some(b)) if a.station != b.station => format!("Enkesitler km {} – {}", a.km_text, b.km_text),
            (Some(a), _) => format!("Enkesit km {}", a.km_text),
            _ => "Enkesitler".to_owned(),
        };
        let center = [input.at[0] + 0.5 * width, top - 0.5 * height];
        frames.push(core::RoadFrame {
            title: title.clone(),
            center: GroundPoint { x: center[0], y: center[1] },
            rotation: 0,
            profile: None,
        });
        laid.push(RoadSheetLaid {
            sheet: String::new(),
            number: String::new(),
            title,
            from: group.iter().copied().fold(f64::INFINITY, f64::min),
            to: group.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            center,
            rotation: 0.0,
            view: Some([[input.at[0], top - height], [input.at[0] + width, top]]),
            outside: false,
        });
    }
    let (book, _) = match doc.sheets() {
        Some(s) => kentos_sheet::project::read_project_sheets(s)
            .map_err(|e| Stop::failed(error("book_unreadable", e.message, None)))?,
        None => (Default::default(), Vec::new()),
    };
    let made = core::road_sheets(
        &book,
        &template,
        &options,
        &frames,
        scale,
        Some(input.first.as_deref().unwrap_or("E-101")),
        &["Yol".to_owned(), def.name.clone()],
        &format!("yol-{id}"),
        |sheet| create::ids_for(&template, sheet),
    )
    .map_err(|e| Stop::failed(error(&e.code, e.message, e.path)))?;
    for (l, m) in laid.iter_mut().zip(&made.made) {
        l.sheet = m.sheet.clone();
        l.number = m.number.clone();
    }
    let edit = SheetsEdit {
        ops: made.ops.iter().map(|op| serde_json::to_value(op).unwrap_or_default()).collect(),
        pictures: made
            .pictures
            .into_iter()
            .map(|p| {
                Ok(SheetPicture {
                    sha256: p.meta.sha256,
                    bytes: template::base64_decode(&p.data).ok_or_else(|| {
                        Stop::failed(error("bad_asset", "Şablon resmi okunamadı.".into(), Some("source.template.assets".into())))
                    })?,
                })
            })
            .collect::<Result<_, Stop>>()?,
        replace_pictures: false,
        expected_revision: None,
    };
    let from = stations.iter().copied().fold(f64::INFINITY, f64::min);
    let to = stations.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let set = RoadSheetSetDef {
        id: id.clone(),
        name: name.clone(),
        alignment: def.alignment.clone(),
        alignment_revision: def.alignment_revision,
        profile: None,
        profile_revision: 0,
        sections: Some(def.id.clone()),
        sections_revision: def.revision,
        from,
        to,
        scale,
        overlap: 0.0,
        sheets: laid.iter().map(|l| RoadSheetDef { sheet: l.sheet.clone(), from: l.from, to: l.to, view: l.view }).collect(),
    };
    let out = RoadSheetsLaid {
        set: id,
        name,
        alignment: def.alignment.clone(),
        profile: None,
        sections: Some(def.id.clone()),
        scale,
        length: width,
        height,
        sheets: laid,
        uids: Vec::new(),
        revision: doc.revision().to_string(),
    };
    Ok(Laid { set, edit, objects, layer: Some(input.layer_id.clone()), out, warnings: dedupe(warnings) })
}

fn dedupe(warnings: Vec<CommandWarning>) -> Vec<CommandWarning> {
    let mut out: Vec<CommandWarning> = Vec::new();
    for w in warnings {
        if !out.contains(&w) {
            out.push(w);
        }
    }
    out
}

/// The sheets `input` would lay, for a window: the answer and its warnings, or why there is none.
pub fn preview(doc: &Document, input: &RoadSheetsSections) -> Result<(RoadSheetsLaid, Vec<CommandWarning>), String> {
    prepare(doc, input).map(|l| (l.out, l.warnings)).map_err(crate::road_corridors::words)
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &RoadSheetsSections) -> CommandResult<()> {
    match prepare(cx.doc, input) {
        Ok(l) => CommandResult::Completed { output: (), warnings: l.warnings },
        Err(stop) => stop.into(),
    }
}

/// What execute would lay now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &RoadSheetsSections) -> CommandResult<RoadSheetsLaid> {
    match prepare(cx.doc, input) {
        Ok(l) => CommandResult::Completed { output: l.out, warnings: l.warnings },
        Err(stop) => stop.into(),
    }
}

/// Draws the views, keeps the set and lays the sheets: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: RoadSheetsSections) -> CommandResult<RoadSheetsLaid> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_SHEETS_SECTIONS,
        kentos_commands::ROAD_SHEETS_SECTIONS_VERSION,
        input,
        |cx, input| match prepare(cx.doc, &input) {
            Ok(l) => write(cx, l),
            Err(stop) => stop.into(),
        },
    )
}
