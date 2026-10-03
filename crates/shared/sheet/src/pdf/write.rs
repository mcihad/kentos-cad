//! The document: a page a sheet with its content and resources, the faces,
//! pictures and maps' layers shared by every page, the geographic
//! viewports, the catalog, the information dictionary and the file's id.
//! Objects are numbered in the order they are made, from the inputs only,
//! so the same inputs give the same file.

use std::collections::BTreeMap;

use pdf_writer::types::PageMode;
use pdf_writer::{Chunk, Date, Filter, Finish, Name, Pdf, Rect, Ref, Str, TextStr};
use sha2::{Digest, Sha256};

use super::content::{PT, Page};
use super::fonts::{Fonts, Usage};
use super::images::{Picture, PictureKind, deflate, picture};
use super::{MapContent, PdfFace, PdfInputs, PdfOptions, geo, maps};
use crate::display::{self, DisplayList, Prim, RenderMode};
use crate::error::{Result, SheetError};
use crate::model::{SheetBook, SheetId, VarValue};

/// The document's objects as they are written, and the next object number.
pub(super) struct Doc {
    pub chunk: Chunk,
    next: i32,
}

impl Doc {
    fn new() -> Doc {
        Doc {
            chunk: Chunk::new(),
            next: 1,
        }
    }

    pub fn alloc(&mut self) -> Ref {
        let r = Ref::new(self.next);
        self.next += 1;
        r
    }
}

/// Each chosen sheet's display list, for export (printable items only).
pub(super) fn lists(
    book: &SheetBook,
    inputs: &PdfInputs,
    sheets: &[SheetId],
) -> Result<Vec<DisplayList>> {
    let mut render = inputs.render.clone();
    render.mode = RenderMode::Export;
    sheets
        .iter()
        .map(|id| display::build(book, id, &render).map(|(l, _)| l))
        .collect()
}

fn content_of<'a>(inputs: &'a PdfInputs, item: &str) -> Option<&'a MapContent> {
    inputs
        .maps
        .iter()
        .find(|m| m.item == item)
        .map(|m| &m.content)
}

/// The text every page writes, by face: the sheets', the maps' and the empty maps' “harita”.
fn usage_of(lists: &[DisplayList], inputs: &PdfInputs) -> Usage {
    let mut u = Usage::default();
    for l in lists {
        for p in &l.prims {
            match p {
                Prim::Text(t) => u.add(&t.font, t.weight, t.italic, &t.text),
                Prim::Map(m) => match content_of(inputs, &m.item) {
                    Some(MapContent::Vector(v)) if m.view.center.is_some() => {
                        for layer in &v.layers {
                            for t in &layer.texts {
                                u.add(&t.font, t.weight, t.italic, &t.text);
                            }
                        }
                    }
                    Some(MapContent::Raster(_)) => {}
                    _ => {
                        let (text, font, weight) = maps::PLACEHOLDER;
                        u.add(font, weight, false, text);
                    }
                },
                _ => {}
            }
        }
    }
    u
}

pub(super) fn faces_of(
    book: &SheetBook,
    inputs: &PdfInputs,
    sheets: &[SheetId],
) -> Result<Vec<PdfFace>> {
    Ok(usage_of(&lists(book, inputs, sheets)?, inputs).faces())
}

/// A picture as an image object (and its soft mask): `Im…` for the pages.
fn write_picture(doc: &mut Doc, p: &Picture) -> Ref {
    let id = doc.alloc();
    let mask = p.mask.as_ref().map(|m| {
        let r = doc.alloc();
        let mut x = doc.chunk.image_xobject(r, m);
        x.width(p.width as i32)
            .height(p.height as i32)
            .color_space_name(Name(b"DeviceGray"))
            .bits_per_component(8);
        x.filter(Filter::FlateDecode);
        x.finish();
        r
    });
    let mut x = doc.chunk.image_xobject(id, &p.data);
    x.width(p.width as i32)
        .height(p.height as i32)
        .bits_per_component(8);
    match p.kind {
        PictureKind::Gray => {
            x.color_space_name(Name(b"DeviceGray"));
            x.filter(Filter::FlateDecode);
        }
        PictureKind::Rgb => {
            x.color_space_name(Name(b"DeviceRGB"));
            x.filter(Filter::FlateDecode);
        }
        PictureKind::Jpeg {
            components,
            inverted,
        } => {
            x.color_space_name(Name(match components {
                1 => b"DeviceGray".as_slice(),
                4 => b"DeviceCMYK".as_slice(),
                _ => b"DeviceRGB".as_slice(),
            }));
            x.filter(Filter::DctDecode);
            if inverted {
                x.decode([1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0]);
            }
        }
    }
    if let Some(m) = mask {
        x.s_mask(m);
    }
    x.finish();
    id
}

