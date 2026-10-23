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

use kentos_commands::{
    CommandResult, DrawingUnit, Entity, HeightKind, LengthStyle, MAX_PLOT_SCALE,
    MAX_SHOWN_DECIMALS, ProjectSettings, ScaleChange, SettingsChanged, SettingsSet, Workspace,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::annotation_follow::follow_annotation_change;
use crate::checks::{self, Stop, error};
use crate::codes;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
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
        || input.map_srid.is_some()
        || input.auto_select.is_some()
        || input.drawing_unit.is_some()
        || input.length_style.is_some()
        || input.length_decimals.is_some()
        || input.area_decimals.is_some()
        || input.area_unit.is_some()
        || input.angle_unit.is_some()
        || input.plot_scale.is_some()
        || input.drawing_font.is_some()
        || input.arch.is_some()
        || input.heights.is_some()
        || input.coordinate_epoch.is_some();
    if !given {
        return Err(Stop::failed(error(
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
    for (value, path) in [
        (input.srid, "srid"),
        (input.second_srid, "secondSrid"),
        (input.map_srid, "mapSrid"),
    ] {
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
    if let Some((path, why)) = input.arch.as_ref().and_then(|a| a.problem()) {
        return Err(fail(codes::INVALID_ARCH, why, &format!("arch.{path}")));
    }
    if let Some((path, why)) = input.heights.as_ref().and_then(|h| h.problem()) {
        return Err(fail(
            codes::INVALID_HEIGHTS,
            format!("Yükseklik tanımı: {why}."),
            &format!("heights.{path}"),
        ));
    }
    if let Some(why) = input
        .coordinate_epoch
        .filter(|e| *e != 0.0)
        .and_then(kentos_commands::epoch_problem)
    {
        return Err(fail(
            codes::INVALID_EPOCH,
            format!("Epok: {why}; 0 epoku kaldırır."),
            "coordinateEpoch",
        ));
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
        // The map's system is never the project's own nor a local project's (docs/adr/0583 §3).
        if s.map_srid == Some(srid) || srid == 0 {
            s.map_srid = None;
        }
    }
    // Otomatik seçim (docs/adr/0622 §5): the project's choice, written even when it is the type's own.
    if let Some(on) = input.auto_select {
        s.auto_select = Some(on);
    }
    if let Some(map) = input.map_srid {
        // The map's system is a view of the project's (docs/adr/0583 §3): none of the project's own, none for a
        // project without a registry system.
        s.map_srid = (map != 0 && map != s.srid).then_some(map);
        if s.map_srid.is_some() && (s.srid == 0 || !s.has_system()) {
            return Err(fail(
                codes::SECOND_NEEDS_SYSTEM,
                "Koordinat sistemi kayıttan olmayan projenin harita sistemi olmaz; önce projeye kayıttaki bir sistemi atayın."
                    .into(),
                "mapSrid",
            ));
        }
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
                "Koordinat sistemi olan projenin birimi metredir; mm, cm, inç ya da ayak yalnız sistemsiz yerel projede seçilir."
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
    // A feet-and-inches style is an inch or foot drawing's (docs/adr/0262 §2): one given
    // for a metric unit is refused, one the unit leaves goes back to decimal.
    if let Some(style) = input.length_style {
        if style != LengthStyle::Decimal && !s.unit().imperial() {
            return Err(fail(
                codes::STYLE_NEEDS_IMPERIAL,
                "Ayak ve inç biçimleri (mimari, mühendislik, kesirli) yalnız inç ya da ayak biriminde seçilir; önce birimi inç ya da ayak yapın."
                    .into(),
                "lengthStyle",
            ));
        }
        s.length_style = (style != LengthStyle::Decimal).then_some(style);
    }
    if !s.unit().imperial() {
        s.length_style = None;
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
    if let Some(arch) = &input.arch {
        // A wall keeps the level it names (docs/adr/0339 §3).
        if let Some(id) = doc.entities().find_map(|e| match e {
            Entity::Wall(w) => w
                .wall
                .arch
                .level
                .as_deref()
                .filter(|id| !arch.levels.iter().any(|l| l.id == *id)),
            _ => None,
        }) {
            return Err(fail(
                codes::LEVEL_IN_USE,
                format!(
                    "“{id}” katını duvarlar anıyor; kaldırılmadı. Önce duvarların katını değiştirin ya da katı bırakın."
                ),
                "arch.levels",
            ));
        }
        s.arch = (!arch.is_empty()).then(|| arch.clone());
    }
    // What the elevations are and the epoch (docs/adr/0519 §1): assigned, no coordinate moves.
    if let Some(h) = &input.heights {
        s.heights = (h.kind != HeightKind::Unknown).then(|| h.clone());
    }
    if let Some(e) = input.coordinate_epoch {
        s.coordinate_epoch = (e != 0.0).then_some(e);
    }
    if !s.has_system() {
        let ellipsoidal = s
            .heights
            .as_ref()
            .is_some_and(|h| h.kind == HeightKind::Ellipsoidal);
        let (given, path) = if input.coordinate_epoch.is_some_and(|e| e != 0.0) {
            (true, "coordinateEpoch")
        } else {
            (ellipsoidal && input.heights.is_some(), "heights.kind")
        };
        if given {
            return Err(fail(
                codes::HEIGHTS_NEED_SYSTEM,
                "Koordinat sistemi olmayan yerel projede elipsoit yüksekliği ve koordinat epoku olmaz: ikisi de bir datumun elipsoidine ve zamanına bağlıdır. Önce projeye bir sistem atayın ya da ortometrik yüksekliği seçin."
                    .into(),
                path,
            ));
        }
        // A system taken away takes what rests on it.
        if ellipsoidal {
            s.heights = None;
        }
        s.coordinate_epoch = None;
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
    crate::call::recorded(
        cx,
        kentos_commands::CAD_SETTINGS_SET,
        kentos_commands::CAD_SETTINGS_SET_VERSION,
        input,
        write_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step of the
/// document, all or nothing, and journals it (docs/adr/0253 §1, §3).
fn write_execute(
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
    key(before.map_srid != after.map_srid, "mapSrid");
    key(before.auto_select != after.auto_select, "autoSelect");
    key(before.drawing_unit != after.drawing_unit, "drawingUnit");
    key(before.length_style != after.length_style, "lengthStyle");
    key(
        before.length_decimals != after.length_decimals,
        "lengthDecimals",
    );
    key(before.area_decimals != after.area_decimals, "areaDecimals");
    key(before.area_unit != after.area_unit, "areaUnit");
    key(before.angle_unit != after.angle_unit, "angleUnit");
    key(before.plot_scale != after.plot_scale, "plotScale");
    key(before.drawing_font != after.drawing_font, "drawingFont");
    key(before.arch != after.arch, "arch");
    key(before.heights != after.heights, "heights");
    key(
        before.coordinate_epoch != after.coordinate_epoch,
        "coordinateEpoch",
    );
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
