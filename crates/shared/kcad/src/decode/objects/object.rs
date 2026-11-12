//! One object read field by field (docs/specs/kcad-v2.md): every key a kind may carry, checked against the
//! schema's features.

use super::*;

// Out of line on purpose: a browser's WebAssembly engine runs a function in its
// baseline code until the function is called again (no on-stack replacement), so the
// work of each object must be a function called once per object, not inlined into the
// one loop that runs once for the whole drawing (docs/adr/0030).
//
// The object, its persistent id (`None` when `has` says objects have none)
// and, for an insert, where its `block` is.
#[inline(never)]
pub(crate) fn object(
    r: &mut Reader<'_>,
    index: usize,
    has: Features,
) -> Result<(Entity, Option<EntityId>, usize), KcadError> {
    let (n, at) = r.map()?;
    if n != 1 {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            &format!("nesne haritasında tek anahtar (tür) olmalı, {n} var"),
        ));
    }
    let mut previous = None;
    let name = r.key(&mut previous)?;
    r.push(Seg::Key(name));
    let Some(&(_, kind)) = KINDS.iter().find(|(k, _)| *k == name).filter(|(_, kind)| {
        (has.blocks || *kind != Kind::Insert)
            && (has.leaders || *kind != Kind::Leader)
            && (has.tables || *kind != Kind::Table)
            && (has.images || *kind != Kind::Image)
            && (has.rasters || *kind != Kind::Raster)
            && (has.point_clouds || *kind != Kind::PointCloud)
            && (has.nurbs || *kind != Kind::Nurbs)
            && (has.walls || *kind != Kind::Wall)
            && (has.openings || *kind != Kind::Opening)
            && (has.spaces || *kind != Kind::Space)
            && (has.arch_elements
                || !matches!(kind, Kind::Grid | Kind::Column | Kind::Stair | Kind::Ramp))
            && (has.tags || *kind != Kind::Tag)
            && (has.viewmarks || *kind != Kind::ViewMark)
            && (has.slabs || *kind != Kind::Slab)
            && (has.beams || *kind != Kind::Beam)
            && (has.roofs || *kind != Kind::Roof)
            && (has.railing_objects || *kind != Kind::Railing)
            && (has.forms || *kind != Kind::Record)
    }) else {
        return Err(r.fail(
            Code::UnknownKind,
            &format!(
                "“{name}” nesne türü bilinmiyor; dosya daha yeni bir KentOS'la yazılmış olabilir"
            ),
        ));
    };
    // Only the drawing's: a block definition holds no table (docs/adr/0184 §1).
    if kind == Kind::Table && !has.uids {
        return Err(r.fail(Code::BadValue, "blok tanımında tablo olamaz"));
    }
    // Only the drawing's: a block definition holds no picture (docs/adr/0192 §1).
    if kind == Kind::Image && !has.uids {
        return Err(r.fail(Code::BadValue, "blok tanımında resim olamaz"));
    }
    // Nor a raster (docs/adr/0204 §2).
    if kind == Kind::Raster && !has.uids {
        return Err(r.fail(Code::BadValue, "blok tanımında raster olamaz"));
    }
    // Nor a point cloud (docs/adr/0207 §3).
    if kind == Kind::PointCloud && !has.uids {
        return Err(r.fail(Code::BadValue, "blok tanımında nokta bulutu olamaz"));
    }
    // Nor a wall (docs/adr/0339 §4): a block holds its drawing, not its source.
    if kind == Kind::Wall && !has.uids {
        return Err(r.fail(Code::BadValue, "blok tanımında duvar olamaz"));
    }
    // Nor a grid, column, stair or ramp: they are the drawing's plan (docs/adr/0343 §1).
    if matches!(kind, Kind::Grid | Kind::Column | Kind::Stair | Kind::Ramp) && !has.uids {
        return Err(r.fail(
            Code::BadValue,
            "blok tanımında aks, kolon, merdiven ya da rampa olamaz",
        ));
    }
    // Nor a tag: its object is the drawing's (docs/adr/0344 §1).
    if kind == Kind::Tag && !has.uids {
        return Err(r.fail(Code::BadValue, "blok tanımında etiket olamaz"));
    }
    // Nor a view mark: its target is the drawing's (docs/adr/0345 §1).
    if kind == Kind::ViewMark && !has.uids {
        return Err(r.fail(Code::BadValue, "blok tanımında görünüm işareti olamaz"));
    }
    // Nor a slab: its walls are the drawing's (docs/adr/0722 §1).
    if kind == Kind::Slab && !has.uids {
        return Err(r.fail(Code::BadValue, "blok tanımında döşeme olamaz"));
    }
    // Nor a beam: it hangs from the drawing's storeys and slabs (docs/adr/8041 §1).
    if kind == Kind::Beam && !has.uids {
        return Err(r.fail(Code::BadValue, "blok tanımında kiriş olamaz"));
    }
    // Nor a roof: it stands on the drawing's storeys (docs/adr/8044 §1).
    if kind == Kind::Roof && !has.uids {
        return Err(r.fail(Code::BadValue, "blok tanımında çatı olamaz"));
    }
    // Nor a railing: it stands on the drawing's storeys (docs/adr/8045 §3).
    if kind == Kind::Railing && !has.uids {
        return Err(r.fail(Code::BadValue, "blok tanımında korkuluk olamaz"));
    }
    // Nor a space: its walls are the drawing's (docs/adr/0342 §1).
    if kind == Kind::Space && !has.uids {
        return Err(r.fail(Code::BadValue, "blok tanımında mahal olamaz"));
    }
    // Nor an opening: its wall is the drawing's (docs/adr/0341 §1).
    if kind == Kind::Opening && !has.uids {
        return Err(r.fail(Code::BadValue, "blok tanımında açıklık olamaz"));
    }
    // Nor a record without a geometry (docs/adr/0410 §2).
    if kind == Kind::Record && !has.uids {
        return Err(r.fail(Code::BadValue, "blok tanımında geometrisiz kayıt olamaz"));
    }
    let mut f = Fields::default();
    map(r, |r, key| {
        if !allowed(kind, key, has) {
            return Err(unknown(r));
        }
        match key {
            "uid" => f.uid = Some(EntityId(id16(r)?)),
            "attrs" => {
                let mut attrs = BTreeMap::new();
                map(r, |r, k| {
                    attrs.insert(k.to_owned(), text(r)?);
                    Ok(())
                })?;
                f.attrs = Some(attrs);
            }
            "color" => f.color = Some(text(r)?),
            "label" => f.label = Some(text(r)?),
            "labelPins" => f.label_pins = Some(super::super::labels::label_pins(r)?),
            "attachments" => f.attachments = Some(super::super::info::attachments(r)?),
            "symbol" => f.symbol = Some(text(r)?),
            "layerId" => f.layer_id = Some(text(r)?),
            "lineWeight" => {
                let w = r.float()?;
                if !(0.0..=MAX_LINE_WEIGHT).contains(&w) {
                    return Err(r.fail(
                        Code::BadValue,
                        &format!("çizgi kalınlığı {w} mm; 0 ile {MAX_LINE_WEIGHT} arasında olmalı"),
                    ));
                }
                f.line_weight = Some(w);
            }
            "lineType" => {
                f.object_look.line_type = Some(super::super::named(
                    r,
                    &kentos_contracts::LineType::ALL.map(|t| (t.name(), t)),
                )?)
            }
            "lineTypeScale" => f.object_look.line_type_scale = Some(r.float()?),
            "linePattern" => f.object_look.line_pattern = Some(text(r)?),
            "transparency" => f.object_look.transparency = Some(r.float()?),
            "order" => f.object_look.order = Some(r.int(crate::MAX_ORDER)? as i32),
            "wipeout" => f.object_look.wipeout = Some(wipeout(r)?),
            "byLayer" => {
                f.object_look.by_layer = list(r, |r, _| {
                    super::super::named(r, &kentos_contracts::LookPart::ALL.map(|p| (p.name(), p)))
                })?
            }
            "p" => f.p = Some(point(r)?),
            "a" => f.a = Some(point(r)?),
            "b" => f.b = Some(point(r)?),
            "c" => f.c = Some(point(r)?),
            "major" => f.major = Some(point(r)?),
            "dir" => f.dir = Some(point(r)?),
            "z" => f.z = Some(r.float()?),
            "za" => {
                f.za_at = r.position();
                f.za = Some(r.float()?);
            }
            "zb" => {
                f.zb_at = r.position();
                f.zb = Some(r.float()?);
            }
            "ma" => f.ma = Some(r.float()?),
            "mb" => f.mb = Some(r.float()?),
            "r" => f.r = Some(r.float()?),
            "a0" => f.a0 = Some(r.float()?),
            "a1" => f.a1 = Some(r.float()?),
            "ratio" => f.ratio = Some(r.float()?),
            "t0" => f.t0 = Some(r.float()?),
            "t1" => f.t1 = Some(r.float()?),
            "width" if kind == Kind::Raster => {
                f.raster.width = Some(r.uint(u64::from(MAX_RASTER_SIDE))? as u32)
            }
            "height" if kind == Kind::Raster => {
                f.raster.height = Some(r.uint(u64::from(MAX_RASTER_SIDE))? as u32)
            }
            "height" if kind == Kind::Leader => {
                let at = r.position();
                let h = r.float()?;
                if h <= 0.0 {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("kılavuzun yüksekliği {h}; 0'dan büyük olmalı"),
                    ));
                }
                f.height = Some(h);
            }
            "height" => {
                f.table_at.height = r.position();
                f.height = Some(r.float()?);
            }
            "rotation" => f.rotation = Some(r.float()?),
            "bands" => f.raster.bands = Some(r.uint(u64::from(MAX_RASTER_BANDS))? as u32),
            "sample" => {
                let names: Vec<(&str, RasterSample)> =
                    RasterSample::ALL.iter().map(|s| (s.name(), *s)).collect();
                f.raster.sample = Some(named(r, &names)?);
            }
            "srid" => f.raster.srid = Some(r.uint(u64::from(u32::MAX))? as u32),
            "affine" => {
                let at = r.position();
                let v = floats(r)?;
                let affine: [f64; 6] = v.as_slice().try_into().map_err(|_| {
                    r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("rasterin dönüşümü 6 sayı olmalı, {} var", v.len()),
                    )
                })?;
                f.raster.affine = Some(affine);
            }
            "style" if kind == Kind::Raster => {
                f.raster.style = Some(raster_style(r, has.multidim)?)
            }
            "dataset" => f.raster.dataset = Some(raster_dataset(r)?),
            "georef" => f.raster.georef = Some(Box::new(raster_georef(r, has.georef_crs)?)),
            "tiles" => f.raster.tiles = Some(raster_tiles(r)?),
            "style" if kind == Kind::PointCloud => f.cloud.style = Some(cloud_style(r)?),
            "sources" => f.cloud.sources = Some(cloud_sources(r, has)?),
            "bounds" => f.cloud.bounds = Some(bounds6(r)?),
            "count" => f.cloud.count = Some(r.uint(u64::MAX)?),
            "url" => f.url = Some(text(r)?),
            "width" => f.width = Some(r.float()?),
            "asset" => f.asset = Some(text(r)?),
            "file" => f.file = Some(text(r)?),
            "clip" => f.clip = Some(points(r)?),
            "path" => {
                f.path_at = r.position();
                f.text_path = Some(text_path(r)?);
            }
            "opacity" => f.opacity = Some(r.float()?),
            "rows" => {
                f.table_at.rows = r.position();
                f.rows = Some(floats(r)?);
            }
            "columns" => {
                f.table_at.columns = r.position();
                f.columns = Some(floats(r)?);
            }
            "cells" => {
                f.table_at.cells = r.position();
                let cells: Vec<Vec<String>> = list(r, |r, _| list(r, |r, _| text(r)))?;
                // Schema 182 (docs/adr/1030 §7): a cell's words of several lines.
                if !has.extraction && cells.iter().flatten().any(|w| w.contains('\n')) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        f.table_at.cells,
                        "hücrede satır sonu var; çok satırlı hücre şema 182'den beri yazılır",
                    ));
                }
                f.cells = Some(cells);
            }
            "tableStyle" if kind == Kind::Table && has.table_styles => {
                let at = r.position();
                let id = text(r)?;
                if id.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "tablonun tablo stili kimliği boş"));
                }
                f.table_style = Some(id);
            }
            "breaks" if kind == Kind::Table => {
                let (v, at): (kentos_contracts::TableBreaks, usize) = json_part(r, "tablonun parçaları")?;
                if let Some(words) = v.problem() {
                    return Err(r.fail_at(Code::BadValue, at, &words));
                }
                f.table_breaks = Some(v);
            }
            "merges" => {
                f.table_at.merges = r.position();
                let ranges = list(r, |r, _| cell_range(r))?;
                // The writer leaves an empty list out: one spelling (§6.6).
                if ranges.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        f.table_at.merges,
                        "birleşik alan listesi boş; birleşik alanı olmayan tabloda alan yazılmaz",
                    ));
                }
                f.merges = Some(ranges);
            }
            "aligns" => {
                f.table_at.aligns = r.position();
                let names: Vec<(&str, TableAlign)> =
                    TableAlign::ALL.iter().map(|a| (a.name(), *a)).collect();
                f.aligns = Some(list(r, |r, _| named(r, &names))?);
            }
            "header" => {
                let at = r.position();
                if !r.bool()? {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "başlık false yazılmaz; başlıksız tabloda alan yoktur",
                    ));
                }
                f.header = Some(true);
            }
            // Schema 79 (docs/adr/0343 §1): a grid's own fields, not a table's grid.
            "grid" if kind == Kind::Grid => f.grid_line = Some(json_part(r, "aksın alanları")?.0),
            // Schema 85 (docs/adr/0351 §1): its JSON, read back exactly as the writer writes it.
            "clipping" => {
                let (read, at): (kentos_contracts::InsertClip, usize) =
                    json_part(r, "kırpma sınırı")?;
                if let Some(why) = read.problem() {
                    return Err(r.fail_at(Code::BadValue, at, &why));
                }
                f.clipping = Some(read);
            }
            "grid" => {
                let names: Vec<(&str, TableGrid)> =
                    TableGrid::ALL.iter().map(|g| (g.name(), *g)).collect();
                f.grid = Some(named(r, &names)?);
            }
            "frame" => {
                f.table_at.frame = r.position();
                f.frame = Some(r.float()?);
            }
            "source" => {
                f.table_at.source = r.position();
                f.source = Some(table_source(r)?);
            }
            "offset" => f.offset = Some(r.float()?),
            "angle" => {
                f.angle_at = r.position();
                f.angle = Some(r.float()?);
            }
            "pts" if kind == Kind::Leader => {
                let at = r.position();
                let pts = points(r)?;
                if pts.len() < 2 {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("kılavuzun {} köşesi var; en az iki olmalı", pts.len()),
                    ));
                }
                f.pts = Some(pts);
            }
            "pts" => f.pts = Some(points(r)?),
            "ring" => f.ring = Some(points(r)?),
            "bulges" => f.bulges = Some(floats(r)?),
            "zs" => {
                f.zs_at = r.position();
                f.zs = Some(elevations(r)?);
            }
            "ms" => {
                f.ms_at = r.position();
                f.ms = Some(elevations(r)?);
            }
            "holes" if kind == Kind::Polygon => f.rings = Some(list(r, |r, _| ring(r, has))?),
            "holes" => f.loops = Some(list(r, |r, _| points(r))?),
            // Schema 79 (docs/adr/0343 §1): a column's, stair's or ramp's fields.
            "column" => f.column = Some(json_part(r, "kolonun alanları")?.0),
            "stair" => f.stair = Some(json_part(r, "merdivenin alanları")?.0),
            "ramp" if kind == Kind::Ramp => f.ramp = Some(json_part(r, "rampanın alanları")?.0),
            // Schema 80 (docs/adr/0344 §1): a tag's fields.
            "tag" if kind == Kind::Tag => f.tag = Some(json_part(r, "etiketin alanları")?.0),
            // Schema 81 (docs/adr/0345 §1): a view mark's fields.
            "viewmark" if kind == Kind::ViewMark => {
                f.viewmark = Some(json_part(r, "görünüm işaretinin alanları")?.0)
            }
            // Schema 123 (docs/adr/0722): a slab's fields.
            "slab" if kind == Kind::Slab => f.slab = Some(json_part(r, "döşemenin alanları")?.0),
            "beam" if kind == Kind::Beam => f.beam = Some(json_part(r, "kirişin alanları")?.0),
            "roof" if kind == Kind::Roof => f.roof = Some(json_part(r, "çatının alanları")?.0),
            "railing" if kind == Kind::Railing => f.railing = Some(json_part(r, "korkuluğun alanları")?.0),
            // Schema 78 (docs/adr/0342 §1): a space's parts and the rest of its fields.
            "parts" if kind == Kind::Space => {
                let (v, _): (Vec<kentos_contracts::SpacePart>, usize) =
                    json_part(r, "mahalin sınırı")?;
                f.space_parts = Some(v);
            }
            "space" => {
                let at = r.position();
                let value = r.opaque()?;
                if !has.space_sources
                    && (value.get("sources").is_some()
                        || value.get("broken").and_then(serde_json::Value::as_str)
                            == Some("sourceUnavailable"))
                {
                    return Err(r.fail_at(
                        Code::UnknownField,
                        at,
                        "mahalın seçilmiş kaynakları şema 110'dan beri yazılır",
                    ));
                }
                if !has.fixtures && value.get("finishes").and_then(|f| f.get("base")).is_some() {
                    return Err(r.fail_at(
                        Code::UnknownField,
                        at,
                        "mahalin süpürgelik kaplaması şema 131'den beri yazılır",
                    ));
                }
                if !has.space_voids && value.get("void").is_some() {
                    return Err(r.fail_at(
                        Code::UnknownField,
                        at,
                        "mahalın boşluk işareti (void) şema 116'dan beri yazılır",
                    ));
                }
                if !has.space_scope && value.get("scope").is_some() {
                    return Err(r.fail_at(
                        Code::UnknownField,
                        at,
                        "mahalın kaynak kapsamı şema 109'dan beri yazılır",
                    ));
                }
                if !has.space_columns {
                    if value.get("columns").is_some() {
                        return Err(r.fail_at(
                            Code::UnknownField,
                            at,
                            "mahalın kolon tercihi şema 108'den beri yazılır",
                        ));
                    }
                    if value
                        .get("broken")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|b| b != "lostEnclosure")
                    {
                        return Err(r.fail_at(
                            Code::BadValue,
                            at,
                            "bu mahal kopukluk nedeni şema 108'den beri yazılır",
                        ));
                    }
                }
                let (v, _): (crate::space::SpaceRest, usize) =
                    json_value_part(r, "mahalin alanları", value, at)?;
                f.space = Some(v);
            }
            "spaceSeparator" if kind == Kind::Polyline => {
                f.space_separator = Some(json_part(r, "mahal ayırıcı rolü")?.0)
            }
            "parts" if kind == Kind::Point => f.point_parts = Some(list(r, |r, _| point_part(r))?),
            "parts" if kind == Kind::Polyline => f.parts = Some(list(r, |r, _| line_part(r, has))?),
            "parts" => f.parts = Some(list(r, |r, _| part(r, has))?),
            // A closed polyline (docs/adr/0258 §1): an open one has no field.
            "closed" if kind == Kind::Polyline => {
                let at = r.position();
                if !r.bool()? {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "kapalılık false yazılmaz; açık çoklu çizgide alan yoktur",
                    ));
                }
                f.closed = Some(true);
            }
            // A NURBS curve's words are written only when true, as a closed polyline's (docs/adr/0259).
            "closed" | "periodic" if kind == Kind::Nurbs => {
                let at = r.position();
                if !r.bool()? {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("{key} false yazılmaz; alan yalnız true iken yazılır"),
                    ));
                }
                if key == "closed" {
                    f.closed = Some(true);
                } else {
                    f.periodic = true;
                }
            }
            "degree" => {
                f.degree = Some(r.uint(u64::from(kentos_contracts::MAX_NURBS_DEGREE))? as u32)
            }
            "ctrl" => f.ctrl = Some(points(r)?),
            "knots" => f.knots = Some(floats(r)?),
            "weights" => f.weights = Some(floats(r)?),
            "fit" => f.fit = Some(points(r)?),
            "fitTolerance" => f.fit_tolerance = Some(r.float()?),
            "tangents" => {
                let at = r.position();
                let n = r.array()?;
                if n != 2 {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("teğetler 2 öğe olmalı (başlangıç, bitiş), {n} var"),
                    ));
                }
                let mut pair = [None, None];
                for t in &mut pair {
                    *t = if r.next_is_null() {
                        r.float_or_null()?;
                        None
                    } else {
                        Some(point(r)?)
                    };
                }
                r.leave();
                f.tangents = Some(pair);
            }
            "closed" => f.closed = Some(r.bool()?),
            "widths" => f.widths = Some(list(r, |r, _| width(r))?),
            "text" if kind == Kind::Leader => {
                let at = r.position();
                let note = text(r)?;
                if note.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "kılavuzun notu boş; notsuz kılavuzun not alanı yazılmaz",
                    ));
                }
                f.text = Some(note);
            }
            "text" => f.text = Some(text(r)?),
            // Schema 77 (docs/adr/0341 §1): an opening's wall, place along it, jambs and the rest.
            "host" if kind == Kind::Opening => f.opening_host = Some(text(r)?),
            "at" if kind == Kind::Opening => f.opening_at = Some(r.float()?),
            "place" if kind == Kind::Opening => f.opening_place = Some(points(r)?),
            "opening" => {
                let (v, _): (crate::opening::OpeningRest, usize) =
                    json_part(r, "açıklığın alanları")?;
                f.opening = Some(v);
            }
            // Schema 75 (docs/adr/0339 §4): a wall's style, justification, ends, fill, components and information.
            "style" if kind == Kind::Wall => f.wall_style = Some(text(r)?),
            "justify" => {
                f.wall_justify = Some(named(
                    r,
                    &[
                        ("left", kentos_contracts::WallJustify::Left),
                        ("center", kentos_contracts::WallJustify::Center),
                        ("right", kentos_contracts::WallJustify::Right),
                    ],
                )?)
            }
            "caps" => {
                let at = r.position();
                if named(
                    r,
                    &[
                        ("open", kentos_contracts::WallCaps::Open),
                        ("closed", kentos_contracts::WallCaps::Closed),
                    ],
                )? != kentos_contracts::WallCaps::Open
                {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "uçlar kapalıyken yazılmaz; alan yalnız open iken yazılır",
                    ));
                }
                f.wall_caps = Some(kentos_contracts::WallCaps::Open);
            }
            "fill" => f.wall_fill = Some(text(r)?),
            "components" => {
                let (v, at): (Vec<kentos_contracts::WallComponent>, usize) =
                    json_part(r, "duvarın bileşenleri")?;
                if v.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "boş bileşen listesi yazılmaz"));
                }
                if !has.wall_joins && v.iter().any(|c| c.priority.is_some()) {
                    return Err(r.fail_at(
                        Code::UnknownField,
                        at,
                        "bileşenin önceliği şema 76'dan beri yazılır",
                    ));
                }
                f.wall_components = v;
            }
            // Schema 76 (docs/adr/0340 §1): a wall's joins and cleanup.
            "joins" => {
                let (v, at): (kentos_contracts::WallJoins, usize) =
                    json_part(r, "duvarın birleşimleri")?;
                if v.is_auto() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "iki ucu da otomatik birleşim yazılmaz",
                    ));
                }
                f.wall_joins = Some(v);
            }
            "cleanup" => {
                let at = r.position();
                if r.bool()? {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "temizlik açıkken yazılmaz; alan yalnız false iken yazılır",
                    ));
                }
                f.wall_cleanup = Some(false);
            }
            // A wall's, and since schema 131 a block insert's level and phase (docs/adr/0733 §1).
            "arch" => {
                let what = if kind == Kind::Insert {
                    "yerleştirmenin katı ve tadilat durumu"
                } else {
                    "duvarın mimari bilgisi"
                };
                let (v, at): (kentos_contracts::ArchInfo, usize) = json_part(r, what)?;
                if v.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "boş mimari bilgi yazılmaz"));
                }
                f.wall_arch = Some(v);
            }
            "style" => {
                // Schema 9's kinds after the first five (docs/adr/0147).
                let styles = DIMENSION_STYLES.map(|s| (s.name(), s));
                // Schema 62's three after them (docs/adr/0322).
                let known = if has.dimension_marks {
                    &styles[..]
                } else if has.dimensions {
                    &styles[..10]
                } else {
                    &styles[..5]
                };
                f.style = Some(named(r, known)?)
            }
            "pattern" => f.pattern = Some(pattern(r, has)?),
            "assoc" => f.assoc = Some(hatch_assoc(r, has.hatch_boundaries, has.hatch_gaps)?),
            "boundary" if kind == Kind::Hatch => {
                let (b, at): (Vec<kentos_contracts::HatchLoop>, usize) =
                    json_part(r, "taramanın sınırı")?;
                if let Some(words) = kentos_contracts::boundary_problem(&b) {
                    return Err(r.fail_at(Code::BadValue, at, &words));
                }
                f.hatch_boundary = Some(b);
            }
            "refs" => {
                f.refs = Some(dimension_refs(
                    r,
                    has.subref_extras,
                    has.dimension_rules,
                    has.dimension_marks,
                    has.dimension_cross,
                )?)
            }
            // Schema 65 (docs/adr/0324 §1): a text's indents, tabs and columns, as their JSON; checked with its box.
            "layout" => {
                let (l, at): (kentos_contracts::ParagraphLayout, usize) =
                    json_part(r, "yazının düzeni")?;
                if l.engine.is_some() && !has.text_engine {
                    return Err(r.fail_at(Code::UnknownField, at, "bu şemada yazı yerleşim motoru yok"));
                }
                f.layout_at = at;
                f.paragraph.layout = Some(Box::new(l));
            }
            // Schema 67 (docs/adr/0326): a leader's arms, tip, blocks and style; checked together.
            "arms" if kind == Kind::Leader => {
                let (l, at): (Vec<kentos_contracts::LeaderArm>, usize) =
                    json_part(r, "kılavuzun kolları")?;
                if l.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "boş kol listesi yazılmaz"));
                }
                f.leader_at = at;
                f.leader_arms = l;
            }
            "tip" if kind == Kind::Leader => {
                let (t, at): (kentos_contracts::SubRef, usize) = json_part(r, "kılavuzun ucu")?;
                f.leader_at = at;
                f.leader_tip = Some(t);
            }
            "blocks" if kind == Kind::Leader => {
                let (b, at): (Vec<kentos_contracts::EntityId>, usize) =
                    json_part(r, "kılavuzun blokları")?;
                if b.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "boş blok listesi yazılmaz"));
                }
                f.leader_at = at;
                f.leader_blocks = b;
            }
            "leaderStyle" if kind == Kind::Leader => f.leader_style = Some(text(r)?),
            // Schema 68 (docs/adr/0327): a table's styles, formulas, fields and sources, as their JSON; checked with
            // the table.
            "styles" if kind == Kind::Table => {
                let (v, at): (Vec<kentos_contracts::CellStyle>, usize) =
                    json_part(r, "tablonun biçimleri")?;
                if v.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "boş biçim listesi yazılmaz"));
                }
                f.table_at.source = at;
                f.table_styles = v;
            }
            "formulas" if kind == Kind::Table => {
                let (v, at): (Vec<kentos_contracts::CellFormula>, usize) =
                    json_part(r, "tablonun formülleri")?;
                if v.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "boş formül listesi yazılmaz"));
                }
                f.table_at.source = at;
                f.table_formulas = v;
            }
            "fields" if kind == Kind::Table => {
                let (v, at): (Vec<kentos_contracts::CellField>, usize) =
                    json_part(r, "tablonun alanları")?;
                if v.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "boş alan listesi yazılmaz"));
                }
                f.table_at.source = at;
                f.table_fields = v;
            }
            "link" if kind == Kind::Table => {
                let (v, at): (Link, usize) = json_part(r, "tablonun bağlı dosyası")?;
                if v.path.is_none()
                    && v.digest.is_none()
                    && v.copy.is_none()
                    && v.overrides.is_keep()
                {
                    return Err(r.fail_at(Code::BadValue, at, "boş bağ yazılmaz"));
                }
                f.table_at.source = at;
                f.table_link = Some(v);
            }
            "schedule" if kind == Kind::Table => {
                let (v, at): (TableSource, usize) = json_part(r, "tablonun blok çizelgesi")?;
                // Schema 80 (docs/adr/0344 §3): a schedule of the drawing's objects too.
                let schedule = matches!(v, TableSource::Blocks { .. })
                    || (has.tags && matches!(v, TableSource::Schedule { .. }))
                    // Schema 182 (docs/adr/1030 §6): a data extraction table's recipe and what it was written from.
                    || (has.extraction && matches!(v, TableSource::Extraction(_)))
                    // Schema 222 (docs/adr/8030): Emsal ve TAKS tablosu.
                    || (has.zoning && matches!(v, TableSource::Zoning {}))
                    // Schema 227 (docs/adr/8036): Otopark and Sığınak hesabı.
                    || (has.provision_tables && matches!(v, TableSource::Parking {} | TableSource::Shelter {}));
                // Schema 131 (docs/adr/0733): a schedule's fixtures, new fields, type filter, headings and subtotals.
                if !has.fixtures
                    && matches!(&v, TableSource::Schedule { of, columns, filter, sort, .. }
                        if kentos_contracts::schedule_extras(*of, columns, filter.as_ref(), sort))
                {
                    return Err(r.fail_at(
                        Code::UnknownField,
                        at,
                        "çizelgenin donatısı, yeni alanları, tür süzgeci, başlığı ve ara toplamı şema 131'den beri yazılır",
                    ));
                }
                // Schema 222 (docs/adr/8030): Emsal ve TAKS tablosu.
                if !has.zoning && matches!(v, TableSource::Zoning {}) {
                    return Err(r.fail_at(
                        Code::UnknownField,
                        at,
                        "emsal ve TAKS tablosunun kaynağı şema 222'den beri yazılır",
                    ));
                }
                // Schema 227 (docs/adr/8036): Otopark and Sığınak hesabı.
                if !has.provision_tables
                    && matches!(v, TableSource::Parking {} | TableSource::Shelter {})
                {
                    return Err(r.fail_at(
                        Code::UnknownField,
                        at,
                        "otopark ve sığınak hesabı tablolarının kaynağı şema 227'den beri yazılır",
                    ));
                }
                // Schema 182 (docs/adr/1030 §6): a data extraction table.
                if !has.extraction && matches!(v, TableSource::Extraction(_)) {
                    return Err(r.fail_at(
                        Code::UnknownField,
                        at,
                        "veri çıkarımı tablosunun kaynağı şema 182'den beri yazılır",
                    ));
                }
                // Schema 122 (docs/adr/0721 §4): a block schedule's further columns.
                if !has.schedule_columns
                    && matches!(&v, TableSource::Blocks { columns, .. } if !columns.is_empty())
                {
                    return Err(r.fail_at(
                        Code::UnknownField,
                        at,
                        "blok çizelgesinin sütunları (columns) şema 122'den önce yazılmaz",
                    ));
                }
                if !schedule || f.source.is_some() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "çizelge yalnız blok ya da mimari çizelgedir ve kaynakla birlikte yazılmaz",
                    ));
                }
                f.table_at.source = at;
                f.source = Some(v);
            }
            // Schema 66 (docs/adr/0325 §1): a text's fields, as their JSON; checked with its letters.
            "fields" => {
                let (l, at): (Vec<kentos_contracts::TextField>, usize) =
                    json_part(r, "yazının alanları")?;
                f.fields_at = at;
                if l.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "boş alan listesi yazılmaz; alanı olmayan yazıda alan yoktur",
                    ));
                }
                f.paragraph.fields = l;
            }
            // Schema 64 (docs/adr/0323): the scales, as their JSON; the whole object's rule is checked once it is read.
            "annotative" => {
                let (a, _): (kentos_contracts::Annotative, usize) = json_part(r, "ölçekler")?;
                f.annotative = Some(Box::new(a));
            }
            // Schema 63 (docs/adr/0322 §5): the fields set apart from the style, by name, as their JSON.
            "overrides" => {
                let (o, at): (Vec<String>, usize) = json_part(r, "geçersiz kılınanlar")?;
                if o.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "boş geçersiz kılma listesi yazılmaz",
                    ));
                }
                let problem = if kind == Kind::Text {
                    TextFace {
                        overrides: o.clone(),
                        font: Some(kentos_contracts::DrawingFont::Barlow),
                        ..TextFace::default()
                    }
                    .problem()
                } else {
                    DimensionLook {
                        overrides: o.clone(),
                        ..DimensionLook::default()
                    }
                    .problem()
                };
                if let Some((_, words)) = problem {
                    return Err(r.fail_at(Code::BadValue, at, &words));
                }
                f.face.overrides.clone_from(&o);
                f.look.overrides = o;
            }
            // Schema 61 (docs/adr/0321): as their JSON, as the writer writes them.
            "options" => {
                let (o, at): (kentos_contracts::DimensionOptions, usize) =
                    json_part(r, "seçenekler")?;
                if let Some(words) = o.problem() {
                    return Err(r.fail_at(Code::BadValue, at, &words));
                }
                f.look.options = Some(Box::new(o));
            }
            "level" => {
                let (l, at): (kentos_contracts::DimensionLevel, usize) =
                    json_part(r, "kotun katı")?;
                if let Some(words) = l.problem() {
                    return Err(r.fail_at(Code::BadValue, at, &words));
                }
                f.dimension_level = Some((l, at));
            }
            "textShift" => {
                let (s, at): (kentos_contracts::TextShift, usize) =
                    json_part(r, "değerin kayması")?;
                if let Some(words) = s.problem() {
                    return Err(r.fail_at(Code::BadValue, at, &words));
                }
                f.text_shift = Some(s);
            }
            "breaks" => {
                let (b, at): (Vec<kentos_contracts::DimensionBreak>, usize) =
                    json_part(r, "kırmalar")?;
                if b.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "boş kırma listesi yazılmaz"));
                }
                if let Some(words) = kentos_contracts::breaks_problem(&b) {
                    return Err(r.fail_at(Code::BadValue, at, &words));
                }
                f.breaks = b;
            }
            "block" => {
                f.block_at = r.position();
                f.block = Some(BlockId(id16(r)?));
            }
            "scale" => {
                let at = r.position();
                let scale = r.float()?;
                if !kentos_contracts::blocks::scale_ok(scale) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("blok ölçeği {scale}; pozitif olmalı"),
                    ));
                }
                f.scale = Some(scale);
            }
            "yScale" => {
                let at = r.position();
                let y = r.float()?;
                if !kentos_contracts::blocks::scale_ok(y) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("blok y ölçeği {y}; pozitif olmalı"),
                    ));
                }
                f.y_scale = Some((y, at));
            }
            "attributePlaces" => {
                let at = r.position();
                let mut places = BTreeMap::new();
                map(r, |r, tag| {
                    places.insert(tag.to_owned(), point(r)?);
                    Ok(())
                })?;
                // One way to write an insert: no places is no map.
                if places.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "boş öznitelik yerleri yazılmaz"));
                }
                f.attribute_places = Some(places);
            }
            "parameters" => {
                let at = r.position();
                let mut values = BTreeMap::new();
                map(r, |r, name| {
                    values.insert(name.to_owned(), param_value(r)?);
                    Ok(())
                })?;
                if values.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "boş parametre değerleri yazılmaz"));
                }
                f.parameters = Some(values);
            }
            "mirror" => {
                let at = r.position();
                if !r.bool()? {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "aynalama false yazılmaz; aynalı olmayan yerleştirmede alan yoktur",
                    ));
                }
                f.mirror = Some(true);
            }
            "align" => f.align = Some(text_align(r)?),
            "boxWidth" => {
                let at = r.position();
                let w = r.float()?;
                if w <= 0.0 {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("çok satırlı yazının kutu genişliği {w}; sıfırdan büyük olmalı"),
                    ));
                }
                f.paragraph.box_width = Some(w);
            }
            "lineSpacing" => {
                let at = r.position();
                let s = r.float()?;
                if !(MIN_LINE_SPACING..=MAX_LINE_SPACING).contains(&s) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!(
                            "çok satırlı yazının satır aralığı {s}; {MIN_LINE_SPACING} ile {MAX_LINE_SPACING} arasında olmalı"
                        ),
                    ));
                }
                f.paragraph.line_spacing = Some(s);
            }
            "runs" => {
                f.runs_at = r.position();
                f.paragraph.runs = list(r, |r, _| text_run(r, has.paragraph_layout))?;
                // The writer leaves an empty list out: one spelling (§6.6).
                if f.paragraph.runs.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        f.runs_at,
                        "biçim dilimi listesi boş; dilimsiz yazıda alan yazılmaz",
                    ));
                }
            }
            "arrow" if kind == Kind::Dimension => {
                let names: Vec<(&str, DimensionArrow)> =
                    DimensionArrow::ALL.iter().map(|a| (a.name(), *a)).collect();
                f.look.arrow = Some(super::super::named(r, &names)?)
            }
            "arrow" => f.arrow = Some(leader_arrow(r, has)?),
            "arrowSize" if kind == Kind::Leader => {
                let at = r.position();
                let x = r.float()?;
                if !leader_arrow_holds(x) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!(
                            "kılavuzun ok boyu {x}; notun yüksekliğinin {MIN_LEADER_ARROW} ile {MAX_LEADER_ARROW} katı olmalı"
                        ),
                    ));
                }
                f.arrow_size = Some(x);
            }
            "dimLineColor" | "extColor" | "textColor" => {
                let at = r.position();
                let c = text(r)?;
                if !is_hex_colour(&c) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("ölçünün {key} değeri “{c}”; #RRGGBB olmalı"),
                    ));
                }
                let slot = match key {
                    "dimLineColor" => &mut f.look.dim_line_color,
                    "extColor" => &mut f.look.ext_color,
                    _ => &mut f.look.text_color,
                };
                *slot = Some(c);
            }
            "dimLineWeight" | "extWeight" => {
                let at = r.position();
                let w = r.float()?;
                if !line_weight_holds(w) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!(
                            "ölçünün {key} değeri {w}; 0 ile {MAX_LINE_WEIGHT} mm arasında olmalı"
                        ),
                    ));
                }
                if key == "dimLineWeight" {
                    f.look.dim_line_weight = Some(w);
                } else {
                    f.look.ext_weight = Some(w);
                }
            }
            "dimLineType" | "extLineType" => {
                let t = super::super::line_type_named(r)?;
                if key == "dimLineType" {
                    f.look.dim_line_type = Some(t);
                } else {
                    f.look.ext_line_type = Some(t);
                }
            }
            "textStyle" | "dimStyle" => {
                let at = r.position();
                let id = text(r)?;
                if id.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "stil kimliği boş"));
                }
                if matches!(kind, Kind::Text | Kind::Table) {
                    f.face_at = at;
                    f.face.text_style = Some(id);
                } else {
                    f.look.dim_style = Some(id);
                }
            }
            "font" => {
                let faced = matches!(kind, Kind::Text | Kind::Table);
                if faced && f.face_at == 0 {
                    f.face_at = r.position();
                }
                let names: Vec<(&str, DrawingFont)> =
                    DrawingFont::ALL.iter().map(|d| (d.id(), *d)).collect();
                let font = Some(super::super::named(r, &names)?);
                if faced {
                    f.face.font = font;
                } else {
                    f.look.font = font;
                }
            }
            "bold" | "italic" => {
                let at = r.position();
                if f.face_at == 0 {
                    f.face_at = at;
                }
                if !r.bool()? {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("{key} false yazılmaz; alan yoksa yazı öyle değildir"),
                    ));
                }
                if key == "bold" {
                    f.face.bold = true;
                } else {
                    f.face.italic = true;
                }
            }
            "oblique" => {
                let at = r.position();
                if f.face_at == 0 {
                    f.face_at = at;
                }
                let o = r.float()?;
                if !oblique_holds(o) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!(
                            "yazının eğikliği {o}; −{MAX_OBLIQUE} ile {MAX_OBLIQUE} arasında ve sıfırdan farklı olmalı"
                        ),
                    ));
                }
                f.face.oblique = Some(o);
            }
            "arrowSize" | "extOffset" | "extBeyond" | "textGap" => {
                let at = r.position();
                let x = r.float()?;
                let positive = key == "arrowSize";
                if !(x <= MAX_DIMENSION_RATIO && if positive { x > 0.0 } else { x >= 0.0 }) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!(
                            "ölçünün {key} değeri {x}; {} ve en çok {MAX_DIMENSION_RATIO} olmalı",
                            if positive {
                                "sıfırdan büyük"
                            } else {
                                "0 ya da büyük"
                            }
                        ),
                    ));
                }
                let slot = match key {
                    "arrowSize" => &mut f.look.arrow_size,
                    "extOffset" => &mut f.look.ext_offset,
                    "extBeyond" => &mut f.look.ext_beyond,
                    _ => &mut f.look.text_gap,
                };
                *slot = Some(x);
            }
            "textPlace" => {
                f.look.text_place = Some(super::super::named(
                    r,
                    &[("centre", DimensionTextPlace::Centre)],
                )?)
            }
            "decimals" => f.look.decimals = Some(r.uint(u64::from(MAX_DIMENSION_DECIMALS))? as u32),
            "unit" => {
                f.look.unit = Some(super::super::named(
                    r,
                    &[
                        ("mm", DrawingUnit::Mm),
                        ("cm", DrawingUnit::Cm),
                        ("m", DrawingUnit::M),
                    ],
                )?)
            }
            "prefix" | "suffix" => {
                let at = r.position();
                let s = text(r)?;
                let n = s.chars().count();
                if n == 0 || n > MAX_AFFIX || s.chars().any(char::is_control) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!(
                            "ölçünün {key} değeri yazılamaz: boş olmamalı, en çok {MAX_AFFIX} harf, satır sonu ya da denetim karakteri yok"
                        ),
                    ));
                }
                if key == "prefix" {
                    f.look.prefix = Some(s);
                } else {
                    f.look.suffix = Some(s);
                }
            }
            "widthFactor" => f.width_factor = Some(width_factor(r)?),
            "labelOf" => {
                if f.label_scale.is_none() {
                    f.link_at = r.position();
                }
                f.label_of = Some(EntityId(id16(r)?));
            }
            "labelScale" => {
                if f.label_of.is_none() {
                    f.link_at = r.position();
                }
                let at = r.position();
                let n = r.float()?;
                if !label_scale_ok(n) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("bağlı yazının ölçeği 1:{n}; sıfırdan büyük olmalı"),
                    ));
                }
                f.label_scale = Some(n);
            }
            "mask" => {
                let at = r.position();
                if !r.bool()? {
                    let words = if kind == Kind::Dimension {
                        "zemin false yazılmaz; zeminsiz ölçüde alan yoktur"
                    } else {
                        "zemin false yazılmaz; zeminsiz yazıda alan yoktur"
                    };
                    return Err(r.fail_at(Code::BadValue, at, words));
                }
                f.mask = Some(true);
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let entity = build(r, kind, index, &mut f)?;
    // Schema 138 (docs/adr/0771): a turning component, a node decision, a bound slab.
    if !has.building_parts && crate::encode::building_parts(&entity) {
        return Err(r.fail(Code::UnknownField, "dönen bileşen, düğüm kararı ya da bağlı döşeme şema 138 ister"));
    }
    // Schema 151 (docs/adr/0795): a schedule of the program's units, its fields, filter or rows' units.
    if !has.unit_schedules && crate::encode::unit_schedules(&entity) {
        return Err(r.fail(Code::UnknownField, "bölüm çizelgesi, alanları, süzgeci ya da satırların bölümleri şema 151 ister"));
    }
    // Schema 147 (docs/adr/0784): a schedule's share field.
    if !has.program_shares && crate::encode::program_shares(&entity) {
        return Err(r.fail(Code::UnknownField, "çizelgenin pay ya da brüt alanı şema 147 ister"));
    }
    // Schema 148 (docs/adr/0785): a component or a slab bound to a project material.
    if !has.materials && crate::encode::materials(&entity) {
        return Err(r.fail(Code::UnknownField, "malzemeye bağlı bileşen ya da döşeme şema 148 ister"));
    }
    // Schema 140 (docs/adr/0773): a space's group, a schedule's program field.
    if !has.building_program && crate::encode::building_program(&entity) {
        return Err(r.fail(Code::UnknownField, "mahalın bölümü ya da çizelgenin program alanı şema 140 ister"));
    }
    // Schema 115 (docs/adr/0705): a stair's or ramp's railings.
    let railed = match &entity {
        Entity::Stair(s) => s.stair.railings.is_some(),
        Entity::Ramp(m) => m.ramp.railings.is_some(),
        _ => false,
    };
    if railed && !has.railings {
        return Err(r.fail(Code::BadValue, "korkuluk (railings) şema 115 ister"));
    }
    // Schema 126 (docs/adr/0725): a gapped grid, an anchored or an L column.
    let formed = match &entity {
        Entity::Grid(g) => g.grid.ends.is_some(),
        Entity::Column(c) => {
            c.column.anchor != kentos_contracts::ColumnAnchor::Center
                || c.column.legs.is_some()
                || c.column.shape == kentos_contracts::ColumnShape::LShape
        }
        _ => false,
    };
    if formed && !has.grid_column_forms {
        return Err(r.fail(
            Code::BadValue,
            "kesik aks (ends), kolonun hizası (anchor) ve L kolon (lShape, legs) şema 126 ister",
        ));
    }
    // Schema 125 (docs/adr/0724): a stair bound to its level's height, or with a centre post.
    if matches!(&entity, Entity::Stair(s) if s.stair.height_from_level || s.stair.post.is_some())
        && !has.stair_levels
    {
        return Err(r.fail(
            Code::BadValue,
            "merdivenin kata bağlı yüksekliği (heightFromLevel) ve merkez direği (post) şema 125 ister",
        ));
    }
    // Schema 124 (docs/adr/0723 §2): a ramp bound to its level.
    if matches!(&entity, Entity::Ramp(m) if m.ramp.base_offset.is_some()) && !has.ramp_levels {
        return Err(r.fail(
            Code::BadValue,
            "rampanın kata göre başlangıç kotu (baseOffset) şema 124 ister",
        ));
    }
    // Schema 117 (docs/adr/0709): a curved stair's arc.
    if matches!(&entity, Entity::Stair(s) if s.stair.bulges.is_some()) && !has.stair_arcs {
        return Err(r.fail(
            Code::BadValue,
            "döner merdivenin bükümü (bulges) şema 117 ister",
        ));
    }
    // Schema 230 (docs/adr/8039): a door's or window's own phase.
    if matches!(&entity, Entity::Opening(o) if o.opening.phase.is_some()) && !has.opening_phases {
        return Err(r.fail(
            Code::UnknownField,
            "kapı ve pencerenin kendi tadilat durumu (phase) şema 230 ister",
        ));
    }
    // Schema 238 (docs/adr/8044 §6): a roof's chimneys, vents and windows.
    if kentos_contracts::needs_roof_parts(&entity) && !has.roof_parts {
        return Err(r.fail(
            Code::UnknownField,
            "çatının bacaları ve pencereleri (parts) şema 238 ister",
        ));
    }
    // Schema 239 (docs/adr/8044 §8): a roof's gutter and snow guard.
    if kentos_contracts::needs_roof_edges(&entity) && !has.roof_edges {
        return Err(r.fail(
            Code::UnknownField,
            "çatının oluğu ve kar siperliği (gutter, snowGuard) şema 239 ister",
        ));
    }
    // Schema 240 (docs/adr/8044 §8): a roof's downpipes.
    if kentos_contracts::needs_roof_downpipes(&entity) && !has.roof_downpipes {
        return Err(r.fail(
            Code::UnknownField,
            "çatının iniş boruları (downpipe) şema 240 ister",
        ));
    }
    // Schema 241 (docs/adr/8045 §1): a balanced stair's winders.
    if kentos_contracts::needs_stair_winders(&entity) && !has.stair_winders {
        return Err(r.fail(
            Code::UnknownField,
            "merdivenin balansmanı (winders) şema 241 ister",
        ));
    }
    // Schema 236 (docs/adr/8041 §3): a beam's role and base.
    if kentos_contracts::needs_beam_roles(&entity) && !has.beam_roles {
        return Err(r.fail(
            Code::UnknownField,
            "kirişin türü (role) ve kattan yüksekliği (baseOffset) şema 236 ister",
        ));
    }
    // Schema 235 (docs/adr/8041 §2): a column's material.
    if matches!(&entity, Entity::Column(c) if c.column.material.is_some()) && !has.column_material {
        return Err(r.fail(Code::UnknownField, "kolonun malzemesi (material) şema 235 ister"));
    }
    // Schema 233 (docs/adr/8040 §3): a room's floor offset and asma tavan.
    if kentos_contracts::needs_room_heights(&entity) && !has.room_heights {
        return Err(r.fail(
            Code::UnknownField,
            "mahalin döşeme kot farkı (floorOffset) ve asma tavanı (suspendedCeiling) şema 233 ister",
        ));
    }
    // Schema 232 (docs/adr/8040 §2): a slab's layers besides its structure.
    if matches!(&entity, Entity::Slab(s) if !s.slab.layers.is_empty()) && !has.slab_layers {
        return Err(r.fail(
            Code::UnknownField,
            "döşemenin katmanları (layers) şema 232 ister",
        ));
    }
    // Schema 228 (docs/adr/8038): the room list's finishes and a spaces schedule's room list columns.
    if kentos_contracts::needs_room_list(&entity) && !has.room_list {
        return Err(r.fail(
            Code::UnknownField,
            "mahalin denizlik, merdiven ve korkuluk kaplamaları ve çizelgenin mahal listesi sütunları (sillFinish, \
             stairFinish, railingFinish, doors, windows) şema 228 ister",
        ));
    }
    // Schema 223 (docs/adr/8031): a units schedule's kat irtifakı fields; before it none was a field.
    if kentos_contracts::needs_land_shares(&entity) && !has.land_shares {
        return Err(r.fail(
            Code::UnknownField,
            "çizelgenin kat irtifakı sütunları (nature, annexNet, annexGross, annexes, totalGross, landShare) şema 223 ister",
        ));
    }
    // Schema 221 (docs/adr/8029): a units schedule's regulation areas; before it none was a field.
    if kentos_contracts::needs_paiy_areas(&entity) && !has.paiy_areas {
        return Err(r.fail(
            Code::UnknownField,
            "çizelgenin yönetmelik alanı sütunları (paiyNet, paiyGross, paiyShare, generalGross) şema 221 ister",
        ));
    }
    // Schema 220 (docs/adr/8028): a tag naming an opening's joinery code or size; before it neither was a place.
    if kentos_contracts::needs_opening_codes(&entity) && !has.opening_codes {
        return Err(r.fail(
            Code::UnknownField,
            "etiketin doğrama kodu ya da ölçü yeri ({code}, {size}) şema 220 ister",
        ));
    }
    // Schema 219 (docs/adr/8027): a tag framed by an ellipse; before it a tag's frame was one of five.
    if kentos_contracts::needs_ellipse_tags(&entity) && !has.ellipse_tags {
        return Err(r.fail(
            Code::UnknownField,
            "etiketin elips çerçevesi (ellipse) şema 219 ister",
        ));
    }
    // Schema 153 (docs/adr/0807): an XOX slide; before it three sliding leaves did not part to both sides.
    if matches!(&entity, Entity::Opening(o) if kentos_contracts::is_xox(&o.opening))
        && !has.slide_more
    {
        return Err(r.fail(
            Code::BadValue,
            "üç kanatlı sürmenin iki yana açılması (XOX) şema 153 ister",
        ));
    }
    // Schema 150 (docs/adr/0792): a three- or four-leaf sliding door or window, its tracks, a schedule of them.
    if kentos_contracts::needs_multi_slide(&entity) && !has.multi_slide {
        return Err(r.fail(
            Code::UnknownField,
            "üç ve dört kanatlı sürme, ray sayısı (tracks) ve çizelgenin ray ve kanat düzeni sütunları şema 150 ister",
        ));
    }
    // Schema 144 (docs/adr/0777): a revolving, pocket or folding door, a sliding door's track, a schedule of them.
    if kentos_contracts::needs_special_doors(&entity) && !has.special_doors {
        return Err(r.fail(
            Code::UnknownField,
            "döner, cebe giren ve katlanır kapı, kayar kapının rayı ve bindirmesi ve çizelgenin açılış sütunları şema 144 ister",
        ));
    }
    // Schema 139 (docs/adr/0772): an opening following a type or built, a schedule of an opening's widths.
    if kentos_contracts::needs_opening_types(&entity) && !has.opening_types {
        return Err(r.fail(
            Code::UnknownField,
            "açıklığın tipi (typeVersion, overrides), yapısı (build) ve çizelgenin genişlik sütunları şema 139 ister",
        ));
    }
    // Schema 119 (docs/adr/0719): a niche, a flue, a depth, an inset, an opening too deep.
    if kentos_contracts::needs_recesses(&entity) && !has.recesses {
        return Err(r.fail(
            Code::BadValue,
            "niş, baca, derinlik (depth) ve yüzden uzaklık (inset) şema 119 ister",
        ));
    }
    // A closed and a wide polyline's rules (docs/adr/0258).
    if let Entity::Polyline(p) = &entity
        && let Some(why) = p.extras_problem(false)
    {
        return Err(r.fail(Code::BadValue, &why));
    }
    // An annotation's scales (docs/adr/0323).
    if let Some(why) = kentos_contracts::annotative_problem(&entity) {
        return Err(r.fail(Code::BadValue, &why));
    }
    let uid = if has.uids {
        Some(required(r, f.uid, "uid")?)
    } else {
        None
    };
    r.pop();
    r.leave();
    Ok((entity, uid, f.block_at))
}

/// A polyline edge's start and end width (docs/adr/0258 §2): two numbers.
pub(super) fn width(r: &mut Reader<'_>) -> Result<[f64; 2], KcadError> {
    let at = r.position();
    let n = r.array()?;
    if n != 2 {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            &format!("genişlik 2 sayı olmalı (başlangıç, bitiş), {n} var"),
        ));
    }
    let start = r.float()?;
    let end = r.float()?;
    r.leave();
    Ok([start, end])
}