/// The document's title, author and subject from its variables and the project.
struct Meta {
    title: String,
    author: String,
    subject: String,
    date: Option<Date>,
}

fn meta_of(book: &SheetBook, inputs: &PdfInputs, sheets: &[SheetId]) -> Meta {
    let first = sheets.first().and_then(|id| book.sheet(id));
    let named = |vars: &[crate::model::Variable]| {
        vars.iter()
            .find(|v| v.name == "baslik")
            .and_then(|v| match &v.value {
                VarValue::Text(t) if !t.trim().is_empty() => Some(t.clone()),
                _ => None,
            })
    };
    let title = first
        .and_then(|s| named(&s.variables))
        .or_else(|| named(&book.variables))
        .or_else(|| first.map(|s| s.name.clone()))
        .unwrap_or_default();
    let p = &inputs.render.project;
    Meta {
        title,
        author: p.user.clone(),
        subject: p.name.clone(),
        date: date_of(&p.date),
    }
}

/// An ISO date (`2026-10-03`, a time after it ignored) as the PDF's; none for anything else.
fn date_of(iso: &str) -> Option<Date> {
    let d = iso.get(..10)?;
    let (y, m, day) = (
        d.get(..4)?.parse::<u16>().ok()?,
        d.get(5..7)?.parse::<u8>().ok()?,
        d.get(8..10)?.parse::<u8>().ok()?,
    );
    (d.as_bytes().get(4) == Some(&b'-') && (1..=12).contains(&m) && (1..=31).contains(&day))
        .then(|| Date::new(y).month(m).day(day))
}

