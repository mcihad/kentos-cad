//! `road.sheets.plan` v1 (plan/road.md Y10.5, docs/adr/0908 §10; ROAD-SEC-11): an alignment laid out on plan–profile
//! sheets as Civil 3D's Create View Frames and Create Sheets do. A sheet's stretch is its plan map's width on the ground
//! at the set's scale; each plan map is centred on its stretch's chord and turned so the alignment runs left to right;
//! with a profile, each sheet's profile view (`road.profiles.draw` over the sheet's stations) is drawn in the model, the
//! first at `at` and each next under the one before, and the template's second map shows it at the same scale, so a
//! station lies over itself in plan and profile. The sheet set is kept in the road settings with the alignment's and the
//! profile's revisions. The profile views, the set and the sheets are one undo step “Yol paftaları” (the sheet book's
//! own edits are not undo steps, docs/adr/0251). The shared cases are `fixtures/commands/v1/road.sheets.plan.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CreateOperation, EntitiesCreate, NewObject, ProfilesDraw, RoadSheetDef, RoadSheetLaid,
    RoadSheetSetDef, RoadSheetsLaid, RoadSheetsPlan, SheetPicture, SheetTemplateSource, SheetsEdit,
};
use kentos_domain::Document;
use kentos_road::alignment::km_text;
use kentos_sheet::{GroundPoint, ItemKind, MapView, create, road_sheets, template};

use crate::checks::{self, Stop, error};
use crate::codes;
use crate::road_alignments as alignments;
use crate::road_terrains::fail;
use crate::{ExecutionContext, road_profiles, road_profiles_draw, sheets_edit};

/// The template when none is given: KentOS's plan–profile sheet.
pub const DEFAULT_TEMPLATE: &str = "sys:yol-plan-boykesit";
/// KGM's A1 plan–profile sheet (docs/adr/0981), the default with KGM's bands: its profile map holds them.
pub const KGM_TEMPLATE: &str = "sys:yol-plan-boykesit-kgm";
/// The undo steps' names.
pub const STEP: &str = "Yol paftaları";
pub const SECTIONS_STEP: &str = "Enkesit paftaları";
const MOST: usize = kentos_commands::MAX_ROAD_SHEETS;

pub(crate) fn bad(path: &str, words: String) -> Stop {
    fail(codes::INVALID_ROAD_SHEETS, words, path)
}

fn num(v: f64) -> String {
    format!("{v}")
}

pub(crate) struct Laid {
    pub(crate) set: RoadSheetSetDef,
    pub(crate) edit: SheetsEdit,
    pub(crate) objects: Vec<NewObject>,
    pub(crate) layer: Option<String>,
    pub(crate) out: RoadSheetsLaid,
    pub(crate) warnings: Vec<CommandWarning>,
}

/// The template `source` names, or KentOS's plan–profile sheet (KGM's A1 one with KGM's bands, docs/adr/0981).
pub(crate) fn template_of(source: Option<&SheetTemplateSource>, kgm: bool) -> Result<template::Template, Stop> {
    match source {
        None => {
            let id = if kgm { KGM_TEMPLATE } else { DEFAULT_TEMPLATE };
            template::system_template(id).cloned().ok_or_else(|| {
                Stop::failed(error("unknown_template", format!("“{id}” sistem şablonu yok."), Some("source".into())))
            })
        }
        Some(SheetTemplateSource::System { id }) => template::system_template(id).cloned().ok_or_else(|| {
            Stop::failed(error(
                "unknown_template",
                format!("“{id}” sistem şablonu yok; cad.sheets.set(details=true) ile şablonları listeleyin."),
                Some("source.id".into()),
            ))
        }),
        Some(SheetTemplateSource::Custom { template }) => template::read_template(&template.to_string())
            .map_err(|e| Stop::failed(error(&e.code, e.message, e.path.map(|p| format!("source.template.{p}"))))),
    }
}

pub(crate) fn not_fixed(words: &str) -> Stop {
    Stop::failed(error("not_fixed_map", words.into(), Some("source".into())))
}

