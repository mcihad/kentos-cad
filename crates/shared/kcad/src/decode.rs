//! Document schema 2 read from a payload (docs/specs/kcad-v2.md §6), straight
//! into the contract (`DocumentSnapshotV2`): item by item, no intermediate
//! tree, so memory follows the drawing, not what a file claims. Every map is
//! checked for its keys (unknown ones are refused, required ones must be
//! there) and every value for its type; the objects get the slots 1, 2, 3 …
//! in file order. The objects themselves are read in `objects.rs`.

mod objects;

use kentos_contracts::{
    AngleUnit, AreaUnit, Bounds, DOCUMENT_FORMAT, DOCUMENT_VERSION, DOCUMENT_VERSION_2,
    DocumentSnapshotV2, DrawingFont, LabelInk, LabelPlacement, LabelStyle, LayerNode,
    LayerNodeType, LayerStyle, LineType, MigrationSource, PointStyle, PointSymbol, ProjectId,
    ProjectSettings, ProjectStyles, Vec2, Workspace,
};

use crate::cbor::{Any, Reader, Seg};
use crate::error::{Code, KcadError};
use crate::watch::Watch;
use objects::objects;

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
        r.fail(
            Code::SchemaVersion,
            &format!(
                "belge şeması sürümü {v} bu uygulamada okunamıyor (desteklenen: {DOCUMENT_VERSION_2}); KentOS'u güncelleyin"
            ),
        )
    };
    let (mut format_ok, mut version_ok) = (false, false);
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
                Any::Uint(v) if v == u64::from(DOCUMENT_VERSION_2) => version_ok = true,
                Any::Uint(v) => return Err(version_error(r, &v.to_string())),
                _ => return Err(version_error(r, "(sayı değil)")),
            },
            "document" => {
                if !format_ok {
                    return Err(not_ours(r));
                }
                if !version_ok {
                    return Err(version_error(r, "(yok)"));
                }
                document = Some(body(r)?);
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    if !format_ok {
        return Err(not_ours(r));
    }
    if !version_ok {
        return Err(version_error(r, "(yok)"));
    }
    required(r, document, "document")
}

fn body(r: &mut Reader<'_>) -> Result<DocumentSnapshotV2, KcadError> {
    let mut name = None;
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
            "layers" => layers = Some(list(r, |r, _| layer(r))?),
            "origin" => origin = Some(point(r)?),
            "styles" => styles = Some(project_styles(r)?),
            // Name and layers come first in the encoded order: the project is known before its objects.
            "entities" => {
                entities = Some(objects(
                    r,
                    name.as_deref(),
                    layers.as_ref().map_or(0, Vec::len),
                )?)
            }
            "homeView" => home_view = Some(bounds(r)?),
            "settings" => settings_ = Some(settings(r)?),
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
        project_id,
        migrated_from,
    })
}

fn settings(r: &mut Reader<'_>) -> Result<ProjectSettings, KcadError> {
    let (mut srid, mut area_unit, mut angle_unit, mut plot_scale) = (None, None, None, None);
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
                        ("hybrid", Workspace::Hybrid),
                        ("cad", Workspace::Cad),
                        ("gis", Workspace::Gis),
                        ("plan3d", Workspace::Plan3d),
                        ("disaster", Workspace::Disaster),
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
            "areaDecimals" => area_decimals = Some(r.uint(u64::from(u32::MAX))? as u32),
            "lengthDecimals" => length_decimals = Some(r.uint(u64::from(u32::MAX))? as u32),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(ProjectSettings {
        srid: required(r, srid, "srid")?,
        length_decimals: required(r, length_decimals, "lengthDecimals")?,
        area_decimals: required(r, area_decimals, "areaDecimals")?,
        area_unit: required(r, area_unit, "areaUnit")?,
        angle_unit: required(r, angle_unit, "angleUnit")?,
        plot_scale: required(r, plot_scale, "plotScale")?,
        workspace,
        drawing_font,
    })
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

fn layer(r: &mut Reader<'_>) -> Result<LayerNode, KcadError> {
    let (mut id, mut name, mut kind, mut style) = (None, None, None, None);
    let (mut locked, mut visible, mut children, mut expanded) = (None, None, None, None);
    map(r, |r, key| {
        match key {
            "id" => id = Some(text(r)?),
            "name" => name = Some(text(r)?),
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
            "children" => children = Some(list(r, |r, _| layer(r))?),
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
    })
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
            "lineType" => {
                line_type = Some(named(
                    r,
                    &[
                        ("continuous", LineType::Continuous),
                        ("dashed", LineType::Dashed),
                        ("dashdot", LineType::Dashdot),
                        ("dotted", LineType::Dotted),
                    ],
                )?)
            }
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