pub(super) fn document(
    book: &SheetBook,
    inputs: &PdfInputs,
    options: &PdfOptions,
    sheets: &[SheetId],
) -> Result<Vec<u8>> {
    let lists = lists(book, inputs, sheets)?;
    let usage = usage_of(&lists, inputs);
    let mut doc = Doc::new();
    let catalog = doc.alloc();
    let tree = doc.alloc();
    let info = doc.alloc();
    let fonts = Fonts::embed(&mut doc, &usage, &inputs.fonts)?;

    // The pictures, each once, in the order the pages first show them.
    let mut images: BTreeMap<String, Option<(String, Ref)>> = BTreeMap::new();
    let mut rasters: BTreeMap<String, (String, Ref)> = BTreeMap::new();
    let mut layers: BTreeMap<String, (String, Ref)> = BTreeMap::new();
    let mut layer_order: Vec<(Ref, String)> = Vec::new();
    for l in &lists {
        for p in &l.prims {
            match p {
                Prim::Image(i) if !images.contains_key(&i.asset) => {
                    // An SVG picture as the PNG the host drew of it (the core draws no SVG).
                    let svg = super::is_svg(book, &i.asset);
                    let pic = inputs
                        .assets
                        .iter()
                        .find(|a| a.sha256 == i.asset)
                        .and_then(|a| {
                            if svg {
                                a.raster.as_deref().and_then(picture)
                            } else {
                                picture(&a.data)
                            }
                        });
                    let named = pic.map(|pic| {
                        let r = write_picture(&mut doc, &pic);
                        (format!("Im{}", images.len() + rasters.len() + 1), r)
                    });
                    images.insert(i.asset.clone(), named);
                }
                Prim::Map(m) => match content_of(inputs, &m.item) {
                    Some(MapContent::Raster(r)) if !rasters.contains_key(&m.item) => {
                        let pic = picture(&r.png).ok_or_else(|| {
                            SheetError::at(
                                "pdf_map_bad",
                                m.item.clone(),
                                "Haritanın resmi PNG değil ya da bozuk: haritayı yeniden çizdirip aktarın.",
                            )
                        })?;
                        let id = write_picture(&mut doc, &pic);
                        rasters.insert(
                            m.item.clone(),
                            (format!("Im{}", images.len() + rasters.len() + 1), id),
                        );
                    }
                    Some(MapContent::Vector(v)) if options.layers => {
                        for layer in &v.layers {
                            if !layers.contains_key(&layer.id) {
                                let r = doc.alloc();
                                layers.insert(
                                    layer.id.clone(),
                                    (format!("oc{}", layers.len() + 1), r),
                                );
                                layer_order.push((r, layer.name.clone()));
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
    for (r, name) in &layer_order {
        let mut d = doc.chunk.indirect(*r).dict();
        d.pair(Name(b"Type"), Name(b"OCG"));
        d.pair(Name(b"Name"), TextStr(name));
    }

    let tm = inputs.render.crs.as_ref().and_then(|c| c.tm.as_ref());
    let mut gstates: BTreeMap<String, Ref> = BTreeMap::new();
    let mut page_refs = Vec::new();
    let mut gpts: Vec<String> = Vec::new();
    for l in &lists {
        let page_ref = doc.alloc();
        let content_ref = doc.alloc();
        let mut page = Page::new(l.size.height, &fonts);
        let full = crate::units::RectUm::new(0, 0, l.size.width, l.size.height);
        page.rect(&full, 0, 0);
        page.paint(Some(&l.paper), None, false);
        let mut viewports = Vec::new();
        for p in &l.prims {
            match p {
                Prim::Rect(r) => {
                    page.rect(&r.rect, r.rotation, r.radius);
                    page.paint(r.fill.as_deref(), r.stroke.as_ref(), false);
                }
                Prim::Path(pp) => {
                    page.path(&pp.segments);
                    page.paint(pp.fill.as_deref(), pp.stroke.as_ref(), pp.even_odd);
                }
                Prim::Text(t) => page.text(t),
                Prim::Image(i) => {
                    let named = images.get(&i.asset).and_then(|n| n.as_ref());
                    page.image(i, named.map(|(n, r)| (n.as_str(), *r)));
                }
                Prim::Map(m) => {
                    match content_of(inputs, &m.item) {
                        Some(MapContent::Vector(v)) => {
                            maps::vector(&mut page, m, v, options.layers.then_some(&layers));
                        }
                        Some(MapContent::Raster(_)) => match rasters.get(&m.item) {
                            Some((n, r)) => maps::raster(&mut page, m, (n.as_str(), *r)),
                            None => maps::placeholder(&mut page, m),
                        },
                        None => maps::placeholder(&mut page, m),
                    }
                    if options.geo && inputs.crs.is_some() {
                        let host = inputs
                            .maps
                            .iter()
                            .find(|x| x.item == m.item)
                            .and_then(|x| x.corners.as_ref());
                        if let Some(v) = geo::viewport(l.size.height, m, tm, host) {
                            viewports.push(v);
                        }
                    }
                }
                Prim::PushClip(c) => page.clip(c),
                Prim::PopClip(_) | Prim::PopGroup(_) => page.restore(),
                Prim::PushGroup(g) => page.group(g.opacity),
            }
        }
        let used_fonts = std::mem::take(&mut page.used_fonts);
        let xobjects = std::mem::take(&mut page.xobjects);
        let page_states = std::mem::take(&mut page.gstates);
        let properties = std::mem::take(&mut page.properties);
        let stream = deflate(page.c.finish().as_slice());
        doc.chunk
            .stream(content_ref, &stream)
            .filter(Filter::FlateDecode);
        let mut state_refs: Vec<(String, Ref)> = Vec::new();
        for (name, (fill, stroke)) in &page_states {
            let r = match gstates.get(name) {
                Some(r) => *r,
                None => {
                    let r = doc.alloc();
                    doc.chunk
                        .ext_graphics(r)
                        .non_stroking_alpha(*fill)
                        .stroking_alpha(*stroke);
                    gstates.insert(name.clone(), r);
                    r
                }
            };
            state_refs.push((name.clone(), r));
        }
        let (w, h) = (
            (f64::from(l.size.width) * PT) as f32,
            (f64::from(l.size.height) * PT) as f32,
        );
        let mut pg = doc.chunk.page(page_ref);
        pg.media_box(Rect::new(0.0, 0.0, w, h))
            .parent(tree)
            .contents(content_ref);
        {
            let mut res = pg.resources();
            if !used_fonts.is_empty() {
                let mut f = res.fonts();
                for (name, r) in &used_fonts {
                    f.pair(Name(name.as_bytes()), *r);
                }
            }
            if !xobjects.is_empty() {
                let mut x = res.x_objects();
                for (name, r) in &xobjects {
                    x.pair(Name(name.as_bytes()), *r);
                }
            }
            if !state_refs.is_empty() {
                let mut g = res.ext_g_states();
                for (name, r) in &state_refs {
                    g.pair(Name(name.as_bytes()), *r);
                }
            }
            if !properties.is_empty() {
                let mut pr = res.insert(Name(b"Properties")).dict();
                for (name, r) in &properties {
                    pr.pair(Name(name.as_bytes()), *r);
                }
            }
        }
        if let Some(crs) = inputs.crs.as_ref().filter(|_| !viewports.is_empty()) {
            let mut vps = pg.insert(Name(b"VP")).array();
            for v in &viewports {
                let n = gpts.len();
                gpts.push(geo::gpts_text(v));
                let placeholder = geo::placeholder(n);
                let mut d = vps.push().dict();
                d.pair(Name(b"Type"), Name(b"Viewport"));
                d.pair(
                    Name(b"BBox"),
                    Rect::new(v.bbox[0], v.bbox[1], v.bbox[2], v.bbox[3]),
                );
                d.pair(Name(b"Name"), TextStr("Harita"));
                let mut m = d.insert(Name(b"Measure")).dict();
                m.pair(Name(b"Type"), Name(b"Measure"));
                m.pair(Name(b"Subtype"), Name(b"GEO"));
                m.insert(Name(b"Bounds"))
                    .array()
                    .items(v.lpts.iter().flat_map(|p| [p[0], p[1]]));
                m.pair(Name(b"GPTS"), Str(placeholder.as_bytes()));
                m.insert(Name(b"LPTS"))
                    .array()
                    .items(v.lpts.iter().flat_map(|p| [p[0], p[1]]));
                {
                    let mut g = m.insert(Name(b"GCS")).dict();
                    let geographic = crs.wkt.trim_start().starts_with("GEOGCS");
                    g.pair(
                        Name(b"Type"),
                        Name(if geographic {
                            b"GEOGCS".as_slice()
                        } else {
                            b"PROJCS".as_slice()
                        }),
                    );
                    if let Some(code) = crs.epsg {
                        g.pair(Name(b"EPSG"), code as i32);
                    }
                    g.pair(Name(b"WKT"), Str(crs.wkt.as_bytes()));
                }
                m.insert(Name(b"PDU"))
                    .array()
                    .items([Name(b"M"), Name(b"SQM"), Name(b"DEG")]);
            }
        }
        pg.finish();
        page_refs.push(page_ref);
    }
    doc.chunk
        .pages(tree)
        .kids(page_refs.iter().copied())
        .count(page_refs.len() as i32);

    // The file's id: the digest of its objects (and the coordinates set into them).
    let mut digest = Sha256::new();
    digest.update(doc.chunk.as_bytes());
    for g in &gpts {
        digest.update(g.as_bytes());
    }
    let id = digest.finalize()[..16].to_vec();

    let meta = meta_of(book, inputs, sheets);
    let mut pdf = Pdf::new();
    pdf.set_version(1, 7);
    pdf.set_file_id((id.clone(), id));
    pdf.extend(&doc.chunk);
    {
        let mut cat = pdf.catalog(catalog);
        cat.pages(tree);
        if !layer_order.is_empty() {
            cat.page_mode(PageMode::UseOC);
            let mut oc = cat.insert(Name(b"OCProperties")).dict();
            oc.insert(Name(b"OCGs"))
                .array()
                .items(layer_order.iter().map(|(r, _)| *r));
            let mut d = oc.insert(Name(b"D")).dict();
            d.pair(Name(b"Name"), TextStr("Katmanlar"));
            d.insert(Name(b"Order"))
                .array()
                .items(layer_order.iter().map(|(r, _)| *r));
            d.insert(Name(b"ON"))
                .array()
                .items(layer_order.iter().map(|(r, _)| *r));
        }
    }
    {
        let mut di = pdf.document_info(info);
        if !meta.title.is_empty() {
            di.title(TextStr(&meta.title));
        }
        if !meta.author.is_empty() {
            di.author(TextStr(&meta.author));
        }
        if !meta.subject.is_empty() {
            di.subject(TextStr(&meta.subject));
        }
        di.creator(TextStr("KentOS pafta düzeni"));
        di.producer(TextStr("KentOS"));
        if let Some(d) = meta.date {
            di.creation_date(d);
            di.modified_date(d);
        }
    }
    Ok(geo::patch(pdf.finish(), &gpts))
}
