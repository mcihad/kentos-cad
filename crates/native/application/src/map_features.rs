//! `map.features.code` v1 (plan/map-cad.md MAP-04, docs/adr/1360): Kodla çiz. A coded point's class is found by its
//! field code in the data profile (`ProfileClass.codes`); a point class's points go on the class's layer, a line or an
//! area class's points are joined into strings (digits after the code number the string; B begins a new one, K closes
//! it, S ends it) and drawn as polylines and polygons on the class's layer; the points stay. One undo step “Kodla çiz”.
//! The shared cases are `fixtures/commands/v1/map.features.code.json`.

use std::collections::BTreeMap;

use kentos_commands::{
    CodedClass, CommandResult, CommandWarning, DEFAULT_CODE_FIELD, DataProfile, Entity, EntityBase, EntityGeometry,
    FeaturesCode, FeaturesCoded, ProfileClass, ProfileGeometry, UnknownCode, Vec2,
};
use kentos_domain::{Document, Slot, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::profile_files::Shelf;
use crate::profiles::{Place, find, path_words, profile_only};

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn warning(code: &str, message: String) -> CommandWarning {
    CommandWarning { code: code.into(), message, path: None }
}

/// A coded point read: its slot, place, elevation, the code's letters and string number, its controls.
struct Coded {
    slot: Slot,
    layer: String,
    p: Vec2,
    z: Option<f64>,
    string: String,
    controls: Vec<String>,
    /// As written, for the new objects' code field.
    token: String,
}

/// A point's code split (docs/adr/1360 §2): the first word, upper-cased: its capital letters then its digits (the
/// string's number); the words after it, upper-cased, its controls. None when the first word is not letters then digits.
fn split(text: &str) -> Option<(String, String, Vec<String>, String)> {
    let mut words = text.split_whitespace().map(str::to_uppercase);
    let first = words.next()?;
    let letters: String = first.chars().take_while(char::is_ascii_uppercase).collect();
    let digits = &first[letters.len()..];
    if letters.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some((letters, digits.to_owned(), words.collect(), first))
}

/// A string being joined: its class, its points.
struct Open {
    class: usize,
    points: Vec<usize>,
}

/// What the input asks for, worked out against the drawing.
struct Plan {
    rows: Vec<CodedClass>,
    /// Points to put on their class's layer.
    moves: Vec<(Slot, String)>,
    /// New objects: their class's layer, geometry, the code field's text.
    makes: Vec<(String, EntityGeometry, String)>,
    unknown: Vec<UnknownCode>,
    warnings: Vec<CommandWarning>,
    field: String,
}

fn check(doc: &Document, input: &FeaturesCode) -> Result<(Plan, DataProfile), Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    let profile = profile_only(Some(&input.name), None, Shelf::of(doc, input.drawing.as_deref()))?;
    let field = input.field.clone().unwrap_or_else(|| DEFAULT_CODE_FIELD.to_owned());
    if field.trim().is_empty() {
        return Err(fail(codes::INVALID_INPUT, "Kod alanının adı boş olamaz.".into(), "field"));
    }
    let layers = doc.layers();
    // The points, in the order given or the drawing's.
    let mut points: Vec<(Slot, &Entity)> = Vec::new();
    match &input.uids {
        Some(uids) => {
            for (i, u) in uids.iter().enumerate() {
                let at = format!("uids[{i}]");
                if !checks::is_uid_text(u) {
                    return Err(fail(codes::INVALID_UID, format!("“{u}” geçerli bir nesne kimliği değil."), &at));
                }
                let Some((slot, e)) = Uuid::parse_str(u).ok().and_then(|x| doc.slot_of(x)).and_then(|s| doc.get(s).map(|e| (s, e)))
                else {
                    return Err(fail(codes::ENTITY_NOT_FOUND, format!("“{u}” kimlikli nesne çizimde yok."), &at));
                };
                if !matches!(e, Entity::Point(_)) {
                    return Err(fail(codes::NOT_A_POINT, format!("“{u}” bir nokta değil; Kodla çiz kodlu noktaları işler."), &at));
                }
                points.push((slot, e));
            }
        }
        None => {
            for (uid, e) in doc.entities_with_uids() {
                let layer = &e.base().layer_id;
                if matches!(e, Entity::Point(_))
                    && layers.is_visible(layer)
                    && !layers.is_locked(layer)
                    && let Some(slot) = doc.slot_of(uid)
                {
                    points.push((slot, e));
                }
            }
        }
    }
    let mut coded: Vec<Coded> = Vec::new();
    let mut unknown: BTreeMap<String, u32> = BTreeMap::new();
    let mut unknown_order: Vec<String> = Vec::new();
    let mut controls_unknown: Vec<String> = Vec::new();
    let mut class_of: Vec<usize> = Vec::new();
    for (slot, e) in points {
        let Entity::Point(pt) = e else { continue };
        let Some(text) = pt.base.attrs.get(&field).map(|t| t.trim()).filter(|t| !t.is_empty()) else { continue };
        let (code, string, controls, token) = match split(text) {
            Some(s) => s,
            None => {
                let raw = text.split_whitespace().next().unwrap_or(text).to_uppercase();
                if !unknown.contains_key(&raw) {
                    unknown_order.push(raw.clone());
                }
                *unknown.entry(raw).or_default() += 1;
                continue;
            }
        };
        let Some(ci) = profile.classes.iter().position(|c| c.codes.contains(&code)) else {
            if !unknown.contains_key(&code) {
                unknown_order.push(code.clone());
            }
            *unknown.entry(code).or_default() += 1;
            continue;
        };
        for c in controls.iter().filter(|c| !matches!(c.as_str(), "B" | "K" | "S")) {
            if !controls_unknown.contains(c) {
                controls_unknown.push(c.clone());
            }
        }
        class_of.push(ci);
        coded.push(Coded { slot, layer: pt.base.layer_id.clone(), p: pt.p, z: pt.z, string, controls, token });
    }
    if coded.is_empty() {
        return Err(fail(
            codes::NO_CODED_POINTS,
            format!("“{field}” alanında profilin bir sınıfının kodunu taşıyan nokta yok; noktaların kodunu yazın ya da alanın adını verin."),
            "uids",
        ));
    }
    // Each named class's layer, in the profile's order.
    let mut layer_of: BTreeMap<usize, String> = BTreeMap::new();
    for ci in 0..profile.classes.len() {
        if !class_of.contains(&ci) {
            continue;
        }
        let c = &profile.classes[ci];
        match find(layers.nodes(), &c.layer) {
            Place::Layer(id) => {
                layer_of.insert(ci, id);
            }
            _ => {
                return Err(fail(
                    codes::CLASS_LAYER_MISSING,
                    format!(
                        "“{}” sınıfının katmanı ({}) çizimde yok; önce “{}” profilini uygulayın.",
                        c.title,
                        path_words(&c.layer),
                        profile.name
                    ),
                    "name",
                ));
            }
        }
    }
    let mut rows: Vec<CodedClass> = layer_of
        .iter()
        .map(|(ci, layer)| CodedClass {
            class: profile.classes[*ci].id.clone(),
            title: profile.classes[*ci].title.clone(),
            layer: layer.clone(),
            ..CodedClass::default()
        })
        .collect();
    let row = |rows: &mut Vec<CodedClass>, ci: usize| -> usize {
        let id = &profile.classes[ci].id;
        rows.iter().position(|r| r.class == *id).unwrap_or(0)
    };
    let mut warnings = Vec::new();
    let mut moves: Vec<(Slot, String)> = Vec::new();
    let mut makes: Vec<(String, EntityGeometry, String)> = Vec::new();
    // Strings by (class, number), and the order they were opened in.
    let mut open: BTreeMap<(usize, String), Open> = BTreeMap::new();
    let mut opened: Vec<(usize, String)> = Vec::new();
    let (mut short, mut unclosed) = (0u32, 0u32);
    let finish = |s: Open, closed: bool, makes: &mut Vec<(String, EntityGeometry, String)>, rows: &mut Vec<CodedClass>,
                  short: &mut u32, unclosed: &mut u32| {
        let class: &ProfileClass = &profile.classes[s.class];
        let pts: Vec<Vec2> = s.points.iter().map(|i| coded[*i].p).collect();
        let zs: Option<Vec<Option<f64>>> =
            s.points.iter().all(|i| coded[*i].z.is_some()).then(|| s.points.iter().map(|i| coded[*i].z).collect());
        let token = coded[s.points[0]].token.clone();
        let r = row(rows, s.class);
        match class.geometry {
            ProfileGeometry::Line => {
                if pts.len() < 2 || (closed && pts.len() < 3) {
                    *short += 1;
                    return;
                }
                makes.push((
                    layer_of[&s.class].clone(),
                    EntityGeometry::Polyline { pts, bulges: None, zs, ms: None, parts: None, closed, widths: None },
                    token,
                ));
                rows[r].lines += 1;
            }
            ProfileGeometry::Area => {
                if !closed {
                    *unclosed += 1;
                    return;
                }
                if pts.len() < 3 {
                    *short += 1;
                    return;
                }
                makes.push((layer_of[&s.class].clone(), EntityGeometry::Polygon { pts, bulges: None, holes: None, zs, parts: None }, token));
                rows[r].areas += 1;
            }
            _ => {}
        }
    };
    for (i, c) in coded.iter().enumerate() {
        let ci = class_of[i];
        let r = row(&mut rows, ci);
        rows[r].points += 1;
        let target = &layer_of[&ci];
        match profile.classes[ci].geometry {
            ProfileGeometry::Point => {
                if c.layer != *target {
                    if layers.is_locked(&c.layer) {
                        return Err(fail(
                            codes::LAYER_LOCKED,
                            format!("“{}” katmanı kilitli: üstündeki kodlu nokta sınıfının katmanına taşınamaz. Kilidi açın.", layers.path(&c.layer)),
                            "uids",
                        ));
                    }
                    moves.push((c.slot, target.clone()));
                    rows[r].moved += 1;
                }
            }
            ProfileGeometry::Line | ProfileGeometry::Area => {
                let key = (ci, c.string.clone());
                if c.controls.iter().any(|w| w == "B")
                    && let Some(s) = open.remove(&key)
                {
                    finish(s, false, &mut makes, &mut rows, &mut short, &mut unclosed);
                }
                if !open.contains_key(&key) {
                    opened.retain(|k| *k != key);
                    opened.push(key.clone());
                }
                open.entry(key.clone()).or_insert_with(|| Open { class: ci, points: Vec::new() }).points.push(i);
                if c.controls.iter().any(|w| w == "K") {
                    if let Some(s) = open.remove(&key) {
                        finish(s, true, &mut makes, &mut rows, &mut short, &mut unclosed);
                    }
                } else if c.controls.iter().any(|w| w == "S")
                    && let Some(s) = open.remove(&key)
                {
                    finish(s, false, &mut makes, &mut rows, &mut short, &mut unclosed);
                }
            }
            ProfileGeometry::Record => {}
        }
    }
    // The strings still open end where their points end, in the order they were begun.
    for key in opened {
        if let Some(s) = open.remove(&key) {
            finish(s, false, &mut makes, &mut rows, &mut short, &mut unclosed);
        }
    }
    for (layer, _, _) in &makes {
        if layers.is_locked(layer) {
            return Err(fail(
                codes::LAYER_LOCKED,
                format!("“{}” katmanı kilitli: Kodla çiz oraya çizemez. Kilidi açın.", layers.path(layer)),
                "name",
            ));
        }
    }
    for code in &unknown_order {
        warnings.push(warning(
            codes::UNKNOWN_CODE,
            format!("“{code}” kodu “{}” profilinin hiçbir sınıfında yok; {} nokta olduğu gibi bırakıldı.", profile.name, unknown[code]),
        ));
    }
    if !controls_unknown.is_empty() {
        warnings.push(warning(
            codes::CONTROL_UNKNOWN,
            format!(
                "{} denetim kodu değil (B başla, K kapat, S son); yok sayıldı.",
                controls_unknown.iter().map(|c| format!("“{c}”")).collect::<Vec<_>>().join(", ")
            ),
        ));
    }
    if short > 0 {
        warnings.push(warning(
            codes::STRING_TOO_SHORT,
            format!("{short} dizi çizilmedi: çizgi için en az 2, kapalı çizgi ve alan için en az 3 nokta gerekir."),
        ));
    }
    if unclosed > 0 {
        warnings.push(warning(
            codes::AREA_NOT_CLOSED,
            format!("{unclosed} alan dizisi K ile kapatılmadığı için alan çizilmedi."),
        ));
    }
    let unknown = unknown_order.into_iter().map(|c| UnknownCode { points: unknown[&c], code: c }).collect();
    Ok((Plan { rows, moves, makes, unknown, warnings, field }, profile))
}

