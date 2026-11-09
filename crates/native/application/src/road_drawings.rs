//! What a road project's commands drew (docs/adr/0911 §1): the nine commands that draw keep, in the step that
//! writes the objects, a drawing in the project's road settings: the command and its input, the sources it was made
//! from with their stamps, the objects in order and their stamp. How a drawing stands now is read from it: its
//! sources' stamps then and now, its objects then and now.
//!
//! A source's stamp is 16 hex digits of the SHA-256 of a canonical JSON (docs/adr/0253 §2): a surface's, of its
//! sources' digest (what `sources::gather` reads and the method: what the surface is made of, not whether it was
//! accepted); the others', of the definition itself, its every number written as a float (a definition that went
//! through a file and came back with `20` for `20.0` is the same). The closure: a profile names its alignment (an
//! offset alignment its base too) and its surface; a sample line set its alignment, profile, template, surface and
//! superelevation; a superelevation its alignment; a corridor its alignment, profile, its regions' templates, surface
//! and superelevation; a guardrail, culvert and wall its corridor; a junction its legs' alignments. A utility network's
//! (docs/adr/3560), of its definition and the objects of its edge and junction layers: a pipe run profile follows its
//! network, the surface its ground is read from and the network whose pipes cross it.

use std::collections::BTreeMap;

use kentos_commands::{CommandResult, RoadDrawingDef, RoadDrawingSource, RoadSourceKind};
use kentos_commands::canonical::stamp_of;
use kentos_domain::{Document, Uuid};
use serde::Serialize;
use serde_json::{Number, Value};

use crate::checks::{Stop, error};
use crate::codes;
use crate::ExecutionContext;

/// `value` with its every number a float.
fn floats(value: Value) -> Value {
    match value {
        Value::Number(n) => n.as_f64().and_then(Number::from_f64).map_or(Value::Null, Value::Number),
        Value::Array(a) => Value::Array(a.into_iter().map(floats).collect()),
        Value::Object(o) => Value::Object(o.into_iter().map(|(k, v)| (k, floats(v))).collect()),
        other => other,
    }
}

/// The stamp of `value`'s JSON, its numbers floats.
pub(crate) fn stamp<T: Serialize>(value: &T) -> String {
    stamp_of(&floats(serde_json::to_value(value).unwrap_or(Value::Null)))
}

fn find<'a, T>(list: &'a [T], id: &str, of: impl Fn(&T) -> &str) -> Option<&'a T> {
    list.iter().find(|x| of(x) == id)
}

/// A utility network's stamp (docs/adr/3560): of its definition and the objects of its edge and junction layers as
/// they are, in the drawing's order. A layer's filter is not applied: an object it leaves out now may count later.
fn network_stamp(doc: &Document, id: &str) -> Option<String> {
    let def = doc.settings().networks.iter().find(|n| n.id == id)?;
    let layers: std::collections::BTreeSet<&str> =
        def.edges.iter().map(|l| l.layer.as_str()).chain(def.junctions.iter().map(|l| l.layer.as_str())).collect();
    let objects: Vec<Value> = doc
        .entities()
        .filter(|e| layers.contains(e.base().layer_id.as_str()))
        .map(|e| serde_json::to_value(e).unwrap_or(Value::Null))
        .collect();
    Some(stamp(&serde_json::json!({ "network": def, "objects": objects })))
}

