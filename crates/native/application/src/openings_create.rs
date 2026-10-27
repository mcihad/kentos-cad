//! `cad.openings.create` v1 (docs/adr/0341 §4, plan/cad.md `CAD-ARCH-15`): doors, windows and bare openings put in
//! their walls on a named layer, one undo step named for the first's form (“Kapı”, “Pencere”, “Boşluk”). The door,
//! window and opening tools preview the plan's openings and write through `execute`; Python and an agent give the wall
//! and the place and get the same openings. The shared cases are `fixtures/commands/v1/cad.openings.create.json`.
//!
//! The checks, in order (the first that fails answers): 1. at least one opening (`no_objects`), at most
//! `MAX_OPENINGS` (`invalid_opening`); 2. each opening in the input's order: its numbers (`not_finite`), its wall
//! (`invalid_uid`, `entity_not_found`, `not_a_wall`), its type (`style_not_found`), its fields (`invalid_opening`),
//! its place on one edge within its wall's ends (`opening_off_wall`); 3. clear of its wall's other openings and the
//! call's (`openings_overlap`); 4. the line weight; 5. the expected revision; 6. the layer; 7. the attributes.

use kentos_commands::{
    CommandResult, CommandWarning, Entity, EntityBase, OpeningFields, OpeningForm, OpeningsCreate,
    OpeningsCreated, OpeningsPlan,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::openings::{self, fail};

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &OpeningsCreate) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok((warnings, _)) => CommandResult::Completed {
            output: (),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &OpeningsCreate) -> CommandResult<OpeningsPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document: what the door, window and opening tools draw as their preview.
pub fn plan_in(doc: &Document, input: &OpeningsCreate) -> CommandResult<OpeningsPlan> {
    match check(doc, input) {
        Ok((warnings, made)) => {
            let openings: Vec<Entity> = made
                .into_iter()
                .map(|(f, a)| openings::entity(base(input, a), f))
                .collect();
            let followers = crate::dimension_sets::derived(doc, &[], &openings, &[]);
            CommandResult::Completed {
                output: OpeningsPlan {
                    openings,
                    followers,
                    revision: doc.revision().to_string(),
                },
                warnings,
            }
        }
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the openings as one undo step.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: OpeningsCreate,
) -> CommandResult<OpeningsCreated> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_OPENINGS_CREATE,
        kentos_commands::CAD_OPENINGS_CREATE_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: OpeningsCreate,
) -> CommandResult<OpeningsCreated> {
    let (warnings, made) = match check(cx.doc, &input) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    let step = openings::step_of(made.first().map_or(OpeningForm::Door, |(f, _)| f.form));
    let doc = &mut *cx.doc;
    let entities: Vec<Entity> = made
        .into_iter()
        .map(|(f, a)| openings::entity(base(&input, a), f))
        .collect();
    let slots = match doc.add_many(entities, step) {
        Ok(slots) => slots,
        Err(full) => {
            return Stop::failed(error(codes::SLOTS_EXHAUSTED, full.to_string(), None)).into();
        }
    };
    CommandResult::Completed {
        output: OpeningsCreated {
            uids: slots
                .iter()
                .filter_map(|s| doc.uid(*s))
                .map(|u| u.to_string())
                .collect(),
            ids: slots.iter().map(|s| s.0).collect(),
            revision: doc.revision().to_string(),
        },
        warnings,
    }
}

type Attrs = Option<std::collections::BTreeMap<String, String>>;
type Checked = (Vec<CommandWarning>, Vec<(OpeningFields, Attrs)>);

