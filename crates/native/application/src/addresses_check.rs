//! `cad.addresses.check` (docs/adr/0579): a project's address data checked, read only. The locator is built as the
//! Adres tab builds it ([`crate::addresses`]) and checked by the shared core (`ops::geocode::check`): what it holds,
//! what it left out, the doors one street has twice, the address codes on more than one door and the streets written
//! in more than one way. The doors are given by their objects' persistent ids.

use std::collections::HashMap;

use kentos_commands::{
    ADDRESS_CHECK_LIMIT, AddressCodeRepeat, AddressRepeat, AddressSpelling, AddressWritten,
    AddressesCheck, AddressesChecked, CommandResult,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::ops::geocode::check::check;

use crate::ExecutionContext;
use crate::addresses;
use crate::checks::{Stop, error};
use crate::codes;

/// The objects' persistent ids by their places in the drawing's order (a door's `order`).
fn uids_by_place(doc: &Document) -> HashMap<u32, String> {
    doc.entities_with_uids()
        .filter_map(|(uid, e)| {
            let place = doc.place(Slot(e.base().id))?;
            Some((place.min(u64::from(u32::MAX)) as u32, uid.to_string()))
        })
        .collect()
}

fn uids(by_place: &HashMap<u32, String>, orders: &[u32]) -> Vec<String> {
    orders
        .iter()
        .filter_map(|o| by_place.get(o).cloned())
        .collect()
}

fn count(n: usize) -> u32 {
    n.min(u32::MAX as usize) as u32
}

/// What the command answers; why not.
pub(crate) fn check_in(doc: &Document, input: &AddressesCheck) -> Result<AddressesChecked, Stop> {
    let limit = input.limit.unwrap_or(ADDRESS_CHECK_LIMIT);
    if limit == 0 || limit > ADDRESS_CHECK_LIMIT {
        return Err(Stop::failed(error(
            codes::INVALID_LIMIT,
            format!("Liste sınırı 1 ile {ADDRESS_CHECK_LIMIT} arasında bir tam sayı olmalı."),
            Some("limit".into()),
        )));
    }
    let limit = limit as usize;
    let locators = &doc.settings().address_locators;
    let Some(first) = locators.first() else {
        return Err(Stop::failed(error(
            codes::NO_ADDRESS_DATA,
            "Projede adres verisi yok; önce Adres verisi penceresinde ya da cad.addresses.define ile tanımlayın."
                .to_owned(),
            None,
        )));
    };
    let locator = match &input.locator {
        None => first,
        Some(id) => addresses::locator(doc, id).ok_or_else(|| {
            Stop::failed(error(
                codes::UNKNOWN_LOCATOR,
                format!("Projede “{id}” kimlikli adres verisi yok."),
                Some("locator".into()),
            ))
        })?,
    };
    let summary = check(&addresses::build(doc, locator));
    let by_place = uids_by_place(doc);
    Ok(AddressesChecked {
        locator: locator.id.clone(),
        doors: count(summary.counts.doors),
        streets: count(summary.counts.streets),
        lines: count(summary.counts.lines),
        neighbourhoods: count(summary.counts.neighbourhoods),
        areas: count(summary.counts.areas),
        not_doors: count(summary.not_doors),
        empty_streets: count(summary.empty_streets),
        unread_doors: count(summary.unread_doors),
        not_streets: count(summary.not_streets),
        unnamed_lines: count(summary.unnamed_lines),
        not_areas: count(summary.not_areas),
        unnamed_areas: count(summary.unnamed_areas),
        repeated: count(summary.repeated),
        repeats: summary
            .repeats
            .iter()
            .take(limit)
            .map(|r| AddressRepeat {
                street: r.street.clone(),
                neighbourhood: r.neighbourhood.clone(),
                door: r.door.clone(),
                uids: uids(&by_place, &r.orders),
            })
            .collect(),
        coded: count(summary.coded),
        codes: summary
            .code_repeats
            .iter()
            .take(limit)
            .map(|c| AddressCodeRepeat {
                code: c.code.clone(),
                uids: uids(&by_place, &c.orders),
            })
            .collect(),
        spelled: count(summary.spelled),
        spellings: summary
            .spellings
            .iter()
            .take(limit)
            .map(|s| AddressSpelling {
                neighbourhood: s.neighbourhood.clone(),
                district: s.district.clone(),
                written: s
                    .written
                    .iter()
                    .map(|(text, n)| AddressWritten {
                        text: text.clone(),
                        doors: count(*n),
                    })
                    .collect(),
                uids: uids(&by_place, &s.orders),
            })
            .collect(),
        revision: doc.revision().to_string(),
    })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &AddressesCheck) -> CommandResult<()> {
    match check_in(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &AddressesCheck) -> CommandResult<AddressesChecked> {
    match check_in(cx.doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// Read-only: the same answer as the plan.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: AddressesCheck,
) -> CommandResult<AddressesChecked> {
    plan(cx, &input)
}
