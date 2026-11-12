//! `cad.entities.flatten` v1 (docs/adr/1061 §4): 2B düzleme indir. The elevations of points, lines, polylines and
//! areas (their parts and holes too) removed or set to one level, and with `measures` their measures removed, as one
//! undo step; the plan is the preview (which objects, how many vertices, between which elevations). The shared cases
//! are `fixtures/commands/v1/cad.entities.flatten.json`.

use kentos_commands::{
    CommandResult, CommandWarning, EntitiesFlatten, EntitiesFlattened, Entity, FLATTEN_STEP,
    PathEntity,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;

/// What a flatten counts while it goes.
#[derive(Default)]
struct Tally {
    vertices: u32,
    measures: u32,
    low: Option<f64>,
    high: Option<f64>,
}

impl Tally {
    /// One elevation: removed (`plane` none) or set to `plane`; counted when it changes.
    fn z(&mut self, z: Option<f64>, plane: Option<f64>) -> Option<f64> {
        let z = z?;
        if plane != Some(z) {
            self.vertices += 1;
            self.low = Some(
                self.low
                    .map_or(z, |l| kentos_geometry_core::jsmath::js_min(l, z)),
            );
            self.high = Some(
                self.high
                    .map_or(z, |h| kentos_geometry_core::jsmath::js_max(h, z)),
            );
        }
        plane
    }

    fn zs(
        &mut self,
        zs: &Option<Vec<Option<f64>>>,
        plane: Option<f64>,
    ) -> Option<Vec<Option<f64>>> {
        let out: Vec<Option<f64>> = zs.as_ref()?.iter().map(|z| self.z(*z, plane)).collect();
        out.iter().any(Option::is_some).then_some(out)
    }

    fn ms(&mut self, ms: &Option<Vec<Option<f64>>>, drop: bool) -> Option<Vec<Option<f64>>> {
        if !drop {
            return ms.clone();
        }
        self.measures += ms
            .as_ref()
            .map_or(0, |m| m.iter().filter(|x| x.is_some()).count() as u32);
        None
    }

    fn m(&mut self, m: Option<f64>, drop: bool) -> Option<f64> {
        if drop && m.is_some() {
            self.measures += 1;
            return None;
        }
        m
    }
}

fn path(p: &PathEntity, plane: Option<f64>, measures: bool, t: &mut Tally) -> PathEntity {
    let mut out = p.clone();
    out.zs = t.zs(&p.zs, plane);
    out.ms = t.ms(&p.ms, measures);
    if let Some(holes) = out.holes.as_mut() {
        for h in holes {
            h.zs = t.zs(&h.zs, plane);
        }
    }
    if let Some(parts) = out.parts.as_mut() {
        for part in parts {
            part.zs = t.zs(&part.zs, plane);
            part.ms = t.ms(&part.ms, measures);
            if let Some(holes) = part.holes.as_mut() {
                for h in holes {
                    h.zs = t.zs(&h.zs, plane);
                }
            }
        }
    }
    out
}

/// The object flattened, or none when it holds nothing to change.
fn flattened(e: &Entity, plane: Option<f64>, measures: bool, t: &mut Tally) -> Option<Entity> {
    let out = match e {
        Entity::Point(p) => {
            let mut q = p.clone();
            q.z = t.z(p.z, plane);
            if let Some(parts) = q.parts.as_mut() {
                for part in parts {
                    part.z = t.z(part.z, plane);
                }
            }
            Entity::Point(q)
        }
        Entity::Line(l) => {
            let mut q = l.clone();
            q.za = t.z(l.za, plane);
            q.zb = t.z(l.zb, plane);
            q.ma = t.m(l.ma, measures);
            q.mb = t.m(l.mb, measures);
            Entity::Line(q)
        }
        Entity::Polyline(p) => Entity::Polyline(path(p, plane, measures, t)),
        Entity::Polygon(p) => Entity::Polygon(path(p, plane, measures, t)),
        _ => return None,
    };
    (&out != e).then_some(out)
}

fn refused(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(checks::error(code, message.into(), Some(path.into())))
}

type Checked = (Vec<(Slot, String, Entity)>, Tally, Vec<CommandWarning>);

fn check(doc: &Document, input: &EntitiesFlatten) -> Result<Checked, Stop> {
    checks::uids(&input.uids, "Düzleme indirilecek nesne verilmedi.")?;
    if input.plane.is_some_and(|z| !z.is_finite()) {
        return Err(refused(
            codes::NOT_FINITE,
            "Düzlemin kotu sayı değil ya da sonsuz.",
            "plane",
        ));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let named = checks::named(doc, &input.uids)?;
    let (named, warnings) = checks::skip_locked(doc, named)?;
    let mut tally = Tally::default();
    let mut out = Vec::new();
    for (_, slot, e, uid) in named {
        if let Some(next) = flattened(e, input.plane, input.measures, &mut tally) {
            out.push((slot, uid.clone(), next));
        }
    }
    if out.is_empty() {
        return Err(refused(
            codes::NOTHING_TO_FLATTEN,
            if input.measures {
                "Seçilen nesnelerde kaldırılacak kot (Z) ya da ölçü (M) yok; çizim zaten düzlemde."
            } else {
                "Seçilen nesnelerde kaldırılacak ya da değişecek kot (Z) yok; çizim zaten düzlemde. Ölçüleri (M) de kaldırmak için measures: true verin."
            },
            "uids",
        ));
    }
    Ok((out, tally, warnings.into_iter().collect()))
}

fn answer(doc: &Document, changes: &[(Slot, String, Entity)], t: &Tally) -> EntitiesFlattened {
    EntitiesFlattened {
        changed: changes.iter().map(|(_, u, _)| u.clone()).collect(),
        ids: changes.iter().map(|(s, _, _)| s.0).collect(),
        vertices: t.vertices,
        measures: t.measures,
        z_min: t.low,
        z_max: t.high,
        revision: doc.revision().to_string(),
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesFlatten) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok((_, _, warnings)) => CommandResult::Completed {
            output: (),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// The preview: what would go, nothing written.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &EntitiesFlatten,
) -> CommandResult<EntitiesFlattened> {
    match check(cx.doc, input) {
        Ok((changes, tally, warnings)) => CommandResult::Completed {
            output: answer(cx.doc, &changes, &tally),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: EntitiesFlatten,
) -> CommandResult<EntitiesFlattened> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_ENTITIES_FLATTEN,
        kentos_commands::CAD_ENTITIES_FLATTEN_VERSION,
        input,
        |cx, input| {
            let (changes, tally, warnings) = match check(cx.doc, &input) {
                Ok(c) => c,
                Err(stop) => return stop.into(),
            };
            let writes = changes.iter().map(|(s, _, e)| (*s, e.clone())).collect();
            cx.doc.update_many(writes, FLATTEN_STEP);
            CommandResult::Completed {
                output: answer(cx.doc, &changes, &tally),
                warnings,
            }
        },
    )
}
