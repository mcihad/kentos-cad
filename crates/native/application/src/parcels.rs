//! `cad.parcels.create` v1 (Parsel oluştur, docs/adr/0067, 0622 §9): a closed area on the parcel layer, whatever layer
//! is active, numbered by the command, its attributes the parcel's, the layer opened in its step when the drawing
//! lacks it; written as `cad.polygon.create` writes an area, Çakışma and Topoloji included (`polygon::ruled_areas`).
//! The tool gives the corners and the session's rules; Python and an agent give the same. The shared cases are
//! `fixtures/commands/v1/cad.parcels.create.json` (`scripts/fixtures/area_rules_cases.py`).

use std::collections::BTreeMap;

use kentos_commands::{
    CommandResult, CommandWarning, PARCEL_LAYER, ParcelCreated, ParcelPlan, ParcelsCreate,
    PolygonCreate,
};
use kentos_domain::{Document, NewLayer, labels};
use kentos_geometry_core::store::Store;

use crate::checks::{Stop, error};
use crate::{ExecutionContext, codes, polygon};

/// The parcel's attributes, in the order Öznitelikler shows them.
pub const ATTRS: [&str; 6] = [
    "Ada",
    "Parsel",
    "Mahalle",
    "Nitelik",
    "Tapu alanı (m²)",
    "Pafta",
];

/// JavaScript's `parseInt(text, 10)` for a whole number that is not negative: white space at the start skipped, the
/// digits up to the first other character; none when there is no digit.
fn parse_int(text: &str) -> Option<u64> {
    let rest = text.trim_start_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
    let rest = rest.strip_prefix('+').unwrap_or(rest);
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    Some(digits.parse().unwrap_or(u64::MAX))
}

/// One above the greatest whole number the parcels' Parsel values start with.
pub fn next_number(doc: &Document) -> String {
    doc.by_layer(PARCEL_LAYER)
        .filter_map(|e| e.base().attrs.get("Parsel"))
        .map(|text| parse_int(text).unwrap_or(0))
        .max()
        .unwrap_or(0)
        .saturating_add(1)
        .to_string()
}

/// The number: the given one trimmed, or the next; a given one another parcel has is warned of.
fn number_of(doc: &Document, input: &ParcelsCreate) -> Result<(String, Vec<CommandWarning>), Stop> {
    let Some(given) = &input.number else {
        return Ok((next_number(doc), Vec::new()));
    };
    let number = given.trim();
    if number.is_empty() {
        return Err(Stop::failed(error(
            codes::INVALID_PARCEL_NUMBER,
            "Parsel numarası boş olamaz. Bir numara verin ya da sıradakini almak için numarayı vermeyin.".into(),
            Some("number".into()),
        )));
    }
    let taken = doc.by_layer(PARCEL_LAYER).any(|e| {
        e.base()
            .attrs
            .get("Parsel")
            .is_some_and(|v| v.trim() == number)
    });
    let warnings = if taken {
        vec![CommandWarning {
            code: codes::PARCEL_NUMBER_TAKEN.into(),
            message: format!(
                "“{number}” numaralı başka bir parsel var; numara yine verildi. Parsel numarasını Öznitelikler'den düzeltebilirsiniz."
            ),
            path: Some("number".into()),
        }]
    } else {
        Vec::new()
    };
    Ok((number.to_owned(), warnings))
}

/// The parcel layer as a new project has it (`Parsel sınırı`).
fn parcel_layer(doc: &Document) -> NewLayer {
    let settings = doc.settings();
    let cad = settings.project_type() == Some(kentos_commands::Workspace::Cad);
    let (name, style) =
        match kentos_project::new_project::standard_layer(PARCEL_LAYER, settings.plot_scale, cad) {
            Some(node) => (node.name, node.style),
            None => (PARCEL_LAYER.to_owned(), NewLayer::layer(PARCEL_LAYER).style),
        };
    NewLayer {
        id: Some(PARCEL_LAYER.to_owned()),
        style,
        ..NewLayer::layer(name)
    }
}

/// The area written: the corners, the look, the parcel's attributes and label.
fn base(input: &ParcelsCreate, number: &str) -> PolygonCreate {
    let mut attrs: BTreeMap<String, String> = ATTRS
        .iter()
        .map(|k| ((*k).to_owned(), String::new()))
        .collect();
    attrs.insert("Nitelik".into(), "Arsa".into());
    attrs.extend(input.attrs.clone().unwrap_or_default());
    attrs.insert("Parsel".into(), number.to_owned());
    PolygonCreate {
        layer_id: PARCEL_LAYER.to_owned(),
        pts: input.pts.clone(),
        bulges: input.bulges.clone(),
        holes: None,
        color: input.color.clone(),
        line_weight: input.line_weight,
        line_type: input.line_type,
        wipeout: None,
        attrs: Some(attrs),
        label: Some(number.to_owned()),
        symbol: None,
        rules: input.rules.clone(),
        expected_revision: input.expected_revision.clone(),
    }
}

/// The parcel's attributes the user fills later kept empty, so Öznitelikler offers them (the web's `createParcel`);
/// the area's write drops empty values.
fn keep_blanks(ruled: &mut polygon::Ruled) {
    let attrs = &mut ruled.1.base_mut().attrs;
    for key in ATTRS {
        attrs.entry(key.to_owned()).or_default();
    }
}

