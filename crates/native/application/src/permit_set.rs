//! Ruhsat takımı (docs/adr/3037, M37): a phase's set of sheets made from the drawing in one step: the drawing's
//! particulars (`kentos_sheet::permit::PARTICULARS`) written into its variables, the ones it lacks added with their
//! defaults, then the sheets in the Mimarlar Odası's order (the information sheet, the site plan, the storeys' plans
//! from the lowest up) on the smallest A paper all the maps fit, through `cad.sheets.edit` (one undo step).
//!
//! - The plans share one ground (every storey's objects' box) at the phase's scale, so they lie over each other on
//!   paper as in the building (docs/adr/0718); a storey with no object has no sheet.
//! - The site plan draws the ground storey (with the objects on no storey, the parcel among them) at 1/500, 1/1000
//!   when 1/500 does not fit A0, around the parcel (docs/adr/3032), else the ground storey.
//! - The storeys' EK-2 codes from their elevations: K00 at ±0.00, K01 … above, B01 … below.

use std::collections::BTreeSet;

use kentos_commands::{
    CommandResult, ProjectVariable, SheetPicture, SheetsEdit, SheetsEdited, VariableKind,
    VariableValue,
};
use kentos_domain::Document;
use kentos_sheet::permit::{self, Page, Phase};
use kentos_sheet::template::{self, PaperChoice};

use crate::checks::{Stop, error};
use crate::{ExecutionContext, sheets_edit, sheets_fit, sheets_levels};

/// What a set is made with.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PermitSet {
    pub phase: Phase,
    /// Particulars written into the drawing's variables first (name, text); an empty text clears one.
    pub values: Vec<(String, String)>,
    /// The calculation tables the set's “Mimari hesap raporu” shows (their names and boxes, metres), in its order;
    /// none: no such sheet (`kentos_interaction::calc_tables::ensure` makes and finds them).
    pub tables: Vec<(String, [f64; 4])>,
    /// The lists that are sheets of their own (docs/adr/3038), after the plans: each its title (Mahal listesi), its
    /// EK-2 kind (MHL), its box (metres) and the scale it is drawn at; each on the smallest A paper it fits.
    pub lists: Vec<ListSheet>,
}

/// A list the set gives a sheet of its own (docs/adr/3038).
#[derive(Clone, Debug, PartialEq)]
pub struct ListSheet {
    pub title: String,
    pub kind: &'static str,
    pub bounds: [f64; 4],
    pub scale: u32,
    /// What it shows is drawn to scale (the joinery list's elevations): its title block says the scale.
    pub scaled: bool,
}

/// `variables` with the particulars it lacks added (their defaults; the work's name the drawing's) and `values`
/// written over them.
pub fn with_particulars(
    variables: &[ProjectVariable],
    drawing: &str,
    values: &[(String, String)],
) -> Vec<ProjectVariable> {
    let fold = |s: &str| kentos_sheet::expr::fold(s);
    let mut out = variables.to_vec();
    for (name, label, default) in permit::PARTICULARS {
        if out.iter().any(|v| fold(&v.name) == fold(name)) {
            continue;
        }
        let default = match name {
            "proje_adi" => Some(drawing).filter(|d| !d.trim().is_empty()),
            _ => default,
        };
        out.push(ProjectVariable {
            name: name.to_owned(),
            label: label.to_owned(),
            kind: VariableKind::Text,
            value: default.map_or(VariableValue::Null, |d| VariableValue::Text(d.to_owned())),
        });
    }
    if !out.iter().any(|v| fold(&v.name) == fold("notlar")) {
        out.push(ProjectVariable {
            name: "notlar".into(),
            label: "Açıklamalar".into(),
            kind: VariableKind::Text,
            value: VariableValue::Text(permit::NOTES.into()),
        });
    }
    for (name, text) in values {
        if let Some(v) = out.iter_mut().find(|v| fold(&v.name) == fold(name)) {
            let t = text.trim();
            v.value = if t.is_empty() {
                VariableValue::Null
            } else {
                VariableValue::Text(t.to_owned())
            };
        }
    }
    out
}

/// The particulars the drawing's variables leave without a value: the title block writes them `‹ad?›` and a
/// publication is held until they are given.
pub fn missing_particulars(doc: &Document) -> Vec<(&'static str, &'static str)> {
    let fold = |s: &str| kentos_sheet::expr::fold(s);
    let vars = &doc.settings().variables;
    permit::PARTICULARS
        .iter()
        .filter(|(name, _, _)| {
            !vars
                .iter()
                .any(|v| fold(&v.name) == fold(name) && !matches!(v.value, VariableValue::Null))
                && !(*name == "proje_adi" && !doc.name().trim().is_empty())
        })
        .map(|(n, l, _)| (*n, *l))
        .collect()
}

fn centre(b: [f64; 4]) -> kentos_sheet::GroundPoint {
    kentos_sheet::GroundPoint {
        x: b[0] + (b[2] - b[0]) / 2.,
        y: b[1] + (b[3] - b[1]) / 2.,
    }
}

