//! `cad.settings.set` v1 (docs/adr/0248 §5): a few of the project's own
//! settings changed by key, as Proje ayarları and the ribbon's Ölçek change
//! them: the drawing's name and type, its coordinate systems (assigned,
//! never a transformation of the coordinates, CLAUDE.md §5), a local
//! project's unit, the shown decimals and units, the plot scale (the
//! annotations at the old general height follow in one undo step,
//! docs/adr/0205 §3) and the drawing's typeface. The settings are an edit of
//! the project, no undo step (`Document::set_settings`, which keeps them as
//! a project may: `ProjectSettings::sanitized`). The shared cases are in
//! `fixtures/commands/v1/cad.settings.set.json`.

use std::sync::OnceLock;

use kentos_contracts::{
    CommandResult, DrawingUnit, MAX_PLOT_SCALE, MAX_SHOWN_DECIMALS, ProjectSettings, ScaleChange,
    SettingsChanged, SettingsSet, Workspace,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::annotation_follow::follow_annotation_change;
use crate::checks::{self, Stop, error};
use crate::codes;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::Failed(error(code, message, Some(path.to_owned())))
}

/// Whether the registry has the system (0: a local project, no system). The
/// registry is the one data file every reader takes it from
/// (`fixtures/crs/v1/registry.json`; the project model's `kentos_project::crs`
/// reads its systems from it too); here only its codes are read, so the
/// commands' crate stays clear of the project model (`scripts/arch/deps.mjs`).
fn registered(srid: u32) -> bool {
    static CODES: OnceLock<Vec<u32>> = OnceLock::new();
    let codes = CODES.get_or_init(|| {
        serde_json::from_str::<serde_json::Value>(include_str!(
            "../../../../fixtures/crs/v1/registry.json"
        ))
        .ok()
        .and_then(|r| {
            r["systems"].as_array().map(|systems| {
                systems
                    .iter()
                    .filter_map(|s| s["srid"].as_u64())
                    .filter_map(|s| u32::try_from(s).ok())
                    .collect()
            })
        })
        .unwrap_or_default()
    });
    srid == 0 || codes.contains(&srid)
}

fn check_input(input: &SettingsSet) -> Result<(), Stop> {
    let given = input.name.is_some()
        || input.project_type.is_some()
        || input.srid.is_some()
        || input.second_srid.is_some()
        || input.drawing_unit.is_some()
        || input.length_decimals.is_some()
        || input.area_decimals.is_some()
        || input.area_unit.is_some()
        || input.angle_unit.is_some()
        || input.plot_scale.is_some()
        || input.drawing_font.is_some();
    if !given {
        return Err(Stop::Failed(error(
            codes::NOTHING_TO_SET,
            "Değişecek bir ayar yok: en az bir anahtar verin.".into(),
            None,
        )));
    }
    if input.name.as_deref().is_some_and(checks::is_blank) {
        return Err(fail(
            codes::EMPTY_NAME,
            "Çizimin adı boş olamaz; bir ad yazın.".into(),
            "name",
        ));
    }
    if input
        .project_type
        .is_some_and(|t| !matches!(t, Workspace::Cad | Workspace::Gis))
    {
        return Err(fail(
            codes::INVALID_PROJECT_TYPE,
            "Proje türü cad ya da gis olabilir; öbürleri henüz seçilemez.".into(),
            "projectType",
        ));
    }
    for (value, path) in [
        (input.length_decimals, "lengthDecimals"),
        (input.area_decimals, "areaDecimals"),
    ] {
        if value.is_some_and(|d| d > MAX_SHOWN_DECIMALS) {
            return Err(fail(
                codes::INVALID_DECIMALS,
                format!(
                    "Gösterilen basamak 0 ile {MAX_SHOWN_DECIMALS} arasında bir tam sayı olmalı."
                ),
                path,
            ));
        }
    }
    if let Some(scale) = input.plot_scale
        && !(scale.is_finite() && scale.fract() == 0.0 && (1.0..=MAX_PLOT_SCALE).contains(&scale))
    {
        return Err(fail(
            codes::INVALID_SCALE,
            "Çizim ölçeği 1 ile 1 000 000 arasında bir tam sayı payda olmalı (1:1000 için 1000)."
                .into(),
            "plotScale",
        ));
    }
    for (value, path) in [(input.srid, "srid"), (input.second_srid, "secondSrid")] {
        if let Some(srid) = value.filter(|s| !registered(*s)) {
            return Err(fail(
                codes::UNKNOWN_SRID,
                format!(
                    "EPSG:{srid} koordinat sistemleri kaydında yok. Kayıttaki bir sistemin kodunu verin ya da Proje ayarları'nda sistemi tanımlayın."
                ),
                path,
            ));
        }
    }
    Ok(())
}