fn prepare(doc: &Document, input: &RoadSheetsPlan) -> Result<Laid, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    alignments::road_project(doc)?;
    let list = alignments::alignments(doc);
    let (_, def) = alignments::find(list, &input.alignment, "alignment")?;
    if def.offset_of.is_some() {
        return Err(bad("alignment", format!("“{}” paralel bir aks; yol paftası PI'larıyla kurulan aksta olur.", def.name)));
    }
    let built = alignments::build(list, def, "alignment")?;
    let core = &built.core;
    // The profile, its views' layer and place.
    let profile = match &input.profile {
        None => None,
        Some(p) => {
            let (_, pdef) = road_profiles::find(road_profiles::profiles(doc), p, "profile")?;
            if pdef.alignment != def.id {
                return Err(bad("profile", format!("“{}” boykesiti “{}” aksında değil.", pdef.name, def.name)));
            }
            let layer = input
                .layer_id
                .clone()
                .ok_or_else(|| bad("layerId", "Boykesit görünümleri için bir katman (layerId) verin.".into()))?;
            let at = input.at.ok_or_else(|| bad("at", "İlk boykesit görünümünün yeri (at) verilmeli.".into()))?;
            let (p0, p1) = match (pdef.pvis.first(), pdef.pvis.last()) {
                (Some(a), Some(b)) => (a.station, b.station),
                _ => return Err(bad("profile", format!("“{}” boykesitinin PVI'ı yok.", pdef.name))),
            };
            Some((pdef, layer, at, p0, p1))
        }
    };
    // The range: the alignment's, or the profile's when there is one.
    let length = core.length;
    let (lo, hi) = profile.as_ref().map_or((0.0, length), |p| (p.3.max(0.0), p.4.min(length)));
    let from = input.from.unwrap_or(lo);
    let to = input.to.unwrap_or(hi);
    let span = format!("aks 0 ile {} m arası", num(length));
    if !from.is_finite() || !(-1e-9..=length + 1e-9).contains(&from) {
        return Err(bad("from", format!("{} m aksın üstünde değil: {span}.", num(from))));
    }
    if !to.is_finite() || !(-1e-9..=length + 1e-9).contains(&to) {
        return Err(bad("to", format!("{} m aksın üstünde değil: {span}.", num(to))));
    }
    if to <= from {
        return Err(bad("to", "Aralığın sonu başından sonra olmalı.".into()));
    }
    let (from, to) = (from.clamp(0.0, length), to.clamp(0.0, length));
    if let Some(p) = &input.layer_id
        && (profile.is_some() || input.match_lines == Some(true))
    {
        checks::layer(doc, p)?;
    }
    // Docs/adr/0998: match lines are drawn objects; they need a layer.
    if input.match_lines == Some(true) && input.layer_id.is_none() {
        return Err(bad("layerId", "Eşleşme çizgileri bir katmana çizilir; layerId verin.".into()));
    }
    let template = template_of(input.source.as_ref(), input.bands == Some(kentos_commands::ProfileBands::Kgm))?;
    let options: template::InstanceOptions = serde_json::from_value(input.options.clone()).map_err(|e| {
        Stop::failed(error("invalid_options", format!("Şablon seçenekleri okunamadı: {e}"), Some("options".into())))
    })?;
    for (field, given) in [("name", options.name.is_some()), ("center", options.center.is_some()), ("scale", options.scale.is_some())] {
        if given {
            return Err(Stop::failed(error(
                "invalid_options",
                format!("Yol paftalarının adı, ortası ve ölçeği aksından ve scale'den gelir; options.{field} verilmez."),
                Some(format!("options.{field}")),
            )));
        }
    }
    // The plan map's size on paper and its scale.
    let maps = road_sheets::template_maps(&template);
    let need = if profile.is_some() { 2 } else { 1 };
    if maps.len() < need {
        return Err(not_fixed(if need == 2 {
            "Şablonun paftasında iki görünüm penceresi yok; plan–boykesit paftası planı birinciye, boykesiti ikinciye koyar."
        } else {
            "Şablonun paftasında görünüm penceresi yok; yol paftası haritasız yerleşmez."
        }));
    }
    let sample = template::instantiate(&template, &create::ids_for(&template, "olcu"), &options)
        .map_err(|e| Stop::failed(error(&e.code, e.message, e.path)))?;
    let item = &sample.sheet.items[maps[0]];
    let ItemKind::Map(m) = &item.kind else { return Err(not_fixed("Şablonun ilk haritası harita değil.")) };
    let MapView::Fixed(view) = &m.view else {
        return Err(not_fixed("Şablonun görünüm penceresi sabit görünümlü değil; yol paftası haritayı kendisi yerleştirir."));
    };
    let scale = input.scale.unwrap_or(view.scale);
    if !(1..=1_000_000).contains(&scale) {
        return Err(bad("scale", "Ölçek 1 ile 1 000 000 arası olmalı.".into()));
    }
    let ground = |um: i32| f64::from(um) / 1_000_000.0 * f64::from(scale);
    let (width, height) = (ground(item.frame.width), ground(item.frame.height));
    let profile_height = profile.as_ref().map(|_| ground(sample.sheet.items[maps[1]].frame.height));
    let overlap = input.overlap.unwrap_or(0.0);
    if !(overlap.is_finite() && overlap >= 0.0 && overlap < 0.5 * width) {
        return Err(bad("overlap", format!("Bindirme 0 ile bir paftanın boyunun yarısı ({} m) arasında olmalı.", num(0.5 * width))));
    }
    // The stretches.
    let mut stretches: Vec<(f64, f64)> = Vec::new();
    let mut a = from;
    loop {
        let b = (a + width).min(to);
        stretches.push((a, b));
        if stretches.len() > MOST {
            return Err(bad("scale", format!("Bu ölçekle {MOST}'den çok pafta çıkar; ölçeği büyütün ya da aralığı daraltın.")));
        }
        if b >= to - 1e-9 {
            break;
        }
        a = b - overlap;
    }
    // The set's name and id.
    let sets = doc.road_sheet_sets();
    let name = input.name.as_deref().map(str::trim).filter(|n| !n.is_empty()).map_or_else(|| format!("{} paftaları", def.name), str::to_owned);
    if sets.iter().any(|s| s.name.to_lowercase() == name.to_lowercase()) {
        return Err(bad("name", format!("“{name}” adında bir pafta takımı var.")));
    }
    let id = (1..).map(|n| format!("ps-{n}")).find(|i| sets.iter().all(|s| s.id != *i)).unwrap_or_default();
    // Each sheet: its plan map, its profile view.
    // Docs/adr/0980: KGM's bands are deeper than KentOS's.
    let depth = if input.bands == Some(kentos_commands::ProfileBands::Kgm) {
        road_profiles_draw::kgm_depth(doc, 2.5)
    } else {
        road_profiles_draw::bands_depth(doc, 2.5)
    };
    // Docs/adr/0995: KGM's earthwork rows go between the frame and the bands.
    let dky = input.haul.as_ref().is_some_and(|n| {
        crate::road_hauls::find(doc.road_hauls(), n, "haul").is_ok_and(|(_, h)| !h.unsuitable.is_empty())
    });
    let depth = depth + if input.haul.is_some() { road_profiles_draw::earthwork_rows_height(doc, 2.5, dky) } else { 0.0 };
    let gap = 0.4 * depth;
    // Docs/adr/0989: KGM's slope and platform rows stand over each view's frame.
    let rise = if input.corridor.is_some() { road_profiles_draw::corridor_rows_height(doc, 2.5) } else { 0.0 };
    let mut frames = Vec::new();
    let mut laid = Vec::new();
    let mut objects = Vec::new();
    let mut warnings: Vec<CommandWarning> = Vec::new();
    let mut next_y: Option<f64> = None;
    let mut outside = Vec::new();
    for (k, (a, b)) in stretches.iter().copied().enumerate() {
        let (pa, pb) = match (core.at(a), core.at(b)) {
            (Some(pa), Some(pb)) => (pa, pb),
            _ => return Err(bad("to", format!("{} m ile {} m arası aksın üstünde değil.", num(a), num(b)))),
        };
        let theta = (pb.y - pa.y).atan2(pb.x - pa.x);
        let center = [0.5 * (pa.x + pb.x), 0.5 * (pa.y + pb.y)];
        let degrees = theta.to_degrees().rem_euclid(360.0);
        let mdeg = (degrees * 1000.0).round() as i64 % 360_000;
        let (c, s) = (theta.cos(), theta.sin());
        let n = (((b - a) / 1.0).ceil() as usize).clamp(2, 2000);
        let mut off = (0..=n).filter_map(|i| core.at(a + (b - a) * i as f64 / n as f64)).any(|p| {
            let (dx, dy) = (p.x - center[0], p.y - center[1]);
            let (u, v) = (dx * c + dy * s, -dx * s + dy * c);
            u.abs() > 0.5 * width + 1e-6 || v.abs() > 0.5 * height + 1e-6
        });
        let mut view = None;
        let mut profile_center = None;
        if let Some((pdef, layer, at, p0, p1)) = &profile {
            let (va, vb) = (a.max(*p0), b.min(*p1));
            if vb > va + 1e-9 {
                let mut asked = ProfilesDraw {
                    profile: pdef.id.clone(),
                    layer_id: layer.clone(),
                    at: [0.0, 0.0],
                    every: None,
                    datum: None,
                    exaggeration: input.exaggeration,
                    height: None,
                    from: Some(va),
                    to: Some(vb),
                    bands: input.bands,
                    superelevation: input.superelevation.clone(),
                    corridor: input.corridor.clone(),
                    span: None,
                    haul: input.haul.clone(),
                    expected_revision: None,
                };
                let (_, sized, _) = road_profiles_draw::view(doc, &asked)?;
                let mut h = sized.frame[1][1] - sized.frame[0][1];
                // Docs/adr/0990, KGM's kot taşıma: a view taller than the map is split to the whole metres of
                // elevation the map holds over its rows and bands; one that cannot be stays whole and outside.
                if let Some(ph) = profile_height.filter(|ph| h + rise + depth > ph + 1e-6) {
                    let ex = sized.exaggeration;
                    let room = ((ph - rise - depth) / ex).floor();
                    if room > 0.0 {
                        let split = ProfilesDraw { span: Some(room), ..asked.clone() };
                        if let Ok((_, tried, _)) = road_profiles_draw::view(doc, &split) {
                            asked = split;
                            h = tried.frame[1][1] - tried.frame[0][1];
                        }
                    }
                }
                // The first view's lower left is `at`; each next one's top is a gap under the bands before it.
                let y = next_y.map_or(at[1], |n| n - h - rise);
                asked.at = [at[0], y];
                let (made, drawn, said) = road_profiles_draw::view(doc, &asked)?;
                for w in said {
                    if warnings.iter().all(|x| x.code != w.code) {
                        warnings.push(w);
                    }
                }
                objects.extend(made);
                next_y = Some(y - depth - gap);
                profile_center = Some(GroundPoint { x: at[0] + 0.5 * (vb - va), y: y + 0.5 * (h + rise - depth) });
                if profile_height.is_some_and(|ph| h + rise + depth > ph + 1e-6) {
                    off = true;
                }
                view = Some(drawn.frame);
            }
        }
        if off {
            outside.push(k + 1);
        }
        let title = format!("Km {} – {}", km_text(built.km(a), 2), km_text(built.km(b), 2));
        frames.push(road_sheets::RoadFrame {
            title: title.clone(),
            center: GroundPoint { x: center[0], y: center[1] },
            rotation: mdeg as i32,
            profile: profile_center,
        });
        laid.push(RoadSheetLaid {
            sheet: String::new(),
            number: String::new(),
            title,
            from: a,
            to: b,
            center,
            rotation: mdeg as f64 / 1000.0,
            view,
            outside: off,
        });
    }
    if !outside.is_empty() {
        let shown: Vec<String> = outside.iter().map(|k| k.to_string()).collect();
        warnings.push(CommandWarning {
            code: codes::ROAD_SHEET_OUTSIDE.into(),
            message: format!(
                "{} paftada aks ya da boykesit görünümü haritanın dışına taşıyor ({}. pafta); ölçeği büyütün ya da şablonun haritasını genişletin.",
                outside.len(),
                shown.join(", ")
            ),
            path: Some("scale".into()),
        });
    }
    let (book, _) = match doc.sheets() {
        Some(s) => kentos_sheet::project::read_project_sheets(s)
            .map_err(|e| Stop::failed(error("book_unreadable", e.message, None)))?,
        None => (Default::default(), Vec::new()),
    };
    let made = road_sheets::road_sheets(
        &book,
        &template,
        &options,
        &frames,
        scale,
        input.first.as_deref(),
        &["Yol".to_owned(), def.name.clone()],
        &format!("yol-{id}"),
        |sheet| create::ids_for(&template, sheet),
    )
    .map_err(|e| Stop::failed(error(&e.code, e.message, e.path)))?;
    for (l, m) in laid.iter_mut().zip(&made.made) {
        l.sheet = m.sheet.clone();
        l.number = m.number.clone();
    }
    // Docs/adr/0998: a match line between each two sheets, at the middle of their overlap.
    if input.match_lines == Some(true) {
        let h = 2.5 * f64::from(scale) / 1000.0;
        for w in laid.windows(2) {
            let s = 0.5 * (w[0].to + w[1].from);
            let Some(p) = core.at(s) else { continue };
            let (t, n) = ([p.direction.cos(), p.direction.sin()], [-p.direction.sin(), p.direction.cos()]);
            let reach = 0.45 * height;
            out_lines(&mut objects, [p.x, p.y], t, n, reach, h, w[0].rotation, &km_text(built.km(s), 2), [&w[1].number, &w[0].number]);
        }
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
    let set = RoadSheetSetDef {
        id: id.clone(),
        name: name.clone(),
        alignment: def.id.clone(),
        alignment_revision: def.revision,
        profile: profile.as_ref().map(|p| p.0.id.clone()),
        profile_revision: profile.as_ref().map_or(0, |p| p.0.revision),
        from,
        to,
        scale,
        overlap,
        sections: None,
        sections_revision: 0,
        sheets: laid.iter().map(|l| RoadSheetDef { sheet: l.sheet.clone(), from: l.from, to: l.to, view: l.view }).collect(),
    };
    let out = RoadSheetsLaid {
        set: id,
        name,
        alignment: def.id.clone(),
        profile: profile.as_ref().map(|p| p.0.id.clone()),
        sections: None,
        scale,
        length: width,
        height,
        sheets: laid,
        uids: Vec::new(),
        revision: doc.revision().to_string(),
    };
    let layer = profile.map(|p| p.1).or_else(|| input.layer_id.clone().filter(|_| input.match_lines == Some(true)));
    Ok(Laid { set, edit, objects, layer, out, warnings })
}