fn output(plan: &Plan, created: Vec<String>, revision: String) -> FeaturesCoded {
    FeaturesCoded { classes: plan.rows.clone(), created, unknown: plan.unknown.clone(), revision }
}

fn write(cx: &mut ExecutionContext<'_>, input: FeaturesCode) -> CommandResult<FeaturesCoded> {
    let (plan, _) = match check(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    let label = "Kodla çiz";
    let mut created = Vec::new();
    let _ = cx.doc.transact(label, |doc| -> Result<(), ()> {
        let updates: Vec<(Slot, Entity)> = plan
            .moves
            .iter()
            .filter_map(|(slot, layer)| {
                let mut e = doc.get(*slot)?.clone();
                e.base_mut().layer_id = layer.clone();
                Some((*slot, e))
            })
            .collect();
        if !updates.is_empty() {
            doc.update_many(updates, label);
        }
        let news: Vec<Entity> = plan
            .makes
            .iter()
            .map(|(layer, geometry, token)| {
                let base = EntityBase {
                    look: Default::default(),
                    id: 0,
                    layer_id: layer.clone(),
                    color: None,
                    attrs: BTreeMap::from([(plan.field.clone(), token.clone())]),
                    label: None,
                    symbol: None,
                    line_weight: None,
                    label_pins: Vec::new(),
                    attachments: Vec::new(),
                };
                crate::geometry::entity_of(geometry, base)
            })
            .collect();
        if !news.is_empty() {
            let slots = doc.add_many(news, label).map_err(|_| ())?;
            created = slots.iter().filter_map(|s| doc.uid(*s)).map(|u| u.to_string()).collect();
        }
        Ok(())
    });
    let output = output(&plan, created, cx.doc.revision().to_string());
    CommandResult::Completed { output, warnings: plan.warnings }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &FeaturesCode) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok((p, _)) => CommandResult::Completed { output: (), warnings: p.warnings },
        Err(stop) => stop.into(),
    }
}

/// What execute would do: the classes and counts; nothing made, nothing written.
pub fn plan(cx: &ExecutionContext<'_>, input: &FeaturesCode) -> CommandResult<FeaturesCoded> {
    match check(cx.doc, input) {
        Ok((p, _)) => {
            let output = output(&p, Vec::new(), cx.doc.revision().to_string());
            CommandResult::Completed { output, warnings: p.warnings }
        }
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: FeaturesCode) -> CommandResult<FeaturesCoded> {
    crate::call::recorded(cx, kentos_commands::MAP_FEATURES_CODE, kentos_commands::MAP_FEATURES_CODE_VERSION, input, write)
}