/// The settings the input leaves, or why not; the drawing's name apart.
fn next_settings(doc: &Document, input: &SettingsSet) -> Result<ProjectSettings, Stop> {
    let mut s = doc.settings().clone();
    if let Some(t) = input.project_type {
        s.workspace = Some(t);
    }
    if let Some(srid) = input.srid {
        // An assignment takes the place of a defined system (docs/adr/0168 §1).
        s.srid = srid;
        s.custom_crs = None;
    }
    if let Some(second) = input.second_srid {
        s.second_srid = (second != 0).then_some(second);
        s.second_custom_crs = None;
        if second != 0 && !s.has_system() {
            return Err(fail(
                codes::SECOND_NEEDS_SYSTEM,
                "Koordinat sistemi olmayan yerel projenin ikinci sistemi olmaz; önce projeye bir sistem atayın."
                    .into(),
                "secondSrid",
            ));
        }
        if second != 0 && second == s.srid {
            return Err(fail(
                codes::SECOND_IS_OWN,
                format!(
                    "EPSG:{second} projenin kendi sistemi; ikinci sistem başka bir sistem olmalı."
                ),
                "secondSrid",
            ));
        }
    }
    if let Some(unit) = input.drawing_unit {
        if unit != DrawingUnit::M && s.has_system() {
            return Err(fail(
                codes::UNIT_NEEDS_LOCAL,
                "Koordinat sistemi olan projenin birimi metredir; mm ya da cm yalnız sistemsiz yerel projede seçilir."
                    .into(),
                "drawingUnit",
            ));
        }
        s.drawing_unit = (unit != DrawingUnit::M).then_some(unit);
    }
    // A drawing unit is a local project's (docs/adr/0165 §2): one given a system is in metres.
    if s.has_system() {
        s.drawing_unit = None;
    }
    if let Some(d) = input.length_decimals {
        s.length_decimals = d;
    }
    if let Some(d) = input.area_decimals {
        s.area_decimals = d;
    }
    if let Some(u) = input.area_unit {
        s.area_unit = u;
    }
    if let Some(u) = input.angle_unit {
        s.angle_unit = u;
    }
    if let Some(p) = input.plot_scale {
        s.plot_scale = p;
    }
    if let Some(f) = input.drawing_font {
        s.drawing_font = Some(f);
    }
    Ok(s)
}

fn check(doc: &Document, input: &SettingsSet) -> Result<ProjectSettings, Stop> {
    check_input(input)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    next_settings(doc, input)
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &SettingsSet) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would do, on a reading copy; the drawing is left as it is.
pub fn plan(cx: &ExecutionContext<'_>, input: &SettingsSet) -> CommandResult<SettingsChanged> {
    let next = match check(cx.doc, input) {
        Ok(next) => next,
        Err(stop) => return stop.into(),
    };
    let mut copy = cx.doc.reading_copy();
    let mut output = write(&mut copy, input, next);
    output.revision = cx.doc.revision().to_string();
    CommandResult::Completed {
        output,
        warnings: Vec::new(),
    }
}

/// Writes the settings, then the annotations that follow the plot scale.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: SettingsSet,
) -> CommandResult<SettingsChanged> {
    let next = match check(cx.doc, &input) {
        Ok(next) => next,
        Err(stop) => return stop.into(),
    };
    CommandResult::Completed {
        output: write(cx.doc, &input, next),
        warnings: Vec::new(),
    }
}

fn write(doc: &mut Document, input: &SettingsSet, next: ProjectSettings) -> SettingsChanged {
    let before = doc.settings().clone();
    let name_before = doc.name().to_owned();
    if let Some(name) = input.name.as_deref() {
        doc.set_name(name.trim());
    }
    doc.set_settings(next);
    let after = doc.settings().clone();
    let mut changed = Vec::new();
    let mut key = |differs: bool, name: &str| {
        if differs {
            changed.push(name.to_owned());
        }
    };
    key(doc.name() != name_before, "name");
    key(before.workspace != after.workspace, "projectType");
    key(
        (before.srid, &before.custom_crs) != (after.srid, &after.custom_crs),
        "srid",
    );
    key(
        (before.second_srid, &before.second_custom_crs)
            != (after.second_srid, &after.second_custom_crs),
        "secondSrid",
    );
    key(before.drawing_unit != after.drawing_unit, "drawingUnit");
    key(
        before.length_decimals != after.length_decimals,
        "lengthDecimals",
    );
    key(before.area_decimals != after.area_decimals, "areaDecimals");
    key(before.area_unit != after.area_unit, "areaUnit");
    key(before.angle_unit != after.angle_unit, "angleUnit");
    key(before.plot_scale != after.plot_scale, "plotScale");
    key(before.drawing_font != after.drawing_font, "drawingFont");
    let heights = after.annotation.clone().unwrap_or_default();
    let followed = follow_annotation_change(
        doc,
        &ScaleChange {
            from_scale: before.plot_scale,
            to_scale: after.plot_scale,
            from: heights.clone(),
            to: heights,
        },
    );
    SettingsChanged {
        changed,
        followed: count(followed.written),
        locked: count(followed.locked),
        revision: doc.revision().to_string(),
    }
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}