/// A match line at `p` (docs/adr/0998): across the alignment (`n`, its normal; `t`, its tangent) `reach` each way, and
/// along it on each side "EŞLEŞME ÇİZGİSİ KM : km · PAFTA …", the side before naming the sheet after and the side after
/// the sheet before, `h` high, masked, readable on a map turned `turn` degrees.
#[allow(clippy::too_many_arguments)]
fn out_lines(
    out: &mut Vec<NewObject>,
    p: [f64; 2],
    t: [f64; 2],
    n: [f64; 2],
    reach: f64,
    h: f64,
    turn: f64,
    km: &str,
    numbers: [&str; 2],
) {
    use kentos_commands::TextAlign;
    let at = |a: f64, b: f64| [p[0] + n[0] * a + t[0] * b, p[1] + n[1] * a + t[1] * b];
    out.push(road_profiles_draw::line(at(-reach, 0.0), at(reach, 0.0), None, Some(0.5)));
    let degrees = n[1].atan2(n[0]).to_degrees();
    let (on_paper, _) = crate::road_plan_axes::readable(degrees - turn, TextAlign::MiddleCenter);
    // Turned to read, the words' foot faces one side of the line or the other: each sits off its own side.
    let flipped = (on_paper + turn - degrees).rem_euclid(360.0) > 90.0 && (on_paper + turn - degrees).rem_euclid(360.0) < 270.0;
    for (side, number) in [(-1.0, numbers[0]), (1.0, numbers[1])] {
        let foot = if (side < 0.0) != flipped { TextAlign::BottomCenter } else { TextAlign::TopCenter };
        let q = at(0.5 * reach, side * 0.4 * h);
        let words = format!("EŞLEŞME ÇİZGİSİ KM : {km} · PAFTA {number}");
        out.push(road_profiles_draw::masked(road_profiles_draw::turned(q, words, h, foot, None, on_paper + turn)));
    }
}