/// A source's stamp now; none when the project has no such source.
pub(crate) fn stamp_now(doc: &Document, kind: RoadSourceKind, id: &str) -> Option<String> {
    if kind == RoadSourceKind::Network {
        return network_stamp(doc, id);
    }
    let road = doc.settings().road.as_ref()?;
    Some(match kind {
        RoadSourceKind::Surface => {
            let def = find(&road.surfaces, id, |s| &s.id)?;
            crate::road_terrains::sources_digest(doc, def)[..16].to_owned()
        }
        RoadSourceKind::Alignment => stamp(find(&road.alignments, id, |s| &s.id)?),
        RoadSourceKind::Profile => stamp(find(&road.profiles, id, |s| &s.id)?),
        RoadSourceKind::Template => stamp(find(&road.templates, id, |s| &s.id)?),
        RoadSourceKind::Sections => stamp(find(&road.sections, id, |s| &s.id)?),
        RoadSourceKind::Superelevation => stamp(find(&road.superelevations, id, |s| &s.id)?),
        RoadSourceKind::Corridor => stamp(find(&road.corridors, id, |s| &s.id)?),
        RoadSourceKind::Guardrail => stamp(find(&road.guardrails, id, |s| &s.id)?),
        RoadSourceKind::Culvert => stamp(find(&road.culverts, id, |s| &s.id)?),
        RoadSourceKind::Wall => stamp(find(&road.walls, id, |s| &s.id)?),
        RoadSourceKind::Junction => stamp(find(&road.junctions, id, |s| &s.id)?),
        RoadSourceKind::Platform => stamp(find(&road.platforms, id, |s| &s.id)?),
        RoadSourceKind::Network => return None,
    })
}

/// The sources a drawing made from `kind` `id` depends on: it first, then what it names, each once.
pub(crate) fn sources(doc: &Document, kind: RoadSourceKind, id: &str) -> Vec<RoadDrawingSource> {
    let mut out = Vec::new();
    add(doc, kind, id, &mut out);
    out
}

fn add(doc: &Document, kind: RoadSourceKind, id: &str, out: &mut Vec<RoadDrawingSource>) {
    use RoadSourceKind as K;
    if out.iter().any(|s| s.kind == kind && s.id == id) {
        return;
    }
    let Some(digest) = stamp_now(doc, kind, id) else { return };
    out.push(RoadDrawingSource { kind, id: id.to_owned(), digest });
    let Some(road) = doc.settings().road.as_ref() else { return };
    let mut named: Vec<(K, String)> = Vec::new();
    match kind {
        K::Surface | K::Template | K::Network => {}
        K::Alignment => {
            if let Some(o) = find(&road.alignments, id, |s| &s.id).and_then(|a| a.offset_of.as_ref()) {
                named.push((K::Alignment, o.alignment.clone()));
            }
        }
        K::Profile => {
            if let Some(p) = find(&road.profiles, id, |s| &s.id) {
                named.push((K::Alignment, p.alignment.clone()));
                named.extend(p.surface.iter().map(|s| (K::Surface, s.clone())));
            }
        }
        K::Sections => {
            if let Some(s) = find(&road.sections, id, |s| &s.id) {
                named.push((K::Alignment, s.alignment.clone()));
                named.extend(s.profile.iter().map(|x| (K::Profile, x.clone())));
                named.extend(s.template.iter().map(|x| (K::Template, x.clone())));
                named.extend(s.surface.iter().map(|x| (K::Surface, x.clone())));
                named.extend(s.superelevation.iter().map(|x| (K::Superelevation, x.clone())));
            }
        }
        K::Superelevation => {
            if let Some(s) = find(&road.superelevations, id, |s| &s.id) {
                named.push((K::Alignment, s.alignment.clone()));
            }
        }
        K::Corridor => {
            if let Some(c) = find(&road.corridors, id, |s| &s.id) {
                named.push((K::Alignment, c.alignment.clone()));
                named.push((K::Profile, c.profile.clone()));
                for r in &c.regions {
                    named.push((K::Template, r.template.clone()));
                }
                named.push((K::Surface, c.surface.clone()));
                named.extend(c.superelevation.iter().map(|x| (K::Superelevation, x.clone())));
            }
        }
        K::Guardrail => named.extend(find(&road.guardrails, id, |s| &s.id).map(|g| (K::Corridor, g.corridor.clone()))),
        K::Culvert => named.extend(find(&road.culverts, id, |s| &s.id).map(|g| (K::Corridor, g.corridor.clone()))),
        K::Wall => named.extend(find(&road.walls, id, |s| &s.id).map(|g| (K::Corridor, g.corridor.clone()))),
        K::Junction => {
            if let Some(j) = find(&road.junctions, id, |s| &s.id) {
                named.extend(j.legs.iter().map(|l| (K::Alignment, l.alignment.clone())));
            }
        }
        K::Platform => named.extend(find(&road.platforms, id, |s| &s.id).map(|p| (K::Surface, p.surface.clone()))),
    }
    for (k, i) in named {
        add(doc, k, &i, out);
    }
}

