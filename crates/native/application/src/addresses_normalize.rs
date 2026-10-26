//! `cad.addresses.normalize` (docs/adr/0647 §2): the doors of an address data's groups that write a name in more than
//! one way given one spelling, as one undo step. The groups are the check's (`ops::geocode::check::name_groups`): a
//! street's doors, or doors grouped by the neighbourhood's and the district's keys, or by the district's. Each group
//! takes the first of `prefer` it writes, else its most written spelling; a door whose trimmed name is another gets it
//! in the door layer's field, through the field's rule (`checks::field_values`).

use std::collections::{BTreeMap, HashMap};

use kentos_commands::{
    AddressName, AddressNormalized, AddressesNormalize, AddressesNormalized, CommandResult,
    CommandWarning,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::ops::geocode::check::{Named, name_groups};

use crate::ExecutionContext;
use crate::addresses;
use crate::checks::{self, Stop, error};
use crate::codes;

/// The undo step's name.
pub const STEP: &str = "Adres yazılışlarını düzelt";

fn refuse(code: &str, message: String, path: Option<&str>) -> Stop {
    Stop::failed(error(code, message, path.map(str::to_owned)))
}

/// What the command writes: its answer and each door's new attributes.
struct Planned {
    out: AddressesNormalized,
    writes: Vec<(Slot, BTreeMap<String, String>)>,
    warnings: Vec<CommandWarning>,
}

fn plan_in(doc: &Document, input: &AddressesNormalize) -> Result<Planned, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    let locators = &doc.settings().address_locators;
    let Some(first) = locators.first() else {
        return Err(refuse(
            codes::NO_ADDRESS_DATA,
            "Projede adres verisi yok; önce Adres verisi penceresinde ya da cad.addresses.define ile tanımlayın."
                .to_owned(),
            None,
        ));
    };
    let locator = match &input.locator {
        None => first,
        Some(id) => addresses::locator(doc, id).ok_or_else(|| {
            refuse(
                codes::UNKNOWN_LOCATOR,
                format!("Projede “{id}” kimlikli adres verisi yok."),
                Some("locator"),
            )
        })?,
    };
    let (word, named) = match input.what {
        AddressName::Street => ("yol", Named::Street),
        AddressName::Neighbourhood => ("mahalle", Named::Neighbourhood),
        AddressName::District => ("ilçe", Named::District),
    };
    let doors = locator.doors.as_ref();
    let field = doors.and_then(|d| match input.what {
        AddressName::Street => Some(d.street.clone()),
        AddressName::Neighbourhood => d.neighbourhood.clone(),
        AddressName::District => d.district.clone(),
    });
    let (Some(doors), Some(field)) = (doors, field.filter(|f| !f.trim().is_empty())) else {
        return Err(refuse(
            codes::INVALID_INPUT,
            format!(
                "“{}” adres verisinin kapılarında {word} alanı yok; önce Adres verisi penceresinde onu seçin.",
                locator.name
            ),
            Some("what"),
        ));
    };
    for (i, p) in input.prefer.iter().enumerate() {
        if p.trim().is_empty() {
            return Err(refuse(
                codes::INVALID_INPUT,
                "Tercih edilen yazılış boş olamaz.".into(),
                Some(&format!("prefer[{i}]")),
            ));
        }
    }
    let warnings = checks::layer_at(doc, &doors.layer, "locator")?;
    let loc = addresses::build(doc, locator);
    // The doors' slots and ids by their places in the drawing's order (a door's `order`).
    let by_place: HashMap<u32, (Slot, String)> = doc
        .entities_with_uids()
        .filter_map(|(uid, e)| {
            let slot = Slot(e.base().id);
            let place = doc.place(slot)?;
            Some((
                place.min(u64::from(u32::MAX)) as u32,
                (slot, uid.to_string()),
            ))
        })
        .collect();
    let mut changed: BTreeMap<u32, (Slot, BTreeMap<String, String>)> = BTreeMap::new();
    let mut groups = Vec::new();
    for g in name_groups(&loc, named) {
        let chosen = input
            .prefer
            .iter()
            .map(|p| p.trim())
            .find(|p| g.written.iter().any(|(t, _)| t == p))
            .unwrap_or(g.written[0].0.as_str())
            .to_owned();
        // The spelling through the field's rule (a field that changed since may refuse it).
        let value = checks::field_values(
            doc,
            &doors.layer,
            &BTreeMap::from([(field.clone(), Some(chosen.clone()))]),
            "what",
        )?
        .remove(&field)
        .flatten()
        .unwrap_or_else(|| chosen.clone());
        let mut n = 0u32;
        for (order, text) in &g.doors {
            if *text == chosen {
                continue;
            }
            let Some((slot, _)) = by_place.get(order) else {
                continue;
            };
            let entry = changed.entry(*order).or_insert_with(|| {
                let attrs = doc
                    .get(*slot)
                    .map(|e| e.base().attrs.clone())
                    .unwrap_or_default();
                (*slot, attrs)
            });
            entry.1.insert(field.clone(), value.clone());
            n += 1;
        }
        groups.push(AddressNormalized {
            from: g
                .written
                .iter()
                .map(|(t, _)| t.clone())
                .filter(|t| *t != chosen)
                .collect(),
            to: chosen,
            doors: n,
        });
    }
    let uids = changed
        .keys()
        .filter_map(|o| by_place.get(o).map(|(_, u)| u.clone()))
        .collect();
    Ok(Planned {
        out: AddressesNormalized {
            locator: locator.id.clone(),
            what: input.what,
            groups,
            written: changed.len() as u32,
            uids,
            revision: doc.revision().to_string(),
        },
        writes: changed.into_values().collect(),
        warnings,
    })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &AddressesNormalize) -> CommandResult<()> {
    match plan_in(cx.doc, input) {
        Ok(p) => CommandResult::Completed {
            output: (),
            warnings: p.warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &AddressesNormalize,
) -> CommandResult<AddressesNormalized> {
    match plan_in(cx.doc, input) {
        Ok(p) => CommandResult::Completed {
            output: p.out,
            warnings: p.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Writes the doors' spellings as one undo step; nothing when nothing is to be written.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: AddressesNormalize,
) -> CommandResult<AddressesNormalized> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_ADDRESSES_NORMALIZE,
        kentos_commands::CAD_ADDRESSES_NORMALIZE_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: AddressesNormalize,
) -> CommandResult<AddressesNormalized> {
    let mut p = match plan_in(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    if !p.writes.is_empty() {
        cx.doc.name_step(STEP);
        for (slot, attrs) in p.writes {
            let Some(mut e) = cx.doc.get(slot).cloned() else {
                continue;
            };
            e.base_mut().attrs = attrs;
            cx.doc.update(slot, e);
        }
        p.out.revision = cx.doc.revision().to_string();
    }
    CommandResult::Completed {
        output: p.out,
        warnings: p.warnings,
    }
}