pub(crate) fn write(cx: &mut ExecutionContext<'_>, laid: Laid) -> CommandResult<RoadSheetsLaid> {
    let Laid { set, edit, objects, layer, mut out, warnings } = laid;
    let label = if set.sections.is_some() { SECTIONS_STEP } else { STEP };
    let group = cx.doc.begin_group(label);
    if let (Some(layer), false) = (layer, objects.is_empty()) {
        let create = EntitiesCreate {
            layer_id: layer,
            objects,
            operation: Some(if label == SECTIONS_STEP { CreateOperation::SectionDrawing } else { CreateOperation::ProfileDrawing }),
            expected_revision: None,
        };
        match crate::create::write_in(cx, create) {
            CommandResult::Completed { output, .. } => out.uids = output.created,
            other => {
                cx.doc.cancel_group(group);
                return crate::trim::relay(other);
            }
        }
    }
    let mut sets = cx.doc.road_sheet_sets().to_vec();
    sets.push(set);
    if let Err(r) = cx.doc.set_road_sheet_sets(sets, label) {
        cx.doc.cancel_group(group);
        return bad("", r.0).into();
    }
    match sheets_edit::write_execute(cx, edit) {
        CommandResult::Completed { .. } => {}
        other => {
            cx.doc.cancel_group(group);
            return crate::trim::relay(other);
        }
    }
    cx.doc.end_group(group);
    out.revision = cx.doc.revision().to_string();
    CommandResult::Completed { output: out, warnings }
}