/// The objects `uids` as they are now, in order (an absent one `null`), each without its number in this document, and
/// how many are present; the stamp of the list as `numbered` asks: an object's `id` is renumbered when the file is read
/// again (docs/adr/3560: a drawing drawn again, or an object before it deleted, saved and opened read as changed by
/// hand), its uid is its identity.
fn objects_list(doc: &Document, uids: &[String], numbered: bool) -> (String, usize) {
    let mut present = 0;
    let list: Vec<Value> = uids
        .iter()
        .map(|u| {
            let entity = Uuid::parse_str(u).ok().and_then(|u| doc.slot_of(u)).and_then(|s| doc.get(s));
            present += usize::from(entity.is_some());
            entity.map_or(Value::Null, |e| {
                let mut v = serde_json::to_value(e).unwrap_or(Value::Null);
                if !numbered && let Value::Object(o) = &mut v {
                    o.remove("id");
                }
                v
            })
        })
        .collect();
    (stamp(&Value::Array(list)), present)
}

/// The objects `uids` as they are now stamped (their numbers in this document left out), and how many are present.
pub(crate) fn objects_stamp(doc: &Document, uids: &[String]) -> (String, usize) {
    objects_list(doc, uids, false)
}

/// The stamp a record kept before docs/adr/3560 wrote: the objects' numbers in it. Such a record is not changed by hand
/// while this still matches; drawn again, it takes the stamp without them.
pub(crate) fn objects_stamp_numbered(doc: &Document, uids: &[String]) -> String {
    objects_list(doc, uids, true).0
}

/// A drawing command's main source: its kind and the input's field naming it.
pub(crate) fn main_of(command: &str) -> Option<(RoadSourceKind, &'static str)> {
    use RoadSourceKind as K;
    Some(match command {
        "road.alignments.label" => (K::Alignment, "alignment"),
        "road.profiles.draw" => (K::Profile, "profile"),
        "road.sections.draw" => (K::Sections, "set"),
        "road.corridors.draw" => (K::Corridor, "corridor"),
        "road.guardrails.draw" => (K::Guardrail, "guardrail"),
        "road.culverts.draw" => (K::Culvert, "culvert"),
        "road.walls.draw" => (K::Wall, "wall"),
        "road.junctions.draw" => (K::Junction, "junction"),
        "road.platforms.draw" => (K::Platform, "platform"),
        "road.terrains.contours" => (K::Surface, "surface"),
        "road.terrains.zeroline" => (K::Surface, "base"),
        "road.terrains.cutfill" => (K::Surface, "base"),
        "cad.pipes.profile" => (K::Network, "network"),
        _ => return None,
    })
}

/// The version of a drawing command that draws it again (docs/adr/1940): a record keeps none; every drawing command is
/// its v1.
pub(crate) fn version_of(command: &str) -> u32 {
    match command {
        "road.alignments.label" => kentos_commands::ROAD_ALIGNMENTS_LABEL_VERSION,
        "road.profiles.draw" => kentos_commands::ROAD_PROFILES_DRAW_VERSION,
        "road.sections.draw" => kentos_commands::ROAD_SECTIONS_DRAW_VERSION,
        "road.corridors.draw" => kentos_commands::ROAD_CORRIDORS_DRAW_VERSION,
        "road.guardrails.draw" => kentos_commands::ROAD_GUARDRAILS_DRAW_VERSION,
        "road.culverts.draw" => kentos_commands::ROAD_CULVERTS_DRAW_VERSION,
        "road.walls.draw" => kentos_commands::ROAD_WALLS_DRAW_VERSION,
        "road.junctions.draw" => kentos_commands::ROAD_JUNCTIONS_DRAW_VERSION,
        "road.platforms.draw" => kentos_commands::ROAD_PLATFORMS_DRAW_VERSION,
        "road.terrains.contours" => kentos_commands::ROAD_TERRAINS_CONTOURS_VERSION,
        "road.terrains.zeroline" => kentos_commands::ROAD_TERRAINS_ZEROLINE_VERSION,
        "road.terrains.cutfill" => kentos_commands::ROAD_TERRAINS_CUTFILL_VERSION,
        "cad.pipes.profile" => kentos_commands::CAD_PIPES_PROFILE_VERSION,
        _ => 1,
    }
}