/// `b`'s size on paper at 1/`scale`, mm.
fn on_paper(b: [f64; 4], scale: u32) -> [f64; 2] {
    let k = 1000.0 / f64::from(scale);
    [(b[2] - b[0]) * k, (b[3] - b[1]) * k]
}

fn union(a: Option<[f64; 4]>, b: [f64; 4]) -> [f64; 4] {
    match a {
        None => b,
        Some(a) => [
            a[0].min(b[0]),
            a[1].min(b[1]),
            a[2].max(b[2]),
            a[3].max(b[3]),
        ],
    }
}

/// The set's pages for `phase` and the paper they go on, and what was left out (a list no paper holds).
pub(crate) fn pages(
    doc: &Document,
    phase: Phase,
    tables: &[(String, [f64; 4])],
    lists: &[ListSheet],
) -> Result<(Vec<Page>, PaperChoice, Vec<String>), Stop> {
    let levels = doc
        .settings()
        .arch
        .as_ref()
        .map(|a| a.levels.clone())
        .unwrap_or_default();
    if levels.is_empty() {
        return Err(Stop::failed(error(
            crate::codes::LEVEL_NOT_FOUND,
            "Projede kat yok; ruhsat takımı için Mimari ayarları'nda kat ekleyin.".into(),
            Some("levels".into()),
        )));
    }
    let codes = permit::level_codes(
        &levels
            .iter()
            .map(|l| (l.id.clone(), l.elevation))
            .collect::<Vec<_>>(),
    );
    let code_of = |id: &str| {
        codes
            .iter()
            .find(|(l, _)| l == id)
            .map_or("KXX", |(_, c)| c.as_str())
    };
    let mut storeys: Vec<(&kentos_commands::ArchLevel, [f64; 4])> = levels
        .iter()
        .filter_map(|l| {
            sheets_levels::level_extent(doc, &BTreeSet::from([l.id.clone()])).map(|b| (l, b))
        })
        .collect();
    if storeys.is_empty() {
        return Err(Stop::failed(error(
            "permit_set_empty",
            "Katlarda çizim yok; ruhsat takımının planları boş kalırdı.".into(),
            Some("levels".into()),
        )));
    }
    storeys.sort_by(|a, b| a.0.elevation.total_cmp(&b.0.elevation));
    let ground = storeys
        .iter()
        .find(|(l, _)| code_of(&l.id) == "K00")
        .or_else(|| storeys.iter().find(|(l, _)| l.elevation > -1e-6))
        .unwrap_or(&storeys[0]);
    let plans = storeys
        .iter()
        .fold(None, |acc, (_, b)| Some(union(acc, *b)))
        .unwrap_or(ground.1);
    // The site plan around the parcel (docs/adr/3032), else the ground storey.
    let parcel = doc
        .settings()
        .arch
        .as_ref()
        .and_then(|a| a.site.as_ref())
        .and_then(|s| s.parcel.as_deref())
        .and_then(|u| kentos_domain::Uuid::parse_str(u).ok())
        .and_then(|u| doc.slot_of(u))
        .and_then(|s| doc.get(s))
        .and_then(|e| sheets_fit::extent_of(doc, std::iter::once(e)));
    let site = parcel.map_or(ground.1, |p| union(Some(p), ground.1));
    let scale = phase.scale();
    // The tables are drawn at the plot scale's height: shown at it.
    let plot = doc.settings().plot_scale.round().max(1.0) as u32;
    let table_sizes: Vec<[f64; 2]> = tables
        .iter()
        .map(|(_, b)| on_paper(permit::padded(*b, plot), plot))
        .collect();
    let fits_tables =
        |p: kentos_sheet::Paper| table_sizes.is_empty() || permit::shelf(p, &table_sizes).is_some();
    let (paper, site_scale) = [500, 1000]
        .into_iter()
        .find_map(|s| {
            let sizes = [on_paper(plans, scale), on_paper(site, s)];
            // The smallest paper the drawings and the tables fit; the drawings' alone when the tables fit none.
            permit::PAPERS
                .into_iter()
                .find(|p| permit::fits(*p, &sizes) && fits_tables(*p))
                .map(|paper| PaperChoice {
                    paper,
                    orientation: kentos_sheet::Orientation::Landscape,
                })
                .or_else(|| permit::paper_for(&sizes))
                .map(|p| (p, s))
        })
        .ok_or_else(|| {
            Stop::failed(error(
                "permit_set_too_large",
                format!("Planlar 1/{scale} ölçekte A0 kâğıda sığmıyor; takımı uygulama ölçeğinde parça parça kurun."),
                Some("phase".into()),
            ))
        })?;
    let mut out = vec![
        Page::info(),
        Page::site(Some(ground.0.id.clone()), Some(centre(site)), site_scale),
    ];
    for (l, _) in &storeys {
        out.push(Page::plan(
            &l.id,
            &l.name,
            code_of(&l.id),
            Some(centre(plans)),
            scale,
        ));
    }
    // The lists after the plans (MO: mahal listesi, then the parking calculation), each on the smallest paper it fits.
    let mut notes = Vec::new();
    for l in lists {
        let size = [on_paper(permit::padded(l.bounds, l.scale), l.scale)];
        match permit::PAPERS
            .into_iter()
            .find(|p| permit::shelf(*p, &size).is_some())
        {
            Some(p) => out.push(Page::list(
                &l.title,
                l.kind,
                vec![(l.title.clone(), l.bounds)],
                l.scale,
                PaperChoice {
                    paper: p,
                    orientation: kentos_sheet::Orientation::Landscape,
                },
                l.scaled,
            )),
            None => notes.push(format!("{} A0 kâğıda sığmadı; paftası kurulmadı.", l.title)),
        }
    }
    // The calculation sheet last (MO: the parking calculation after the drawings), when its tables fit the paper.
    if !tables.is_empty() && fits_tables(paper.paper) {
        out.push(Page::tables(tables.to_vec(), plot));
    }
    Ok((out, paper, notes))
}

