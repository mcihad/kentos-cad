//! `cad.masses.create` (docs/adr/8178): masses, a massing study's prisms, each its footprint, height and bottom over
//! its storey, its name. A mass's rings and height follow a space's rules; its refusal is its own, `invalid_mass`.

use super::*;
use kentos_commands::{MassesCreated, MassesPlan};

/// The undo step's name.
pub const MASS_STEP: &str = "Kütle";

/// `cad.masses.create`'s check: each mass's numbers, its fields, its level.
fn masses_check(doc: &Document, input: &kentos_commands::MassesCreate) -> Result<Checked, Stop> {
    if input.masses.is_empty() {
        return Err(fail(
            codes::NO_OBJECTS,
            "Yazılacak kütle verilmedi. En az bir kütlenin oturumunu ve yüksekliğini verin.".into(),
            "masses",
        ));
    }
    too_many(input.masses.len(), codes::INVALID_MASS, "kütle", "masses")?;
    let mut made: Vec<Made> = Vec::new();
    for (i, n) in input.masses.iter().enumerate() {
        let at = format!("masses[{i}]");
        for (k, part) in n.parts.iter().enumerate() {
            points(&part.outer.pts, &format!("{at}.parts[{k}].outer"))?;
            for (h, hole) in part.holes.iter().enumerate() {
                points(&hole.pts, &format!("{at}.parts[{k}].holes[{h}]"))?;
            }
        }
        finite_each(
            &[
                (Some(n.height), "height", "Yükseklik"),
                (n.offset, "offset", "Kot farkı"),
            ],
            &at,
        )?;
        let m = kentos_commands::MassFields {
            parts: n.parts.clone(),
            height: n.height,
            offset: n.offset.unwrap_or(0.0),
            arch: n.arch.clone().unwrap_or_default(),
            name: n.name.clone(),
        };
        if let Some((field, words)) = m.problem() {
            return Err(fail(codes::INVALID_MASS, words, &format!("{at}.{field}")));
        }
        crate::walls::level_known(doc, &m.arch, &format!("{at}.arch.level"))?;
        made.push((
            at,
            n.attrs.clone(),
            Box::new(move |base| {
                Entity::Mass(kentos_commands::MassEntity {
                    base,
                    mass: Box::new(m),
                })
            }),
        ));
    }
    finish(
        doc,
        &input.layer_id,
        input.line_weight,
        input.expected_revision.as_deref(),
        made,
        &input.color,
        Vec::new(),
    )
}

create_command!(
    masses_create,
    kentos_commands::MassesCreate,
    MassesPlan,
    masses,
    MassesCreated,
    masses_check,
    MASS_STEP,
    CAD_MASSES_CREATE,
    CAD_MASSES_CREATE_VERSION
);