/// The (id, name) of the project's sources of `kind`.
fn named(doc: &Document, kind: RoadSourceKind) -> Vec<(String, String)> {
    use RoadSourceKind as K;
    if kind == K::Network {
        return doc.settings().networks.iter().map(|n| (n.id.clone(), n.name.clone())).collect();
    }
    let Some(r) = doc.settings().road.as_ref() else { return Vec::new() };
    fn pairs<T>(list: &[T], f: impl Fn(&T) -> (&String, &String)) -> Vec<(String, String)> {
        list.iter().map(f).map(|(i, n)| (i.clone(), n.clone())).collect()
    }
    match kind {
        K::Surface => pairs(&r.surfaces, |x| (&x.id, &x.name)),
        K::Alignment => pairs(&r.alignments, |x| (&x.id, &x.name)),
        K::Profile => pairs(&r.profiles, |x| (&x.id, &x.name)),
        K::Template => pairs(&r.templates, |x| (&x.id, &x.name)),
        K::Sections => pairs(&r.sections, |x| (&x.id, &x.name)),
        K::Superelevation => pairs(&r.superelevations, |x| (&x.id, &x.name)),
        K::Corridor => pairs(&r.corridors, |x| (&x.id, &x.name)),
        K::Guardrail => pairs(&r.guardrails, |x| (&x.id, &x.name)),
        K::Culvert => pairs(&r.culverts, |x| (&x.id, &x.name)),
        K::Wall => pairs(&r.walls, |x| (&x.id, &x.name)),
        K::Junction => pairs(&r.junctions, |x| (&x.id, &x.name)),
        K::Platform => pairs(&r.platforms, |x| (&x.id, &x.name)),
        K::Network => Vec::new(),
    }
}

/// The id of the source of `kind` a command names by its id or, ignoring case, its name (as the commands find it).
pub(crate) fn resolve(doc: &Document, kind: RoadSourceKind, text: &str) -> Option<String> {
    let all = named(doc, kind);
    let low = text.to_lowercase();
    all.iter().find(|(i, _)| i == text).or_else(|| all.iter().find(|(_, n)| n.to_lowercase() == low)).map(|(i, _)| i.clone())
}

/// A source's name now.
pub(crate) fn name_of(doc: &Document, kind: RoadSourceKind, id: &str) -> Option<String> {
    named(doc, kind).into_iter().find(|(i, _)| i == id).map(|(_, n)| n)
}

/// The input as a drawing keeps it: its main source by id (a later rename does not lose it), without its expected
/// revision (drawing it again does not wait for that one).
fn kept_input<I: Serialize>(input: &I, field: &str, id: &str) -> BTreeMap<String, Value> {
    let mut o: BTreeMap<String, Value> = match serde_json::to_value(input) {
        Ok(Value::Object(o)) => o.into_iter().filter(|(k, _)| k != "expectedRevision").collect(),
        _ => BTreeMap::new(),
    };
    o.insert(field.to_owned(), Value::String(id.to_owned()));
    o
}

/// The next drawing id: `cz-1`, `cz-2` … (çizim) the first not taken.
fn next_id(list: &[RoadDrawingDef]) -> String {
    (1..).map(|n| format!("cz-{n}")).find(|id| list.iter().all(|d| &d.id != id)).unwrap_or_default()
}

