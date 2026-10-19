//! `cad.stac.prepare` v1 (docs/adr/0528 §4–§6; plan/gis.md `CBS-31`): the assets of a STAC item the caller chose,
//! added as rasters linked by address, in one new group with a layer per asset, as one undo step “STAC verisini
//! hazırla”. What is planned (the grid, data type, nodata, scale and offset, the quality band and its mask, the
//! record) is the services core's (`kentos_services::stac::prepare`); this writes it. No file is opened and no
//! network is used: the catalog's own words place the rasters. The record travels in the command's answer and in
//! each raster's attributes (`stac.*`).
//! The shared cases are `fixtures/commands/v1/cad.stac.prepare.json` (`scripts/fixtures/stac_cases.py`).

use kentos_contracts::{
    CommandResult, CommandWarning, Entity, EntityBase, RasterEntity, StacPrepare, StacPrepared,
};
use kentos_domain::{Document, NewLayer};
use kentos_services::stac::prepare::{self, Prepared, RasterPlan};

use crate::ExecutionContext;
use crate::checks::{self, Stop};

fn check(doc: &Document, input: &StacPrepare) -> Result<Prepared, Stop> {
    let planned = prepare::plan(input, doc.settings().srid)
        .map_err(|p| Stop::failed(checks::error(p.code, p.message, p.path)))?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(planned)
}

fn warnings_of(p: &Prepared) -> Vec<CommandWarning> {
    p.warnings
        .iter()
        .map(|(code, message, path)| CommandWarning {
            code: (*code).into(),
            message: message.clone(),
            path: Some(path.clone()),
        })
        .collect()
}

fn answer(doc: &Document, p: &Prepared) -> StacPrepared {
    StacPrepared {
        preparation: p.preparation.clone(),
        group: None,
        revision: doc.revision().to_string(),
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &StacPrepare) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed {
            output: (),
            warnings: warnings_of(&p),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &StacPrepare) -> CommandResult<StacPrepared> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed {
            output: answer(cx.doc, &p),
            warnings: warnings_of(&p),
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the group, the layers and the rasters as one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: StacPrepare) -> CommandResult<StacPrepared> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_STAC_PREPARE,
        kentos_contracts::CAD_STAC_PREPARE_VERSION,
        input,
        write,
    )
}

fn raster(layer_id: &str, plan: &RasterPlan) -> Entity {
    Entity::Raster(RasterEntity {
        base: EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: layer_id.to_owned(),
            color: None,
            attrs: plan.attrs.clone(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        raster: plan.fields.clone(),
    })
}

/// A raster's layer, then the raster on it: the layer's id and the raster's persistent id.
fn put(doc: &mut Document, group: &str, plan: &RasterPlan) -> Result<(String, String), Stop> {
    let layer = doc
        .add_layer(NewLayer::layer(plan.layer.clone()), Some(group), false)
        .map_err(|r| {
            Stop::failed(checks::error(
                kentos_contracts::codes::INVALID_GROUP_NAME,
                r.to_string(),
                None,
            ))
        })?;
    let (_, uid, _) = checks::add(doc, raster(&layer, plan))?;
    Ok((layer, uid))
}

fn write(cx: &mut ExecutionContext<'_>, input: StacPrepare) -> CommandResult<StacPrepared> {
    let planned = match check(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    let warnings = warnings_of(&planned);
    let written = cx.doc.transact("STAC verisini hazırla", |doc| {
        let group = doc
            .add_layer(NewLayer::group(planned.group.clone()), None, false)
            .map_err(|r| {
                Stop::failed(checks::error(
                    kentos_contracts::codes::INVALID_GROUP_NAME,
                    r.to_string(),
                    Some("group".into()),
                ))
            })?;
        let mut done = Vec::new();
        for band in &planned.bands {
            done.push(put(doc, &group, band)?);
        }
        let qa = match &planned.qa {
            Some(q) => Some(put(doc, &group, q)?),
            None => None,
        };
        Ok::<_, Stop>((group, done, qa))
    });
    let (group, bands, qa) = match written {
        Ok(w) => w,
        Err(stop) => return stop.into(),
    };
    let mut out = answer(cx.doc, &planned);
    out.group = Some(group);
    for (b, (layer, uid)) in out.preparation.bands.iter_mut().zip(bands) {
        b.layer = Some(layer);
        b.uid = Some(uid);
    }
    if let (Some(q), Some((layer, uid))) = (out.preparation.qa.as_mut(), qa) {
        q.layer = Some(layer);
        q.uid = Some(uid);
    }
    CommandResult::Completed {
        output: out,
        warnings,
    }
}
