//! Document schemas 2 to 6 read from a payload (docs/specs/kcad-v2.md §6),
//! straight into the contract (`DocumentSnapshotV2`): item by item, no
//! intermediate tree, so memory follows the drawing, not what a file claims.
//! Every map is checked for its keys (unknown ones are refused, required ones
//! must be there) and every value for its type; the objects get the slots
//! 1, 2, 3 … in file order. The objects themselves are read in `objects.rs`,
//! the block definitions in `blocks.rs`.

mod blocks;
mod crs;
mod objects;
mod services;
mod styles;

use kentos_contracts::{
    AngleUnit, AnnotationHeights, AnnotationKind, AreaUnit, Bounds, DOCUMENT_FORMAT,
    DOCUMENT_VERSION, DOCUMENT_VERSION_2, DocumentSnapshotV2, DrawingFont, DrawingUnit,
    FieldChoice, LabelInk, LabelPlacement, LabelStyle, LayerField, LayerFieldKind, LayerNode,
    LayerNodeType, LayerSnap, LayerState, LayerStateNode, LayerStyle, LineType, MAX_ANNOTATION_MM,
    MigrationSource, PointStyle, PointSymbol, ProjectId, ProjectSettings, ProjectStyles,
    SurveySettings, TopologyException, TopologyRule, TopologyRuleKind, TopologySettings, Vec2,
    Workspace, annotation_mm_holds, layer_fields_problem, layer_states_problem,
};

use crate::SCHEMAS;
use crate::cbor::{Any, Reader, Seg};
use crate::error::{Code, KcadError};
use crate::watch::Watch;
use objects::{Features, objects};

/// The most items a list reserves before they are read.
const PREALLOCATE: usize = 4096;

/// The drawing in a payload whose container was checked; `watch` hears the
/// project before its objects, then the objects as they are read, and may
/// stop the reading (docs/adr/0030).
pub(crate) fn payload(data: &[u8], watch: &mut dyn Watch) -> Result<DocumentSnapshotV2, KcadError> {
    let mut r = Reader::watched(data, watch);
    let doc = root(&mut r)?;
    if !r.at_end() {
        return Err(r.fail(Code::CborTrailing, "yükte belgeden sonra fazladan bayt var"));
    }
    Ok(doc)
}

/// A value that must be there, or the missing field's error.
fn required<T>(r: &mut Reader<'_>, value: Option<T>, key: &'static str) -> Result<T, KcadError> {
    value.ok_or_else(|| {
        r.push(Seg::Name(key));
        let e = r.fail(Code::MissingField, "zorunlu alan yok");
        r.pop();
        e
    })
}

fn unknown(r: &mut Reader<'_>) -> KcadError {
    r.fail(Code::UnknownField, "bilinmeyen alan")
}

/// Reads a map's pairs: `each(r, key)` reads the value of one key (the key is on the path).
fn map<'a>(
    r: &mut Reader<'a>,
    mut each: impl FnMut(&mut Reader<'a>, &'a str) -> Result<(), KcadError>,
) -> Result<(), KcadError> {
    let (n, _) = r.map()?;
    let mut previous = None;
    for _ in 0..n {
        let key = r.key(&mut previous)?;
        r.push(Seg::Key(key));
        each(r, key)?;
        r.pop();
    }
    r.leave();
    Ok(())
}

fn list<'a, T>(
    r: &mut Reader<'a>,
    mut item: impl FnMut(&mut Reader<'a>, usize) -> Result<T, KcadError>,
) -> Result<Vec<T>, KcadError> {
    let n = r.array()?;
    // `array` checked that n items can fit the bytes left, but an item in memory
    // may be larger than its smallest encoding (an object, a JSON value): only a
    // little is reserved ahead, the rest grows with the items actually read.
    let mut out = Vec::with_capacity(n.min(PREALLOCATE));
    for i in 0..n {
        r.push(Seg::Index(i));
        out.push(item(r, i)?);
        r.pop();
    }
    r.leave();
    Ok(out)
}

fn text(r: &mut Reader<'_>) -> Result<String, KcadError> {
    r.text().map(str::to_owned)
}

/// A value of an enumerated text: `values` pairs the written text with the value.
fn named<T: Copy>(r: &mut Reader<'_>, values: &[(&str, T)]) -> Result<T, KcadError> {
    let at = r.position();
    let t = r.text()?;
    values
        .iter()
        .find(|(name, _)| *name == t)
        .map(|&(_, v)| v)
        .ok_or_else(|| {
            let known: Vec<&str> = values.iter().map(|(n, _)| *n).collect();
            r.fail_at(
                Code::BadValue,
                at,
                &format!("“{t}” bilinmiyor ({})", known.join(", ")),
            )
        })
}