/// Keeps what `command` drew from the source of `kind` its input's `field` names: its objects `uids`, as a drawing of
/// the project (a new one, or in place of `replacing`). Answers its id.
pub(crate) fn keep<I: Serialize>(
    doc: &mut Document,
    command: &str,
    input: &I,
    (kind, field): (RoadSourceKind, &str),
    uids: &[String],
    replacing: Option<&str>,
) -> Result<String, Stop> {
    let named = serde_json::to_value(input).ok().and_then(|v| v.get(field).and_then(Value::as_str).map(str::to_owned));
    let Some(main) = named.and_then(|t| resolve(doc, kind, &t)) else {
        return Err(Stop::failed(error(codes::ROAD_DRAWINGS_FULL, "Çizimin kaynağı projede bulunamadı.".into(), Some(field.into()))));
    };
    let mut list = doc.road_drawings().to_vec();
    let id = replacing.map_or_else(|| next_id(&list), str::to_owned);
    let drawing = RoadDrawingDef {
        id: id.clone(),
        command: command.to_owned(),
        input: kept_input(input, field, &main),
        sources: {
            let mut all = sources(doc, kind, &main);
            // Docs/adr/0980: a profile drawn with KGM's Dever band follows its superelevation schedule too.
            let dever = serde_json::to_value(input).ok().and_then(|v| v.get("superelevation").and_then(Value::as_str).map(str::to_owned));
            if command == "road.profiles.draw"
                && let Some(id) = dever.and_then(|t| resolve(doc, RoadSourceKind::Superelevation, &t))
            {
                add(doc, RoadSourceKind::Superelevation, &id, &mut all);
            }
            // Docs/adr/1720, 1840: a zero line and cut and fill zones follow the comparison surface as well as their base.
            let comparison =
                serde_json::to_value(input).ok().and_then(|v| v.get("comparison").and_then(Value::as_str).map(str::to_owned));
            if matches!(command, "road.terrains.zeroline" | "road.terrains.cutfill")
                && let Some(id) = comparison.and_then(|t| resolve(doc, RoadSourceKind::Surface, &t))
            {
                add(doc, RoadSourceKind::Surface, &id, &mut all);
            }
            // Docs/adr/3560: a pipe run profile follows the surface its ground is read from and the network whose pipes
            // cross it as well as its own network.
            if command == "cad.pipes.profile" {
                let v = serde_json::to_value(input).unwrap_or(Value::Null);
                let ground = v.pointer("/ground/surface").and_then(Value::as_str).map(str::to_owned);
                if let Some(id) = ground.and_then(|t| resolve(doc, RoadSourceKind::Surface, &t)) {
                    add(doc, RoadSourceKind::Surface, &id, &mut all);
                }
                let crossing = v.pointer("/crossing/network").and_then(Value::as_str).map(str::to_owned);
                if let Some(id) = crossing.and_then(|t| resolve(doc, RoadSourceKind::Network, &t)) {
                    add(doc, RoadSourceKind::Network, &id, &mut all);
                }
            }
            all
        },
        uids: uids.to_vec(),
        digest: objects_stamp(doc, uids).0,
        // Docs/adr/2840: the version that drew it, and the revision its write makes: inside the command's step the
        // document's revision is already the one the step records.
        version: Some(version_of(command)),
        revision: Some(doc.revision().to_string()),
    };
    match list.iter_mut().find(|d| d.id == id) {
        Some(d) => *d = drawing,
        None => list.push(drawing),
    }
    doc.set_road_drawings(list, "Yol çizimi")
        .map_err(|e| Stop::failed(error(codes::ROAD_DRAWINGS_FULL, format!("Çizim kaydedilemedi: {}.", e.0), None)))?;
    Ok(id)
}

/// `result` of a command that drew, its drawing kept when it completed (the step undoes both when it cannot be).
pub(crate) fn kept<I: Serialize, O>(
    cx: &mut ExecutionContext<'_>,
    command: &str,
    input: &I,
    main: (RoadSourceKind, &str),
    result: CommandResult<O>,
    uids: impl Fn(&O) -> &[String],
) -> CommandResult<O> {
    if let CommandResult::Completed { output, .. } = &result
        && let Err(stop) = keep(cx.doc, command, input, main, uids(output), None)
    {
        return stop.into();
    }
    result
}
