//! `cad.pointclouds.place` v1 (plan/map-cad.md MAP-13, docs/adr/5100): Bulutu konumla. The map (16 numbers, an axis
//! and an angle, or Euler angles about Z, Y′, X″; the point cloud core's `Affine`) is taken after each file's place
//! (`then`, the core's fixed order) and the cloud written as `cad.entities.edit`'s `pointCloudPlace` writes it, its box
//! in the drawing the files' placed boxes together; one undo step “Bulutu konumla”. Konumu kaldır takes the places
//! away; on a cloud with none it writes nothing. The shared cases are `fixtures/commands/v1/cad.pointclouds.place.json`
//! (`scripts/fixtures/pointcloud_place_cases.py`).

use kentos_commands::{
    CloudPlaceBy, CommandResult, EditOperation, EntitiesEdit, EntitiesEditPlan, Entity, EntityEdit, EntityGeometry,
    PointCloudFields, PointCloudsPlace, PointCloudsPlaced,
};
use kentos_domain::{Document, Uuid};
use kentos_pointcloud::ops::transform::Affine;

use crate::checks::{self, Stop, error, is_uid_text};
use crate::{ExecutionContext, codes, edit};

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

/// The cloud: a point cloud of the drawing, not on a locked layer.
fn cloud<'d>(doc: &'d Document, uid: &str) -> Result<&'d PointCloudFields, Stop> {
    if !is_uid_text(uid) {
        return Err(refuse(
            codes::INVALID_UID,
            format!(
                "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
            ),
            "uid",
        ));
    }
    let entity = Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u)).and_then(|s| doc.get(s)).ok_or_else(|| {
        refuse(
            codes::ENTITY_NOT_FOUND,
            format!("“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nokta bulutunun kimliğini verin."),
            "uid",
        )
    })?;
    let Entity::PointCloud(c) = entity else {
        return Err(refuse(
            codes::NOT_A_POINTCLOUD,
            "Verilen nesne bir nokta bulutu değil; yalnız nokta bulutu konumlanır. Bir nokta bulutunun kimliğini verin.",
            "uid",
        ));
    };
    let layers = doc.layers();
    let layer = &entity.base().layer_id;
    if layers.is_locked(layer) {
        let name = layers.get(layer).map_or(layer.as_str(), |n| n.name.as_str());
        return Err(refuse(
            codes::LAYER_LOCKED,
            format!("“{name}” katmanı kilitli; üzerindeki bulut konumlanamaz. Kilidi Katmanlar panelinden açın."),
            "uid",
        ));
    }
    Ok(&c.cloud)
}

/// Every number finite (`not_finite` at `path`).
fn finite(values: &[f64], what: &str, path: &str) -> Result<(), Stop> {
    match values.iter().find(|v| !v.is_finite()) {
        Some(v) => checks::finite(*v, what, "Sonlu bir sayı verin.", path),
        None => Ok(()),
    }
}

/// The map given, about the middle of the cloud's box in the drawing when no centre is given; none for Konumu kaldır.
fn map_of(by: &CloudPlaceBy, c: &PointCloudFields) -> Result<Option<Affine>, Stop> {
    let b = c.bounds;
    let middle = [(b[0] + b[3]) / 2.0, (b[1] + b[4]) / 2.0, (b[2] + b[5]) / 2.0];
    let invalid = |e: kentos_pointcloud::PcError| refuse(codes::INVALID_INPUT, e.0, "place");
    let turn = |center: &Option<[f64; 3]>, translation: &Option<[f64; 3]>| -> Result<([f64; 3], [f64; 3]), Stop> {
        let (c, t) = (center.unwrap_or(middle), translation.unwrap_or([0.0; 3]));
        finite(&c, "Merkezin bir sayısı", "place.center")?;
        finite(&t, "Ötelemenin bir sayısı", "place.translation")?;
        Ok((c, t))
    };
    let map = match by {
        CloudPlaceBy::Remove => return Ok(None),
        CloudPlaceBy::Matrix { matrix } => {
            finite(matrix, "Matrisin bir sayısı", "place.matrix")?;
            let m: [f64; 16] = matrix.as_slice().try_into().map_err(|_| {
                refuse(
                    codes::INVALID_INPUT,
                    format!("Matris 16 sayıdır (4 satır, 4 sütun, satır satır); {} sayı verildi.", matrix.len()),
                    "place.matrix",
                )
            })?;
            Affine::new(m).map_err(invalid)?
        }
        CloudPlaceBy::AxisAngle { axis, angle, center, translation } => {
            finite(axis, "Eksenin bir sayısı", "place.axis")?;
            finite(&[*angle], "Açı", "place.angle")?;
            let (c, t) = turn(center, translation)?;
            Affine::axis_angle(*axis, *angle, c, t).map_err(invalid)?
        }
        CloudPlaceBy::Euler { yaw, pitch, roll, center, translation } => {
            finite(&[*yaw], "Sapma", "place.yaw")?;
            finite(&[*pitch], "Yunuslama", "place.pitch")?;
            finite(&[*roll], "Yatış", "place.roll")?;
            let (c, t) = turn(center, translation)?;
            Affine::euler(*yaw, *pitch, *roll, c, t).map_err(invalid)?
        }
    };
    if let Some(why) = kentos_commands::pose_problem(&map.matrix()) {
        return Err(refuse(codes::INVALID_INPUT, why, "place"));
    }
    Ok(Some(map))
}