/// The sheets `input` would lay, for a window: the answer and its warnings, or why there is none.
pub fn preview(doc: &Document, input: &RoadSheetsPlan) -> Result<(RoadSheetsLaid, Vec<CommandWarning>), String> {
    prepare(doc, input).map(|l| (l.out, l.warnings)).map_err(crate::road_corridors::words)
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &RoadSheetsPlan) -> CommandResult<()> {
    match prepare(cx.doc, input) {
        Ok(l) => CommandResult::Completed { output: (), warnings: l.warnings },
        Err(stop) => stop.into(),
    }
}

/// What execute would lay now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &RoadSheetsPlan) -> CommandResult<RoadSheetsLaid> {
    match prepare(cx.doc, input) {
        Ok(l) => CommandResult::Completed { output: l.out, warnings: l.warnings },
        Err(stop) => stop.into(),
    }
}

/// Draws the profile views, keeps the set and lays the sheets: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: RoadSheetsPlan) -> CommandResult<RoadSheetsLaid> {
    crate::call::recorded(cx, kentos_commands::ROAD_SHEETS_PLAN, kentos_commands::ROAD_SHEETS_PLAN_VERSION, input, |cx, input| {
        match prepare(cx.doc, &input) {
            Ok(l) => write(cx, l),
            Err(stop) => stop.into(),
        }
    })
}