/// `[x, y]` (§6.3).
fn point(r: &mut Reader<'_>) -> Result<Vec2, KcadError> {
    let at = r.position();
    let n = r.array()?;
    if n != 2 {
        return Err(r.fail_at(Code::BadValue, at, &format!("nokta 2 sayı olmalı, {n} var")));
    }
    let x = r.float()?;
    let y = r.float()?;
    r.leave();
    Ok(Vec2 { x, y })
}

fn points(r: &mut Reader<'_>) -> Result<Vec<Vec2>, KcadError> {
    list(r, |r, _| point(r))
}

fn floats(r: &mut Reader<'_>) -> Result<Vec<f64>, KcadError> {
    list(r, |r, _| r.float())
}

/// `[minX, minY, maxX, maxY]` (§6.3).
fn bounds(r: &mut Reader<'_>) -> Result<Bounds, KcadError> {
    let at = r.position();
    let n = r.array()?;
    if n != 4 {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            &format!("sınırlar 4 sayı olmalı, {n} var"),
        ));
    }
    let b = Bounds {
        min_x: r.float()?,
        min_y: r.float()?,
        max_x: r.float()?,
        max_y: r.float()?,
    };
    r.leave();
    Ok(b)
}

/// 16 bytes, not all zero (§6.3).
fn id16(r: &mut Reader<'_>) -> Result<[u8; 16], KcadError> {
    let (raw, at) = r.bytes()?;
    let Ok(id) = <[u8; 16]>::try_from(raw) else {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            &format!("{} bayt; kimlik 16 bayt olmalı", raw.len()),
        ));
    };
    if id == [0; 16] {
        return Err(r.fail_at(Code::BadValue, at, "kimlik boş (sıfır) olamaz"));
    }
    Ok(id)
}

// ── The root and the document ───────────────────────────────────────────