fn check(doc: &Document, input: &OpeningsCreate) -> Result<Checked, Stop> {
    if input.openings.is_empty() {
        return Err(fail(
            codes::NO_OBJECTS,
            "Yazılacak açıklık verilmedi. En az bir açıklığın duvarını ve yerini verin.".into(),
            "openings",
        ));
    }
    if input.openings.len() > kentos_commands::MAX_OPENINGS {
        return Err(fail(
            codes::INVALID_OPENING,
            format!(
                "Bir çağrıda en çok {} açıklık yazılır; {} verildi.",
                kentos_commands::MAX_OPENINGS,
                input.openings.len()
            ),
            "openings",
        ));
    }
    let mut made: Vec<OpeningFields> = Vec::with_capacity(input.openings.len());
    for (i, n) in input.openings.iter().enumerate() {
        let at = format!("openings[{i}]");
        for (v, what, name) in [
            (n.at, "at", "Konum"),
            (n.distance, "distance", "Uzaklık"),
            (n.width, "width", "Genişlik"),
            (n.swing, "swing", "Açılma açısı"),
        ] {
            if let Some(x) = v {
                checks::finite(x, name, "Sonlu bir sayı verin.", &format!("{at}.{what}"))?;
            }
        }
        if let Some(p) = n.point {
            checks::point(p, "Açıklığın noktası", &format!("{at}.point"))?;
        }
        if n.distance_reference.is_some() && n.distance.is_none() {
            return Err(fail(
                codes::INVALID_OPENING,
                "Mesafe referansı için distance verilmelidir.".into(),
                &format!("{at}.distanceReference"),
            ));
        }
        if let Some(span) = n.span {
            for (j, p) in span.into_iter().enumerate() {
                checks::point(p, "Açıklığın kenar noktası", &format!("{at}.span[{j}]"))?;
            }
            if n.at.is_some() || n.distance.is_some() || n.middle || n.width.is_some() {
                return Err(fail(
                    codes::INVALID_OPENING,
                    "İki kenar noktası (span), at/distance/middle/width ile birlikte verilemez."
                        .into(),
                    &format!("{at}.span"),
                ));
            }
        }
        let (_, _, line) = openings::wall_at(doc, &n.host, &format!("{at}.host"))?;
        let kind = match &n.kind {
            Some(key) => openings::find_type(key, &format!("{at}.type"))?,
            None => openings::first_type(n.form.unwrap_or(OpeningForm::Door)),
        };
        let mut f = openings::fields_of(n, &kind);
        let (centre, side) = if let Some(span) = n.span {
            let (centre, width, side) = crate::opening_placement::span(&line, span, n.point, &at)?;
            f.width = width;
            (centre, side)
        } else {
            openings::centre_of(n, f.width, f.hand, &line, &at)?
        };
        f.at = centre;
        f.side = n.side.or(side).unwrap_or_default();
        f.place = vec![kentos_commands::Vec2 { x: 0.0, y: 0.0 }; 4];
        openings::valid(&f, &at)?;
        made.push(openings::placed(&f, &line, &at)?);
    }
    let paths: Vec<String> = (0..made.len()).map(|i| format!("openings[{i}]")).collect();
    let new: Vec<(&OpeningFields, String)> = made.iter().zip(paths).collect();
    openings::clear_of_others(doc, &[], &new)?;
    checks::line_weight(input.line_weight, "lineWeight")?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let warnings = checks::layer(doc, &input.layer_id)?;
    let mut out = Vec::with_capacity(made.len());
    for (i, (f, n)) in made.into_iter().zip(&input.openings).enumerate() {
        let attrs = checks::object_attrs(
            doc,
            &input.layer_id,
            n.attrs.as_ref(),
            &format!("openings[{i}].attrs"),
        )?;
        out.push((f, attrs));
    }
    Ok((warnings, out))
}

/// An opening's base as the document stores it; its `id` is the document's to give (0 in a plan).
fn base(input: &OpeningsCreate, attrs: Attrs) -> EntityBase {
    EntityBase {
        look: Default::default(),
        id: 0,
        layer_id: input.layer_id.clone(),
        color: input.color.clone(),
        attrs: attrs.unwrap_or_default(),
        label: None,
        symbol: None,
        line_weight: input.line_weight,
        label_pins: Vec::new(),
        attachments: Vec::new(),
    }
}