/// The cloud as it would be written, what the answer says, and the change; no change for Konumu kaldır on a cloud
/// with no place.
struct Prepared {
    out: PointCloudsPlaced,
    change: Option<EntityEdit>,
}

fn prepare(doc: &Document, input: &PointCloudsPlace) -> Result<Prepared, Stop> {
    let c = cloud(doc, &input.uid)?;
    let given = map_of(&input.place, c)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let b = c.bounds;
    let middle = [(b[0] + b[3]) / 2.0, (b[1] + b[4]) / 2.0, (b[2] + b[5]) / 2.0];
    let mut fields = c.clone();
    // Konumu kaldır: its map is the first file's place taken back (the identity when it has none).
    let map = match given {
        Some(m) => {
            for s in &mut fields.sources {
                let had = s.pose.map_or_else(|| Affine::new(IDENTITY), Affine::new);
                let had = had.map_err(|e| refuse(codes::INVALID_INPUT, e.0, "uid"))?;
                s.pose = Some(had.then(&m).matrix());
            }
            m
        }
        None => {
            let first = c.sources.iter().find_map(|s| s.pose).unwrap_or(IDENTITY);
            for s in &mut fields.sources {
                s.pose = None;
            }
            Affine::new(first).map_err(|e| refuse(codes::INVALID_INPUT, e.0, "uid"))?.inverse()
        }
    };
    fields.bounds = fields.placed_bounds();
    let changed = fields != *c;
    let out = PointCloudsPlaced {
        uid: input.uid.clone(),
        matrix: map.matrix().to_vec(),
        poses: fields.sources.iter().map(|s| s.pose.map(|m| m.to_vec())).collect(),
        bounds: fields.bounds,
        shift: map.shift(middle),
        angle: map.angle(),
        changed,
        revision: doc.revision().to_string(),
    };
    let change = changed.then(|| EntityEdit::Update { uid: input.uid.clone(), geometry: EntityGeometry::PointCloud(fields) });
    Ok(Prepared { out, change })
}

/// The 4 × 4 identity, row by row.
const IDENTITY: [f64; 16] = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];

fn edit_input(change: EntityEdit, input: &PointCloudsPlace) -> EntitiesEdit {
    EntitiesEdit {
        operation: EditOperation::PointCloudPlace,
        changes: vec![change],
        expected_revision: input.expected_revision.clone(),
    }
}

/// The map's checks and the writing command's.
pub fn validate(cx: &ExecutionContext<'_>, input: &PointCloudsPlace) -> CommandResult<()> {
    match prepare(cx.doc, input) {
        Ok(Prepared { change: Some(change), .. }) => edit::validate(cx, &edit_input(change, input)),
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The cloud as it would be written.
pub fn plan(cx: &ExecutionContext<'_>, input: &PointCloudsPlace) -> CommandResult<EntitiesEditPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] over the document.
pub fn plan_in(doc: &Document, input: &PointCloudsPlace) -> CommandResult<EntitiesEditPlan> {
    match prepare(doc, input) {
        Ok(Prepared { change: Some(change), .. }) => edit::plan_in(doc, &edit_input(change, input)),
        Ok(_) => CommandResult::Completed {
            output: EntitiesEditPlan {
                changed: Vec::new(),
                created: Vec::new(),
                removed: Vec::new(),
                revision: doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// Places the cloud, one undo step “Bulutu konumla”.
pub fn execute(cx: &mut ExecutionContext<'_>, input: PointCloudsPlace) -> CommandResult<PointCloudsPlaced> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_POINTCLOUDS_PLACE,
        kentos_commands::CAD_POINTCLOUDS_PLACE_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: PointCloudsPlace) -> CommandResult<PointCloudsPlaced> {
    let Prepared { mut out, change } = match prepare(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    let Some(change) = change else {
        return CommandResult::Completed { output: out, warnings: Vec::new() };
    };
    match edit::write_in(cx, edit_input(change, &input)) {
        CommandResult::Completed { output, warnings } => {
            out.revision = output.revision;
            CommandResult::Completed { output: out, warnings }
        }
        other => crate::trim::relay(other),
    }
}