fn root(r: &mut Reader<'_>) -> Result<DocumentSnapshotV2, KcadError> {
    let not_ours = |r: &Reader<'_>| {
        r.fail(
            Code::SchemaFormat,
            &format!("yük bir KentOS çizimi değil (format ≠ {DOCUMENT_FORMAT})"),
        )
    };
    let version_error = |r: &Reader<'_>, v: &str| {
        let known: Vec<String> = SCHEMAS.iter().map(u32::to_string).collect();
        r.fail(
            Code::SchemaVersion,
            &format!(
                "belge şeması sürümü {v} bu uygulamada okunamıyor (desteklenen: {}); KentOS'u güncelleyin",
                known.join(", ")
            ),
        )
    };
    let mut format_ok = false;
    let mut schema: Option<u32> = None;
    let mut document = None;
    map(r, |r, key| {
        match key {
            "format" => {
                if !matches!(r.any()?, Any::Text(DOCUMENT_FORMAT)) {
                    return Err(not_ours(r));
                }
                format_ok = true;
            }
            "version" => match r.any()? {
                Any::Uint(v) => match SCHEMAS.iter().find(|&&s| u64::from(s) == v) {
                    Some(&known) => schema = Some(known),
                    None => return Err(version_error(r, &v.to_string())),
                },
                _ => return Err(version_error(r, "(sayı değil)")),
            },
            "document" => {
                if !format_ok {
                    return Err(not_ours(r));
                }
                let Some(schema) = schema else {
                    return Err(version_error(r, "(yok)"));
                };
                document = Some(body(r, schema)?);
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    if !format_ok {
        return Err(not_ours(r));
    }
    if schema.is_none() {
        return Err(version_error(r, "(yok)"));
    }
    required(r, document, "document")
}

/// The document; `schema` is its payload's (2, 3 with objects' line weights,
/// 4 with vertex elevations too, 5 with areas' parts, 6 with blocks).
fn body(r: &mut Reader<'_>, schema: u32) -> Result<DocumentSnapshotV2, KcadError> {
    let has = Features::of(schema);
    let mut name = None;
    let mut blocks: Option<blocks::Definitions> = None;
    let mut layers = None;
    let mut origin = None;
    let mut styles = None;
    let mut entities = None;
    let mut home_view = None;
    let mut settings_ = None;
    let mut project_id = None;
    let mut active_layer = None;
    let mut migrated_from = None;
    map(r, |r, key| {
        match key {
            "name" => name = Some(text(r)?),
            "blocks" if has.blocks => blocks = Some(blocks::definitions(r, has)?),
            "layers" => layers = Some(list(r, |r, _| layer(r, has))?),
            "origin" => origin = Some(point(r)?),
            "styles" => styles = Some(project_styles(r)?),
            // Name, blocks and layers come first in the encoded order: the project and its
            // blocks are known before its objects.
            "entities" => {
                let none = Default::default();
                let (list, index) = blocks
                    .as_ref()
                    .map_or((&[][..], &none), |(l, i)| (&l[..], i));
                entities = Some(objects(
                    r,
                    name.as_deref(),
                    layers.as_ref().map_or(0, Vec::len),
                    has,
                    list,
                    index,
                )?)
            }
            "homeView" => home_view = Some(bounds(r)?),
            "settings" => settings_ = Some(settings(r, has)?),
            "projectId" => project_id = Some(ProjectId(id16(r)?)),
            "activeLayer" => active_layer = Some(text(r)?),
            "migratedFrom" => migrated_from = Some(source(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let name = required(r, name, "name")?;
    let layers = required(r, layers, "layers")?;
    let origin = required(r, origin, "origin")?;
    let styles = required(r, styles, "styles")?;
    let (entities, uids) = required(r, entities, "entities")?;
    let settings = required(r, settings_, "settings")?;
    let active_layer = required(r, active_layer, "activeLayer")?;
    service_links(r, &layers, &settings, &entities)?;
    Ok(DocumentSnapshotV2 {
        format: DOCUMENT_FORMAT.to_owned(),
        version: DOCUMENT_VERSION_2,
        name,
        settings,
        origin,
        home_view,
        layers,
        active_layer,
        entities,
        uids,
        styles,
        blocks: blocks.map(|(list, _)| list).unwrap_or_default(),
        project_id,
        migrated_from,
    })
}

/// Schema 32's links (docs/adr/0208 §2), as `service_links` says them: a
/// service's and a feed's connection is one of the project's; a layer drawn
/// from a service holds no objects.
fn service_links(
    r: &mut Reader<'_>,
    layers: &[LayerNode],
    settings: &ProjectSettings,
    entities: &[kentos_contracts::Entity],
) -> Result<(), KcadError> {
    let Some(fault) = kentos_contracts::service_links(layers, &settings.connections, entities)
    else {
        return Ok(());
    };
    let segs = match fault {
        kentos_contracts::ServiceLinkFault::Object { index } => {
            vec![Seg::Name("entities"), Seg::Index(index)]
        }
        _ => vec![Seg::Name("layers")],
    };
    for s in &segs {
        r.push(*s);
    }
    let e = r.fail(Code::BadValue, &fault.words());
    for _ in &segs {
        r.pop();
    }
    Err(e)
}

/// The settings; `has` says whether the schema has the drawing unit (11 and
/// up), the second coordinate system (12 and up), the project's own
/// systems and datum choices (13 and up) and the survey settings (14).
fn settings(r: &mut Reader<'_>, has: Features) -> Result<ProjectSettings, KcadError> {
    let (mut srid, mut area_unit, mut angle_unit, mut plot_scale) = (None, None, None, None);
    let (mut drawing_unit, mut second_srid) = (None, None);
    let (mut custom_crs, mut second_custom_crs, mut datum_transforms) = (None, None, Vec::new());
    let mut survey = None;
    let mut layer_states = Vec::new();
    let mut topology = None;
    let mut annotation = None;
    let mut connections = Vec::new();
    let (mut text_styles, mut dimension_styles) = (Vec::new(), Vec::new());
    let (mut workspace, mut drawing_font, mut area_decimals, mut length_decimals) =
        (None, None, None, None);
    map(r, |r, key| {
        match key {
            "srid" => srid = Some(r.uint(u64::from(u32::MAX))? as u32),
            "areaUnit" => {
                area_unit = Some(named(
                    r,
                    &[
                        ("m2", AreaUnit::M2),
                        ("donum", AreaUnit::Donum),
                        ("ha", AreaUnit::Ha),
                    ],
                )?)
            }
            "angleUnit" => {
                angle_unit = Some(named(
                    r,
                    &[("grad", AngleUnit::Grad), ("deg", AngleUnit::Deg)],
                )?)
            }
            "plotScale" => plot_scale = Some(r.float()?),
            "workspace" => {
                workspace = Some(named(
                    r,
                    &[
                        ("cad", Workspace::Cad),
                        ("gis", Workspace::Gis),
                        ("plan3d", Workspace::Plan3d),
                        ("disaster", Workspace::Disaster),
                        // The former Hibrit mode: a type not asked yet (docs/adr/0165 §1).
                        ("hybrid", Workspace::LegacyHybrid),
                    ],
                )?)
            }
            "drawingFont" => {
                drawing_font = Some(named(
                    r,
                    &[
                        ("barlow", DrawingFont::Barlow),
                        ("arimo", DrawingFont::Arimo),
                        ("overpass", DrawingFont::Overpass),
                        ("quicksand", DrawingFont::Quicksand),
                        ("architects-daughter", DrawingFont::ArchitectsDaughter),
                        ("courier-prime", DrawingFont::CourierPrime),
                        ("plex-mono", DrawingFont::PlexMono),
                    ],
                )?)
            }
            "drawingUnit" if has.drawing_unit => {
                drawing_unit = Some(named(
                    r,
                    &[
                        ("mm", DrawingUnit::Mm),
                        ("cm", DrawingUnit::Cm),
                        ("m", DrawingUnit::M),
                    ],
                )?)
            }
            "customCrs" if has.custom_crs => custom_crs = Some(crs::crs_definition(r)?),
            "secondCustomCrs" if has.custom_crs => {
                second_custom_crs = Some(crs::crs_definition(r)?)
            }
            "datumTransforms" if has.custom_crs => datum_transforms = crs::datum_transforms(r)?,
            "survey" if has.survey => survey = Some(survey_settings(r, has)?),
            "layerStates" if has.layer_states => layer_states = layer_states_list(r)?,
            "topology" if has.topology => topology = Some(topology_settings(r)?),
            "annotation" if has.annotation => annotation = Some(annotation_heights(r)?),
            "connections" if has.services => connections = services::connections(r)?,
            "textStyles" if has.styles => text_styles = styles::text_styles(r)?,
            "dimensionStyles" if has.styles => {
                dimension_styles = styles::dimension_styles(r, has.annotation)?
            }
            // `srid` and `customCrs` come first in the encoded order: the project's own system is known.
            "secondSrid" if has.second_srid => {
                let at = r.position();
                let second = r.uint(u64::from(u32::MAX))? as u32;
                if second == 0 || (srid == Some(0) && custom_crs.is_none()) || srid == Some(second)
                {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "ikinci koordinat sistemi projeninkinden başka bir sistem olmalı; yerel projenin ikinci sistemi olmaz",
                    ));
                }
                second_srid = Some(second);
            }
            "areaDecimals" => area_decimals = Some(r.uint(u64::from(u32::MAX))? as u32),
            "lengthDecimals" => length_decimals = Some(r.uint(u64::from(u32::MAX))? as u32),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let srid = required(r, srid, "srid")?;
    // A definition of its own is the system of a project without an EPSG
    // code; a second definition is no second EPSG code, and needs a system to
    // be the second of (docs/adr/0168 §1).
    let relation = |key: &'static str, what: &str, r: &mut Reader<'_>| {
        r.push(Seg::Name(key));
        let e = r.fail(Code::BadValue, what);
        r.pop();
        e
    };
    if custom_crs.is_some() && srid != 0 {
        return Err(relation(
            "customCrs",
            "projenin kendi tanımı yalnız EPSG kodu olmayan (srid 0) projede olur",
            r,
        ));
    }
    if second_custom_crs.is_some() && (second_srid.is_some() || (srid == 0 && custom_crs.is_none()))
    {
        return Err(relation(
            "secondCustomCrs",
            "ikinci sistem ya EPSG kodu ya tanımdır; koordinat sistemi olmayan projenin ikinci sistemi olmaz",
            r,
        ));
    }
    Ok(ProjectSettings {
        srid,
        length_decimals: required(r, length_decimals, "lengthDecimals")?,
        area_decimals: required(r, area_decimals, "areaDecimals")?,
        area_unit: required(r, area_unit, "areaUnit")?,
        angle_unit: required(r, angle_unit, "angleUnit")?,
        plot_scale: required(r, plot_scale, "plotScale")?,
        workspace,
        drawing_font,
        drawing_unit,
        second_srid,
        custom_crs,
        second_custom_crs,
        datum_transforms,
        survey,
        layer_states,
        text_styles,
        dimension_styles,
        topology,
        annotation,
        connections,
    })
}

/// Schema 30's annotation heights (docs/adr/0205 §1): each kind's paper
/// height, mm; a key the contract does not have is `unknown_field`, a height
/// that does not hold and an empty map (a writer leaves none out but the
/// field) are `bad_value`.
fn annotation_heights(r: &mut Reader<'_>) -> Result<AnnotationHeights, KcadError> {
    let at = r.position();
    let mut heights = AnnotationHeights::default();
    let mut any = false;
    map(r, |r, key| {
        let Some(kind) = AnnotationKind::from_key(key) else {
            return Err(unknown(r));
        };
        let at = r.position();
        let mm = r.float()?;
        if !annotation_mm_holds(mm) {
            return Err(r.fail_at(
                Code::BadValue,
                at,
                &format!(
                    "{} yüksekliği {mm} mm; sıfırdan büyük, en çok {MAX_ANNOTATION_MM} olmalı",
                    kind.label()
                ),
            ));
        }
        heights = std::mem::take(&mut heights).with(kind, Some(mm));
        any = true;
        Ok(())
    })?;
    if !any {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "yazı yükseklikleri boş; yüksekliği olmayan proje alanı yazmaz",
        ));
    }
    Ok(heights)
}

/// Schema 27's topology settings (docs/adr/0202 §7): the tolerance, the
/// rules (`id`, `kind`, `layer`, `other`, `value`) and the exceptions (`at`,
/// `rule`, `objects`), checked whole as the contract checks them
/// (`TopologySettings::problem`).
fn topology_settings(r: &mut Reader<'_>) -> Result<TopologySettings, KcadError> {
    let at = r.position();
    let mut t = TopologySettings::default();
    map(r, |r, key| {
        match key {
            "rules" => {
                t.rules = list(r, |r, _| {
                    let (mut id, mut kind, mut layer, mut other, mut value) =
                        (None, None, None, None, None);
                    map(r, |r, key| {
                        match key {
                            "id" => id = Some(text(r)?),
                            "kind" => {
                                let at = r.position();
                                let k = r.text()?;
                                kind = Some(TopologyRuleKind::of_key(k).ok_or_else(|| {
                                    r.fail_at(
                                        Code::BadValue,
                                        at,
                                        &format!("“{k}” bilinen bir topoloji kuralı değil"),
                                    )
                                })?);
                            }
                            "layer" => layer = Some(text(r)?),
                            "other" => other = Some(text(r)?),
                            "value" => value = Some(r.float()?),
                            _ => return Err(unknown(r)),
                        }
                        Ok(())
                    })?;
                    Ok(TopologyRule {
                        id: required(r, id, "id")?,
                        kind: required(r, kind, "kind")?,
                        layer: required(r, layer, "layer")?,
                        other,
                        value,
                    })
                })?
            }
            "tolerance" => t.tolerance = Some(r.float()?),
            "exceptions" => {
                t.exceptions = list(r, |r, _| {
                    let (mut at, mut rule, mut objects) = (None, None, None);
                    map(r, |r, key| {
                        match key {
                            "at" => at = Some(point(r)?),
                            "rule" => rule = Some(text(r)?),
                            "objects" => {
                                objects =
                                    Some(list(r, |r, _| id16(r).map(kentos_contracts::EntityId))?)
                            }
                            _ => return Err(unknown(r)),
                        }
                        Ok(())
                    })?;
                    Ok(TopologyException {
                        rule: required(r, rule, "rule")?,
                        objects: required(r, objects, "objects")?,
                        at: required(r, at, "at")?,
                    })
                })?
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    match t.problem() {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(t),
    }
}

/// Schema 19's layer states (docs/adr/0177 §4): each its id, name and nodes,
/// a node its id, visibility and, when kept, its lock and a layer's style;
/// checked whole (`layer_states_problem`): no empty or repeated id or name,
/// no empty or repeated node in a state.
fn layer_states_list(r: &mut Reader<'_>) -> Result<Vec<LayerState>, KcadError> {
    let at = r.position();
    let all = list(r, |r, _| {
        let (mut id, mut name, mut nodes) = (None, None, None);
        map(r, |r, key| {
            match key {
                "id" => id = Some(text(r)?),
                "name" => name = Some(text(r)?),
                "nodes" => nodes = Some(list(r, |r, _| layer_state_node(r))?),
                _ => return Err(unknown(r)),
            }
            Ok(())
        })?;
        Ok(LayerState {
            id: required(r, id, "id")?,
            name: required(r, name, "name")?,
            nodes: required(r, nodes, "nodes")?,
        })
    })?;
    match layer_states_problem(&all) {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(all),
    }
}

fn layer_state_node(r: &mut Reader<'_>) -> Result<LayerStateNode, KcadError> {
    let (mut node, mut visible, mut locked, mut style) = (None, None, None, None);
    map(r, |r, key| {
        match key {
            "node" => node = Some(text(r)?),
            "style" => style = Some(layer_style(r)?),
            "locked" => locked = Some(r.bool()?),
            "visible" => visible = Some(r.bool()?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(LayerStateNode {
        node: required(r, node, "node")?,
        visible: required(r, visible, "visible")?,
        locked,
        style,
    })
}

/// Schema 14's survey settings (docs/adr/0169 §3), checked whole: at least
/// one, k within [−1, 1], tolerances above zero; schema 15's traverse
/// tolerances too; schema 16's ground height within [−500, 9000] m and its
/// reduction to the grid only with one (docs/adr/0171); schema 28's a priori
/// standard deviations above zero, the parts per million and the centering
/// not below (docs/adr/0203 §1).
fn survey_settings(r: &mut Reader<'_>, has: Features) -> Result<SurveySettings, KcadError> {
    let at = r.position();
    let mut s = SurveySettings::default();
    map(r, |r, key| {
        match key {
            "index" => s.index = Some(r.float()?),
            "faceHz" => s.face_hz = Some(r.float()?),
            "faceSlope" => s.face_slope = Some(r.float()?),
            "refraction" => s.refraction = Some(r.float()?),
            "twoWay" if has.traverse_tolerances => s.two_way = Some(r.float()?),
            "traverseAngle" if has.traverse_tolerances => s.traverse_angle = Some(r.float()?),
            "traverseCoord" if has.traverse_tolerances => s.traverse_coord = Some(r.float()?),
            "groundHeight" if has.ground => s.ground_height = Some(r.float()?),
            "reduceToGrid" if has.ground => s.reduce_to_grid = Some(r.bool()?),
            "sigmaDirection" if has.survey_sigmas => s.sigma_direction = Some(r.float()?),
            "sigmaDistance" if has.survey_sigmas => s.sigma_distance = Some(r.float()?),
            "sigmaPpm" if has.survey_sigmas => s.sigma_ppm = Some(r.float()?),
            "sigmaCentering" if has.survey_sigmas => s.sigma_centering = Some(r.float()?),
            "sigmaZenith" if has.survey_sigmas => s.sigma_zenith = Some(r.float()?),
            "sigmaLevelling" if has.survey_sigmas => s.sigma_levelling = Some(r.float()?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    match s.problem() {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(s),
    }
}

fn source(r: &mut Reader<'_>) -> Result<MigrationSource, KcadError> {
    let at = r.position();
    let (mut format, mut version, mut sha) = (None, None, None);
    map(r, |r, key| {
        match key {
            "format" => format = Some(text(r)?),
            "version" => version = Some(r.uint(u64::from(u32::MAX))?),
            "sourceSha256" => {
                let (raw, at) = r.bytes()?;
                if raw.len() != 32 {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("{} bayt; SHA-256 32 bayt olmalı", raw.len()),
                    ));
                }
                sha = Some(hex(raw));
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let format = required(r, format, "format")?;
    let version = required(r, version, "version")?;
    let sha = required(r, sha, "sourceSha256")?;
    if format != DOCUMENT_FORMAT || version != u64::from(DOCUMENT_VERSION) {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "göç kaynağı yalnız kentos.document sürüm 1 olabilir",
        ));
    }
    Ok(MigrationSource::v1(sha))
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(char::from(DIGITS[usize::from(b >> 4)]));
        out.push(char::from(DIGITS[usize::from(b & 0x0f)]));
    }
    out
}

fn project_styles(r: &mut Reader<'_>) -> Result<ProjectStyles, KcadError> {
    let (mut items, mut categories) = (None, None);
    map(r, |r, key| {
        match key {
            "items" => items = Some(list(r, |r, _| r.opaque())?),
            "categories" => categories = Some(list(r, |r, _| r.opaque())?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(ProjectStyles {
        items: required(r, items, "items")?,
        categories: required(r, categories, "categories")?,
    })
}

// ── The layer tree (§6.5) ───────────────────────────────────────────────

fn layer(r: &mut Reader<'_>, has: Features) -> Result<LayerNode, KcadError> {
    let (mut id, mut name, mut kind, mut style) = (None, None, None, None);
    let (mut locked, mut visible, mut children, mut expanded) = (None, None, None, None);
    let (mut snap, mut fields, mut service, mut feed) = (None, None, None, None);
    map(r, |r, key| {
        match key {
            "id" => id = Some(text(r)?),
            "name" => name = Some(text(r)?),
            "snap" if has.layer_snap => snap = Some(layer_snap(r)?),
            "fields" if has.layer_fields => fields = Some(layer_fields(r)?),
            "service" if has.services => service = Some(services::service_layer(r)?),
            "feed" if has.services => feed = Some(services::feature_feed(r)?),
            "type" => {
                kind = Some(named(
                    r,
                    &[
                        ("group", LayerNodeType::Group),
                        ("layer", LayerNodeType::Layer),
                    ],
                )?)
            }
            "style" => style = Some(layer_style(r)?),
            "locked" => locked = Some(r.bool()?),
            "visible" => visible = Some(r.bool()?),
            "children" => children = Some(list(r, |r, _| layer(r, has))?),
            "expanded" => expanded = Some(r.bool()?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(LayerNode {
        id: required(r, id, "id")?,
        name: required(r, name, "name")?,
        kind: required(r, kind, "type")?,
        visible: required(r, visible, "visible")?,
        locked: required(r, locked, "locked")?,
        expanded: required(r, expanded, "expanded")?,
        style: required(r, style, "style")?,
        children: required(r, children, "children")?,
        snap: match (snap, kind) {
            (Some(_), Some(LayerNodeType::Group)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "grubun keneti olmaz; kenet yalnız katmanındır",
                ));
            }
            (snap, _) => snap,
        },
        fields: match (fields, kind) {
            (Some(_), Some(LayerNodeType::Group)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "grubun alanları olmaz; alanlar yalnız katmanındır",
                ));
            }
            (fields, _) => fields.unwrap_or_default(),
        },
        service: match (service, kind) {
            (Some(_), _) if feed.is_some() => {
                return Err(r.fail(
                    Code::BadValue,
                    "katman hem servisten çizilir hem nesnelerini bir kaynaktan alır; ikisi birden olmaz",
                ));
            }
            (Some(_), Some(LayerNodeType::Group)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "grubun servisi olmaz; servis yalnız katmanındır",
                ));
            }
            (service, _) => service,
        },
        feed: match (feed, kind) {
            (Some(_), Some(LayerNodeType::Group)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "grubun veri kaynağı olmaz; kaynak yalnız katmanındır",
                ));
            }
            (feed, _) => feed,
        },
    })
}

/// Schema 26's fields of a layer (docs/adr/0199 §1), checked whole
/// (`layer_fields_problem`): a list that is not empty, each field by its
/// kind's rules, no name twice.
fn layer_fields(r: &mut Reader<'_>) -> Result<Vec<LayerField>, KcadError> {
    let at = r.position();
    let all = list(r, |r, _| layer_field(r))?;
    match layer_fields_problem(&all) {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(all),
    }
}

fn layer_field(r: &mut Reader<'_>) -> Result<LayerField, KcadError> {
    let (mut name, mut kind, mut alias, mut length, mut scale) = (None, None, None, None, None);
    let (mut min, mut max, mut values, mut default, mut must) = (None, None, None, None, None);
    let count = |r: &mut Reader<'_>| r.uint(u64::from(u32::MAX)).map(|n| n as u32);
    map(r, |r, key| {
        match key {
            "max" => max = Some(text(r)?),
            "min" => min = Some(text(r)?),
            "kind" => kind = Some(named(r, &LayerFieldKind::ALL.map(|k| (k.name(), k)))?),
            "name" => name = Some(text(r)?),
            "alias" => alias = Some(text(r)?),
            "scale" => scale = Some(count(r)?),
            "length" => length = Some(count(r)?),
            "values" => values = Some(list(r, |r, _| field_choice(r))?),
            "default" => default = Some(text(r)?),
            "required" => must = Some(r.bool()?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    if must == Some(false) {
        return Err(r.fail(
            Code::BadValue,
            "required yalnız true yazılır; alan zorunlu değilse yazılmaz",
        ));
    }
    Ok(LayerField {
        name: required(r, name, "name")?,
        alias,
        kind: required(r, kind, "kind")?,
        length,
        scale,
        min,
        max,
        values,
        required: must == Some(true),
        default,
    })
}

fn field_choice(r: &mut Reader<'_>) -> Result<FieldChoice, KcadError> {
    let (mut code, mut label) = (None, None);
    map(r, |r, key| {
        match key {
            "code" => code = Some(text(r)?),
            "label" => label = Some(text(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(FieldChoice {
        code: required(r, code, "code")?,
        label: required(r, label, "label")?,
    })
}

/// A layer's own snapping (docs/adr/0163 §4): `off` (true only) or `kinds`.
fn layer_snap(r: &mut Reader<'_>) -> Result<LayerSnap, KcadError> {
    let mut snap = LayerSnap::default();
    let mut off = None;
    map(r, |r, key| {
        match key {
            "off" => off = Some(r.bool()?),
            "kinds" => snap.kinds = Some(list(r, |r, _| text(r))?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    if off == Some(false) {
        return Err(r.fail(
            Code::BadValue,
            "off yalnız true yazılır; kenet kapalı değilse alan yazılmaz",
        ));
    }
    snap.off = off == Some(true);
    match snap.problem() {
        Some(problem) => Err(r.fail(Code::BadValue, &problem)),
        None => Ok(snap),
    }
}

/// A line type by its name (a layer style's, a dimension's lines'; §6.4).
pub(super) fn line_type_named(r: &mut Reader<'_>) -> Result<LineType, KcadError> {
    named(
        r,
        &[
            ("continuous", LineType::Continuous),
            ("dashed", LineType::Dashed),
            ("dashdot", LineType::Dashdot),
            ("dotted", LineType::Dotted),
        ],
    )
}

fn layer_style(r: &mut Reader<'_>) -> Result<LayerStyle, KcadError> {
    let (mut fill, mut color, mut label, mut point_) = (None, None, None, None);
    let (mut line_type, mut renderer, mut line_weight, mut pick_interior) =
        (None, None, None, None);
    map(r, |r, key| {
        match key {
            "fill" => fill = Some(text(r)?),
            "color" => color = Some(text(r)?),
            "label" => label = Some(label_style(r)?),
            "point" => point_ = Some(point_style(r)?),
            "lineType" => line_type = Some(line_type_named(r)?),
            "renderer" => {
                if r.next_is_null() {
                    return Err(r.fail(
                        Code::BadValue,
                        "null yazılmaz; çizici yoksa alan hiç yazılmaz",
                    ));
                }
                renderer = Some(r.opaque()?);
            }
            "lineWeight" => line_weight = Some(r.float()?),
            "pickInterior" => pick_interior = Some(r.bool()?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(LayerStyle {
        color: required(r, color, "color")?,
        line_type: required(r, line_type, "lineType")?,
        line_weight: required(r, line_weight, "lineWeight")?,
        fill,
        point: point_,
        label,
        pick_interior,
        renderer,
    })
}

fn point_style(r: &mut Reader<'_>) -> Result<PointStyle, KcadError> {
    let (mut size, mut symbol) = (None, None);
    map(r, |r, key| {
        match key {
            "size" => size = Some(r.float()?),
            "symbol" => {
                symbol = Some(named(
                    r,
                    &[
                        ("ring", PointSymbol::Ring),
                        ("cross", PointSymbol::Cross),
                        ("triangle", PointSymbol::Triangle),
                    ],
                )?)
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(PointStyle {
        symbol: required(r, symbol, "symbol")?,
        size: required(r, size, "size")?,
    })
}

fn label_style(r: &mut Reader<'_>) -> Result<LabelStyle, KcadError> {
    let (mut ink, mut grow, mut size, mut weight, mut max_size) = (None, None, None, None, None);
    let (mut max_scale, mut min_scale, mut template, mut placement, mut min_feature_px) =
        (None, None, None, None, None);
    map(r, |r, key| {
        match key {
            "ink" => {
                ink = Some(named(
                    r,
                    &[
                        ("fg", LabelInk::Fg),
                        ("fg-dim", LabelInk::FgDim),
                        ("label", LabelInk::Label),
                    ],
                )?)
            }
            "grow" => grow = Some(r.float()?),
            "size" => size = Some(r.float()?),
            "weight" => weight = Some(r.uint(u64::from(u16::MAX))? as u16),
            "maxSize" => max_size = Some(r.float()?),
            "maxScale" => max_scale = Some(r.float()?),
            "minScale" => min_scale = Some(r.float()?),
            "template" => template = Some(text(r)?),
            "placement" => {
                placement = Some(named(
                    r,
                    &[
                        ("center", LabelPlacement::Center),
                        ("corner", LabelPlacement::Corner),
                        ("beside", LabelPlacement::Beside),
                        ("along", LabelPlacement::Along),
                    ],
                )?)
            }
            "minFeaturePx" => min_feature_px = Some(r.float()?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(LabelStyle {
        placement: required(r, placement, "placement")?,
        size: required(r, size, "size")?,
        grow,
        max_size,
        weight,
        template,
        min_feature_px,
        min_scale,
        max_scale,
        ink,
    })
}