/// A locked parcel layer, said in the tool's words: another active layer would not help.
fn check_layer(doc: &Document) -> Result<(), Stop> {
    let layers = doc.layers();
    if let Some(node) = layers.get(PARCEL_LAYER)
        && layers.is_locked(PARCEL_LAYER)
    {
        return Err(Stop::failed(error(
            codes::LAYER_LOCKED,
            format!(
                "“{}” katmanı kilitli; Parsel bu katmana yazar. Kilidi Katmanlar panelinden açın.",
                node.name
            ),
            None,
        )));
    }
    Ok(())
}

/// What would be written, on the drawing as it is (the layer opened on a copy when it lacks it): the polygon's
/// warnings and object, the number, whether the layer is opened.
fn ruled(
    doc: &Document,
    store: Option<&Store>,
    input: &ParcelsCreate,
) -> Result<(polygon::Ruled, String, bool), Stop> {
    let (number, mut warnings) = number_of(doc, input)?;
    check_layer(doc)?;
    // The revision is the drawing's, before the layer is opened.
    crate::checks::revision(doc, input.expected_revision.as_deref())?;
    let base = PolygonCreate {
        expected_revision: None,
        ..base(input, &number)
    };
    let area = crate::area_rules::area_of(&base.pts, base.bulges.as_deref(), None);
    let opened = doc.layers().get(PARCEL_LAYER).is_none();
    let mut ruled = if opened {
        let mut scratch = doc.clone();
        scratch
            .add_layer(parcel_layer(doc), None, false)
            .map_err(|refusal| {
                Stop::failed(error(codes::LAYER_NOT_FOUND, refusal.to_string(), None))
            })?;
        polygon::ruled_areas(&scratch, store, &base, vec![area])?
    } else {
        polygon::ruled_areas(doc, store, &base, vec![area])?
    };
    warnings.append(&mut ruled.0);
    ruled.0 = warnings;
    keep_blanks(&mut ruled);
    Ok((ruled, number, opened))
}

/// The parcel's checks and the polygon's, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ParcelsCreate) -> CommandResult<()> {
    match ruled(cx.doc, cx.store, input) {
        Ok(((warnings, _, _), _, _)) => CommandResult::Completed {
            output: (),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// The parcel as it would be written, its number and whether the layer is opened.
pub fn plan(cx: &ExecutionContext<'_>, input: &ParcelsCreate) -> CommandResult<ParcelPlan> {
    match ruled(cx.doc, cx.store, input) {
        Ok(((warnings, entity, _), number, opened)) => CommandResult::Completed {
            output: ParcelPlan {
                entity,
                number,
                opened,
                revision: cx.doc.revision().to_string(),
            },
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Writes the parcel (the layer first when the drawing lacks it), one undo step “Ekle”.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: ParcelsCreate,
) -> CommandResult<ParcelCreated> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_PARCELS_CREATE,
        kentos_commands::CAD_PARCELS_CREATE_VERSION,
        input,
        |cx, input| {
            let (number, mut warnings) = match number_of(cx.doc, &input) {
                Ok(n) => n,
                Err(stop) => return stop.into(),
            };
            if let Err(stop) = check_layer(cx.doc)
                .and_then(|()| crate::checks::revision(cx.doc, input.expected_revision.as_deref()))
            {
                return stop.into();
            }
            let opened = cx.doc.layers().get(PARCEL_LAYER).is_none();
            let group = cx.doc.begin_group(labels::ADD);
            if opened {
                let layer = parcel_layer(cx.doc);
                if let Err(refusal) = cx.doc.add_layer(layer, None, false) {
                    cx.doc.cancel_group(group);
                    return Stop::failed(error(codes::LAYER_NOT_FOUND, refusal.to_string(), None))
                        .into();
                }
            }
            let base = PolygonCreate {
                expected_revision: None,
                ..base(&input, &number)
            };
            let area = crate::area_rules::area_of(&base.pts, base.bulges.as_deref(), None);
            let ruled = match polygon::ruled_areas(cx.doc, cx.store, &base, vec![area]) {
                Ok(mut ruled) => {
                    keep_blanks(&mut ruled);
                    ruled
                }
                Err(stop) => {
                    cx.doc.cancel_group(group);
                    return stop.into();
                }
            };
            match polygon::write_ruled(cx, ruled) {
                CommandResult::Completed {
                    output,
                    warnings: more,
                } => {
                    cx.doc.end_group(group);
                    warnings.extend(more);
                    CommandResult::Completed {
                        output: ParcelCreated {
                            uid: output.uid,
                            id: output.id,
                            number,
                            opened,
                            revision: cx.doc.revision().to_string(),
                        },
                        warnings,
                    }
                }
                CommandResult::Failed { error } => {
                    cx.doc.cancel_group(group);
                    CommandResult::Failed { error }
                }
                CommandResult::Conflict { error } => {
                    cx.doc.cancel_group(group);
                    CommandResult::Conflict { error }
                }
                CommandResult::NeedsInput { error } => {
                    cx.doc.cancel_group(group);
                    CommandResult::NeedsInput { error }
                }
                CommandResult::Queued { job_id } => {
                    cx.doc.cancel_group(group);
                    CommandResult::Queued { job_id }
                }
                CommandResult::Cancelled => {
                    cx.doc.cancel_group(group);
                    CommandResult::Cancelled
                }
            }
        },
    )
}