/// The set's sheets' change of the book, through `cad.sheets.edit`'s input.
pub(crate) fn edit(
    doc: &Document,
    phase: Phase,
    tables: &[(String, [f64; 4])],
    lists: &[ListSheet],
) -> Result<(SheetsEdit, Vec<permit::Made>, Vec<String>), Stop> {
    let template = template::system_template(permit::TEMPLATE).ok_or_else(|| {
        Stop::failed(error(
            "unknown_template",
            format!("“{}” sistem şablonu yok.", permit::TEMPLATE),
            Some("source".into()),
        ))
    })?;
    let (pages, paper, notes) = pages(doc, phase, tables, lists)?;
    let (book, _) = match doc.sheets() {
        Some(s) => kentos_sheet::project::read_project_sheets(s)
            .map_err(|e| Stop::failed(error("book_unreadable", e.message, None)))?,
        None => (Default::default(), Vec::new()),
    };
    let north = doc.settings().has_system();
    let made = permit::permit_sheets(&book, template, phase, paper, &pages, north, |sheet| {
        kentos_sheet::create::ids_for(template, sheet)
    })
    .map_err(|e| Stop::failed(error(&e.code, e.message, e.path)))?;
    let pictures = made
        .pictures
        .into_iter()
        .map(|p| {
            Ok(SheetPicture {
                sha256: p.meta.sha256,
                bytes: template::base64_decode(&p.data).ok_or_else(|| {
                    Stop::failed(error("bad_asset", "Şablon resmi okunamadı.".into(), None))
                })?,
            })
        })
        .collect::<Result<_, Stop>>()?;
    Ok((
        SheetsEdit {
            ops: made
                .ops
                .iter()
                .map(|op| serde_json::to_value(op).unwrap_or_default())
                .collect(),
            pictures,
            replace_pictures: false,
            expected_revision: None,
        },
        made.made,
        notes,
    ))
}

/// The particulars written (a settings edit, never an undo step), then the set's sheets (one undo step). The sheets
/// made, with the change's answer and what was left out.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: &PermitSet,
) -> Result<(SheetsEdited, Vec<permit::Made>, Vec<String>), String> {
    let mut settings = cx.doc.settings().clone();
    settings.variables = with_particulars(&settings.variables, cx.doc.name(), &input.values);
    cx.doc.set_settings(settings);
    let (edit, made, notes) = edit(cx.doc, input.phase, &input.tables, &input.lists)
        .map_err(|s| s.into_error().message)?;
    match sheets_edit::execute(cx, edit) {
        CommandResult::Completed { output, .. } => Ok((output, made, notes)),
        CommandResult::Failed { error } | CommandResult::NeedsInput { error } => Err(error.message),
        _ => Err("Ruhsat takımı kurulamadı.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particulars_are_added_once_and_written_over() {
        let have = vec![ProjectVariable {
            name: "ada".into(),
            label: "Ada".into(),
            kind: VariableKind::Text,
            value: VariableValue::Text("101".into()),
        }];
        let out = with_particulars(
            &have,
            "Daire",
            &[
                ("parsel".into(), " 7 ".into()),
                ("ada".into(), String::new()),
            ],
        );
        let get = |n: &str| out.iter().find(|v| v.name == n).map(|v| v.value.clone());
        assert_eq!(out.iter().filter(|v| v.name == "ada").count(), 1);
        assert_eq!(
            get("ada"),
            Some(VariableValue::Null),
            "an empty text clears it"
        );
        assert_eq!(get("parsel"), Some(VariableValue::Text("7".into())));
        assert_eq!(
            get("proje_adi"),
            Some(VariableValue::Text("Daire".into())),
            "the work's name the drawing's"
        );
        assert_eq!(get("bina_kodu"), Some(VariableValue::Text("00".into())));
        assert_eq!(get("mimar"), Some(VariableValue::Null), "must be given");
        assert!(matches!(get("notlar"), Some(VariableValue::Text(_))));
        assert_eq!(out.len(), permit::PARTICULARS.len() + 1);
    }
}
